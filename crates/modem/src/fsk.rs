//! Frequency Shift Keying (FSK) implementation

use crate::{ModemError, Result};
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality};
use openham_core::buffer::Complex;
use std::f64::consts::PI;

/// FSK modulator
pub struct FskModulator {
    config: ModulationConfig,
    phase: f64,
    freq_mark: f64,   // Frequency for '1' bit
    freq_space: f64,  // Frequency for '0' bit
}

impl FskModulator {
    /// Create a new FSK modulator
    pub fn new(config: ModulationConfig) -> Result<Self> {
        let shift = 500.0; // 500 Hz frequency shift
        let freq_mark = config.carrier_frequency + shift / 2.0;
        let freq_space = config.carrier_frequency - shift / 2.0;
        
        Ok(Self { 
            config,
            phase: 0.0,
            freq_mark,
            freq_space,
        })
    }
}

impl Modulator for FskModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        let samples_per_symbol = self.samples_per_symbol();
        
        for &byte in bits {
            for bit_idx in (0..8).rev() {
                let bit = (byte >> bit_idx) & 1;
                let freq = if bit == 1 { self.freq_mark } else { self.freq_space };
                
                for _ in 0..samples_per_symbol {
                    let sample = Complex {
                        real: (2.0 * PI * freq * self.phase / self.config.sample_rate).cos(),
                        imag: (2.0 * PI * freq * self.phase / self.config.sample_rate).sin(),
                    };
                    output.push(sample);
                    self.phase += 1.0;
                    
                    // Prevent phase overflow
                    if self.phase >= self.config.sample_rate {
                        self.phase -= self.config.sample_rate;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        self.config.samples_per_symbol() as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        self.config.symbol_rate
    }
    
    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// FSK demodulator
pub struct FskDemodulator {
    config: ModulationConfig,
    freq_mark: f64,   // Frequency for '1' bit
    freq_space: f64,  // Frequency for '0' bit
    buffer: Vec<Complex>,
    bit_buffer: u8,
    bit_count: usize,
}

impl FskDemodulator {
    /// Create a new FSK demodulator
    pub fn new(config: ModulationConfig) -> Result<Self> {
        let shift = 500.0; // 500 Hz frequency shift
        let freq_mark = config.carrier_frequency + shift / 2.0;
        let freq_space = config.carrier_frequency - shift / 2.0;
        
        Ok(Self { 
            config,
            freq_mark,
            freq_space,
            buffer: Vec::new(),
            bit_buffer: 0,
            bit_count: 0,
        })
    }
}

impl Demodulator for FskDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        self.buffer.extend_from_slice(samples);
        output.clear();

        let samples_per_symbol = self.config.samples_per_symbol() as usize;
        if samples_per_symbol == 0 || self.buffer.len() < samples_per_symbol { return Ok(()); }

        // Helper: demodulate from a given offset building bytes
    let sync: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E];
    let sync_inv: [u8; 8] = [0xAA, 0xAA, 0xAA, 0xAA, 0x55, 0x55, 0x81, 0x81];
        let mut candidate_streams: Vec<Vec<u8>> = Vec::new();
        let mut best_sync: Option<(usize, usize)> = None; // (offset, pos)

        for offset in 0..samples_per_symbol {
            let mut bits_acc: Vec<u8> = Vec::new();
            let mut bytes_acc: Vec<u8> = Vec::new();

            let mut idx = offset;
            while idx + samples_per_symbol <= self.buffer.len() {
                let symbol_samples = &self.buffer[idx..idx + samples_per_symbol];
                // Noncoherent energy detection at mark/space
                let mut mi = 0.0; let mut mq = 0.0;
                let mut si = 0.0; let mut sq = 0.0;
                for (k, sample) in symbol_samples.iter().enumerate() {
                    let t = k as f64 / self.config.sample_rate;
                    let cr_m = (2.0 * PI * self.freq_mark * t).cos();
                    let sr_m = (2.0 * PI * self.freq_mark * t).sin();
                    mi += sample.real * cr_m;
                    mq += sample.real * (-sr_m);
                    let cr_s = (2.0 * PI * self.freq_space * t).cos();
                    let sr_s = (2.0 * PI * self.freq_space * t).sin();
                    si += sample.real * cr_s;
                    sq += sample.real * (-sr_s);
                }
                let e_mark = mi * mi + mq * mq;
                let e_space = si * si + sq * sq;
                let bit = if e_mark > e_space { 1u8 } else { 0u8 };
                bits_acc.push(bit);
                if bits_acc.len() == 8 {
                    let mut byte = 0u8;
                    for (j, &b) in bits_acc.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                    bytes_acc.push(byte);
                    bits_acc.clear();
                }
                idx += samples_per_symbol;
            }
            if !bits_acc.is_empty() {
                let mut byte = 0u8;
                for (j, &b) in bits_acc.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                bytes_acc.push(byte);
            }

            // Search for sync
            let mut found: Option<usize> = None;
            if bytes_acc.len() >= sync.len() {
                for pos in 0..=bytes_acc.len() - sync.len() {
                    if &bytes_acc[pos..pos + sync.len()] == sync { found = Some(pos); break; }
                    if &bytes_acc[pos..pos + sync_inv.len()] == sync_inv { found = Some(pos); break; }
                }
            }
            if let Some(pos) = found {
                match best_sync { None => best_sync = Some((offset, pos)), Some((_, bp)) if pos < bp => best_sync = Some((offset, pos)), _ => {} }
            }
            candidate_streams.push(bytes_acc);
        }

        if let Some((best_o, _)) = best_sync {
            output.extend_from_slice(&candidate_streams[best_o]);
            return Ok(());
        }

        // Fallback: choose the longest stream
        if let Some((best_o, _)) = candidate_streams.iter().enumerate().max_by_key(|(_, v)| v.len()) {
            output.extend_from_slice(&candidate_streams[best_o]);
        }
        Ok(())
    }
    
    fn is_synchronized(&self) -> bool {
        true // Simple implementation always claims sync
    }
    
    fn signal_quality(&self) -> SignalQuality {
        SignalQuality::default()
    }
    
    fn reset(&mut self) {
        self.buffer.clear();
        self.bit_buffer = 0;
        self.bit_count = 0;
    }
}

// (no additional helpers)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fsk_creation() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let _modulator = FskModulator::new(config.clone()).unwrap();
        let _demodulator = FskDemodulator::new(config).unwrap();
    }
}
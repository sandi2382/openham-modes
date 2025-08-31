//! Binary Phase Shift Keying (BPSK) implementation

use crate::{ModemError, Result};
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality, PulseShaper};
use openham_core::buffer::Complex;
use std::f64::consts::PI;

/// BPSK modulator
pub struct BpskModulator {
    config: ModulationConfig,
    pulse_shaper: PulseShaper,
    phase: f64,
    sample_counter: f64,
}

impl BpskModulator {
    /// Create a new BPSK modulator
    pub fn new(config: ModulationConfig) -> Result<Self> {
        let samples_per_symbol = config.samples_per_symbol();
        let pulse_shaper = PulseShaper::root_raised_cosine(
            samples_per_symbol,
            config.rolloff_factor,
            config.filter_length,
        )?;
        
        Ok(Self {
            config,
            pulse_shaper,
            phase: 0.0,
            sample_counter: 0.0,
        })
    }
    
    /// Generate carrier wave
    fn generate_carrier(&mut self, symbol: f64) -> Complex {
        let omega = 2.0 * PI * self.config.carrier_frequency / self.config.sample_rate;
        let phase = omega * self.sample_counter;
        
        self.sample_counter += 1.0;
        
        // BPSK: multiply carrier by symbol (+1 or -1)
        Complex::new(symbol * phase.cos(), symbol * phase.sin())
    }
}

impl Modulator for BpskModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let samples_per_symbol = self.config.samples_per_symbol() as usize;
        
        for &byte in bits {
            for bit_pos in (0..8).rev() {
                let bit = (byte >> bit_pos) & 1;
                let symbol = if bit == 0 { -1.0 } else { 1.0 };
                
                // Generate samples for this symbol
                for _ in 0..samples_per_symbol {
                    let modulated = self.generate_carrier(symbol);
                    output.push(modulated);
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
        self.pulse_shaper.reset();
        self.phase = 0.0;
        self.sample_counter = 0.0;
    }
}

/// BPSK demodulator
pub struct BpskDemodulator {
    config: ModulationConfig,
    pulse_shaper: PulseShaper,
    phase: f64,
    sample_counter: f64,
    is_sync: bool,
    signal_quality: SignalQuality,
}

impl BpskDemodulator {
    /// Create a new BPSK demodulator
    pub fn new(config: ModulationConfig) -> Result<Self> {
        let samples_per_symbol = config.samples_per_symbol();
        let pulse_shaper = PulseShaper::root_raised_cosine(
            samples_per_symbol,
            config.rolloff_factor,
            config.filter_length,
        )?;
        
        Ok(Self {
            config,
            pulse_shaper,
            phase: 0.0,
            sample_counter: 0.0,
            is_sync: false,
            signal_quality: SignalQuality::default(),
        })
    }
    
    /// Demodulate to baseband
    fn demodulate_to_baseband(&mut self, sample: Complex) -> Complex {
        let omega = 2.0 * PI * self.config.carrier_frequency / self.config.sample_rate;
        let phase = omega * self.sample_counter;
        
        self.sample_counter += 1.0;
        
        // Local oscillator for demodulation (note: conjugate for downmixing)
        let lo = Complex::new(phase.cos(), -phase.sin());
        
        // Mix down to baseband
        Complex::new(
            sample.real * lo.real - sample.imag * lo.imag,
            sample.real * lo.imag + sample.imag * lo.real,
        )
    }
    
    /// Detect synchronization (simplified)
    fn detect_sync(&mut self, _sample: Complex) -> bool {
        // TODO: Implement proper sync detection
        // For now, assume always synchronized
        self.is_sync = true;
        true
    }
}

impl Demodulator for BpskDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();

        let samples_per_symbol = self.config.samples_per_symbol() as usize;
        if samples_per_symbol == 0 { return Ok(()); }

        // Precompute baseband (real) using a local oscillator based on absolute index
        let omega = 2.0 * PI * self.config.carrier_frequency / self.config.sample_rate;
        let mut bb_real: Vec<f64> = Vec::with_capacity(samples.len());
        for (i, s) in samples.iter().enumerate() {
            let phase = omega * (i as f64);
            let c = phase.cos();
            let si = phase.sin();
            // Real part after downmix with conjugate LO: I*c + Q*si
            bb_real.push(s.real * c + s.imag * si);
        }

        // Try all symbol phase offsets, choose the one with earliest sync;
        // If none contain sync, choose the strongest (integrated magnitude) candidate.
        let sync: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E];
        let mut candidate_streams: Vec<Vec<u8>> = Vec::new();
        let mut candidate_strengths: Vec<f64> = Vec::new();
        let mut best_idx: Option<(usize, usize)> = None; // (offset, position)

        for offset in 0..samples_per_symbol {
            let mut tmp_bits: Vec<u8> = Vec::new();
            let mut tmp_bytes: Vec<u8> = Vec::new();
            let mut strength_acc: f64 = 0.0;

            let mut i = offset;
            while i + samples_per_symbol <= bb_real.len() {
                let mut acc = 0f64;
                let end = i + samples_per_symbol;
                let mut j = i;
                while j < end {
                    acc += bb_real[j];
                    j += 1;
                }
                strength_acc += acc.abs();
                let bit = if acc > 0.0 { 1u8 } else { 0u8 };
                tmp_bits.push(bit);
                if tmp_bits.len() == 8 {
                    let mut byte = 0u8;
                    for (k, &b) in tmp_bits.iter().enumerate() {
                        if b != 0 { byte |= 1 << (7 - k); }
                    }
                    tmp_bytes.push(byte);
                    tmp_bits.clear();
                }
                i += samples_per_symbol;
            }
            if !tmp_bits.is_empty() {
                let mut byte = 0u8;
                for (k, &b) in tmp_bits.iter().enumerate() {
                    if b != 0 { byte |= 1 << (7 - k); }
                }
                tmp_bytes.push(byte);
            }

            // Skip leading CW tone artifacts: trim leading 0x00/0xFF when searching for sync
            let mut search_start = 0usize;
            while search_start < tmp_bytes.len() && (tmp_bytes[search_start] == 0x00 || tmp_bytes[search_start] == 0xFF) {
                search_start += 1;
            }

            // Search for sync sequence in tmp_bytes (from trimmed start)
            let mut found_pos: Option<usize> = None;
            if tmp_bytes.len() >= sync.len() && search_start <= tmp_bytes.len() - sync.len() {
                for pos in search_start..=tmp_bytes.len() - sync.len() {
                    if &tmp_bytes[pos..pos + sync.len()] == sync {
                        found_pos = Some(pos);
                        break;
                    }
                }
            }

            if let Some(pos) = found_pos {
                match best_idx {
                    None => best_idx = Some((offset, pos)),
                    Some((_, best_pos)) => {
                        if pos < best_pos { best_idx = Some((offset, pos)); }
                    }
                }
            }

            candidate_strengths.push(strength_acc);
            candidate_streams.push(tmp_bytes);
        }

        // Output the best candidate if found by sync
        if let Some((best_offset, _)) = best_idx {
            output.extend_from_slice(&candidate_streams[best_offset]);
            self.is_sync = true;
            return Ok(());
        }

        // Otherwise, choose the strongest candidate by integrated magnitude
        if !candidate_streams.is_empty() {
            let mut best_o = 0usize;
            let mut best_s = candidate_strengths[0];
            for o in 1..candidate_strengths.len() {
                if candidate_strengths[o] > best_s {
                    best_s = candidate_strengths[o];
                    best_o = o;
                }
            }
            output.extend_from_slice(&candidate_streams[best_o]);
            self.is_sync = true;
            return Ok(());
        }

        Ok(())
    }
    
    fn is_synchronized(&self) -> bool {
        self.is_sync
    }
    
    fn signal_quality(&self) -> SignalQuality {
        self.signal_quality.clone()
    }
    
    fn reset(&mut self) {
        self.pulse_shaper.reset();
        self.phase = 0.0;
        self.sample_counter = 0.0;
        self.is_sync = false;
        self.signal_quality = SignalQuality::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpsk_modulator_creation() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let modulator = BpskModulator::new(config).unwrap();
        assert_eq!(modulator.symbol_rate(), 1000.0);
        assert_eq!(modulator.samples_per_symbol(), 48);
    }

    #[test]
    fn test_bpsk_demodulator_creation() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let demodulator = BpskDemodulator::new(config).unwrap();
        assert!(!demodulator.is_synchronized());
    }

    #[test]
    fn test_bpsk_modulation() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let mut modulator = BpskModulator::new(config).unwrap();
        
        let data = vec![0b10101010];
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        
        // Should produce samples for 8 bits
        let expected_samples = 8 * modulator.samples_per_symbol();
        assert_eq!(output.len(), expected_samples);
    }
}
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
    signal_quality: SignalQuality,
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
            signal_quality: SignalQuality::default(),
        })
    }

    /// Recover the bit stream at the best symbol-timing offset using per-symbol
    /// noncoherent mark/space energy detection. Trying every offset lets the
    /// receiver lock onto a burst that begins anywhere in the stream.
    fn demod_bits(&self, samples: &[Complex]) -> (Vec<u8>, SignalQuality) {
        let sps = self.config.samples_per_symbol() as usize;
        if sps == 0 || samples.len() < sps {
            return (Vec::new(), SignalQuality::default());
        }
        let mut best_bits: Vec<u8> = Vec::new();
        let mut best_energies: Vec<(f64, f64)> = Vec::new();
        let mut best_strength = -1.0f64;
        for offset in 0..sps {
            let mut bits = Vec::new();
            let mut energies = Vec::new();
            let mut strength = 0.0f64;
            let mut idx = offset;
            while idx + sps <= samples.len() {
                let win = &samples[idx..idx + sps];
                let (mut mi, mut mq, mut si, mut sq) = (0.0, 0.0, 0.0, 0.0);
                for (k, s) in win.iter().enumerate() {
                    let t = k as f64 / self.config.sample_rate;
                    mi += s.real * (2.0 * PI * self.freq_mark * t).cos();
                    mq += s.real * -(2.0 * PI * self.freq_mark * t).sin();
                    si += s.real * (2.0 * PI * self.freq_space * t).cos();
                    sq += s.real * -(2.0 * PI * self.freq_space * t).sin();
                }
                let e_mark = mi * mi + mq * mq;
                let e_space = si * si + sq * sq;
                strength += (e_mark - e_space).abs();
                energies.push((e_mark.max(e_space), e_mark.min(e_space)));
                bits.push(if e_mark > e_space { 1 } else { 0 });
                idx += sps;
            }
            if strength > best_strength {
                best_strength = strength;
                best_bits = bits;
                best_energies = energies;
            }
        }
        (best_bits, crate::common::discrimination_quality(&best_energies))
    }
}

impl Demodulator for FskDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();
        let (bits, quality) = self.demod_bits(samples);
        self.signal_quality = quality;

        // Pack bits into bytes, MSB first.
        let mut byte = 0u8;
        let mut n = 0;
        for b in bits {
            byte = (byte << 1) | (b & 1);
            n += 1;
            if n == 8 {
                output.push(byte);
                byte = 0;
                n = 0;
            }
        }
        Ok(())
    }

    fn is_synchronized(&self) -> bool {
        true // Timing recovery always produces a stream; framing handles sync.
    }

    fn signal_quality(&self) -> SignalQuality {
        self.signal_quality.clone()
    }

    fn reset(&mut self) {
        self.signal_quality = SignalQuality::default();
    }
}

impl crate::common::BitDemodulator for FskDemodulator {
    fn demodulate_bits(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();
        let (bits, quality) = self.demod_bits(samples);
        self.signal_quality = quality;
        *output = bits;
        Ok(())
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

    fn roundtrip(baud: f64, payload: &[u8]) -> Vec<u8> {
        let config = ModulationConfig::new(48000.0, baud, 1500.0).unwrap();
        let mut m = FskModulator::new(config.clone()).unwrap();
        let mut samples = Vec::new();
        m.modulate(payload, &mut samples).unwrap();
        let mut d = FskDemodulator::new(config).unwrap();
        let mut out = Vec::new();
        d.demodulate(&samples, &mut out).unwrap();
        out
    }

    #[test]
    fn fsk_raw_roundtrip_clean() {
        // A clean modulate -> demodulate must reproduce the payload exactly,
        // across symbol rates. (Regression: the no-sync fallback used to pick a
        // misaligned offset, yielding ~random bits.)
        let payload = b"FSK ROUNDTRIP TEST 12345";
        for baud in [125.0, 250.0, 1200.0] {
            assert_eq!(roundtrip(baud, payload), payload, "baud {baud}");
        }
    }
}
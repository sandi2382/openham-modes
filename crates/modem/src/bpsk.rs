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

impl BpskDemodulator {
    /// Recover the bit stream at the best symbol timing, with carrier phase
    /// recovery so a coherent BPSK burst that starts at an arbitrary offset
    /// (hence an arbitrary carrier phase) still decodes.
    ///
    /// Timing is chosen by total complex symbol magnitude, which is
    /// phase-invariant. The residual carrier phase is estimated by squaring the
    /// symbols — which strips the 0/π data modulation — averaging, and halving
    /// the resulting angle. A 180° ambiguity remains (squaring loses the sign);
    /// the framing layer's inversion-tolerant sync search resolves it.
    fn recover_bits(&self, samples: &[Complex]) -> (Vec<u8>, SignalQuality) {
        let sps = self.config.samples_per_symbol() as usize;
        if sps == 0 || samples.len() < sps {
            return (Vec::new(), SignalQuality::default());
        }
        let omega = 2.0 * PI * self.config.carrier_frequency / self.config.sample_rate;

        // Downmix to complex baseband with a conjugate LO at the absolute index.
        let mut bb: Vec<Complex> = Vec::with_capacity(samples.len());
        for (i, s) in samples.iter().enumerate() {
            let phase = omega * (i as f64);
            let (c, sn) = (phase.cos(), phase.sin());
            bb.push(Complex::new(s.real * c + s.imag * sn, s.imag * c - s.real * sn));
        }

        // Symbol-timing offset search by total symbol magnitude (phase-invariant).
        let mut best_syms: Vec<Complex> = Vec::new();
        let mut best_strength = -1.0f64;
        for offset in 0..sps {
            let mut syms = Vec::new();
            let mut strength = 0.0f64;
            let mut i = offset;
            while i + sps <= bb.len() {
                let (mut re, mut im) = (0.0f64, 0.0f64);
                for s in &bb[i..i + sps] {
                    re += s.real;
                    im += s.imag;
                }
                let sym = Complex::new(re, im);
                strength += sym.magnitude();
                syms.push(sym);
                i += sps;
            }
            if strength > best_strength {
                best_strength = strength;
                best_syms = syms;
            }
        }

        // Carrier phase recovery: the average of the squared symbols has phase 2θ.
        let (mut s2re, mut s2im) = (0.0f64, 0.0f64);
        for s in &best_syms {
            s2re += s.real * s.real - s.imag * s.imag;
            s2im += 2.0 * s.real * s.imag;
        }
        let theta = 0.5 * s2im.atan2(s2re);
        let (ct, st) = (theta.cos(), theta.sin());

        // Decide bits on the rotated real axis. Estimate quality (EVM, and SNR
        // derived from it) over the strong, signal-bearing symbols only, so that
        // dead air at the start/end of the capture does not skew the result.
        let max_mag = best_syms.iter().map(|s| s.magnitude()).fold(0.0_f64, f64::max);
        let thresh = 0.3 * max_mag;

        let mut bits = Vec::with_capacity(best_syms.len());
        let (mut amp_sum, mut count) = (0.0f64, 0usize);
        for s in &best_syms {
            let re = s.real * ct + s.imag * st;
            bits.push(if re > 0.0 { 1u8 } else { 0u8 });
            if s.magnitude() > thresh {
                amp_sum += re.abs();
                count += 1;
            }
        }
        let amp = if count > 0 { amp_sum / count as f64 } else { 0.0 };

        let mut err2 = 0.0;
        if amp > 0.0 {
            for s in &best_syms {
                if s.magnitude() > thresh {
                    let re = s.real * ct + s.imag * st;
                    let im = s.imag * ct - s.real * st; // quadrature error after rotation
                    let ideal = if re > 0.0 { amp } else { -amp };
                    err2 += (re - ideal) * (re - ideal) + im * im;
                }
            }
        }

        let quality = if amp > 0.0 && count > 0 {
            let evm = ((err2 / count as f64).sqrt() / amp).min(1.0);
            SignalQuality {
                // EVM^2 ~ 1/SNR for a 2-D constellation, so SNR(dB) = -20log10(EVM).
                snr_db: if evm > 0.0 { -20.0 * evm.log10() } else { 99.0 },
                evm_percent: evm * 100.0,
                phase_error_deg: theta.to_degrees(),
                ..Default::default()
            }
        } else {
            SignalQuality::default()
        };

        (bits, quality)
    }
}

impl Demodulator for BpskDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();
        let (bits, quality) = self.recover_bits(samples);
        self.signal_quality = quality;
        self.is_sync = !bits.is_empty();

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

impl crate::common::BitDemodulator for BpskDemodulator {
    fn demodulate_bits(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();
        let (bits, quality) = self.recover_bits(samples);
        self.signal_quality = quality;
        *output = bits;
        Ok(())
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
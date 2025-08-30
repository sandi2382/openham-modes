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
        let phase = omega * self.sample_counter + symbol * PI;
        
        self.sample_counter += 1.0;
        
        Complex::new(phase.cos(), phase.sin())
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
                    let baseband = Complex::new(symbol, 0.0);
                    let shaped = self.pulse_shaper.filter(baseband);
                    let modulated = self.generate_carrier(symbol);
                    
                    output.push(Complex::new(
                        shaped.real * modulated.real - shaped.imag * modulated.imag,
                        shaped.real * modulated.imag + shaped.imag * modulated.real,
                    ));
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
        
        let lo = Complex::new(phase.cos(), -phase.sin());
        
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
        let mut bit_buffer = Vec::new();
        
        for (i, &sample) in samples.iter().enumerate() {
            // Demodulate to baseband
            let baseband = self.demodulate_to_baseband(sample);
            let shaped = self.pulse_shaper.filter(baseband);
            
            // Check for sync
            if !self.is_sync {
                self.detect_sync(shaped);
                continue;
            }
            
            // Symbol sampling (simplified - should use proper timing recovery)
            if i % samples_per_symbol == samples_per_symbol / 2 {
                let bit = if shaped.real > 0.0 { 1 } else { 0 };
                bit_buffer.push(bit);
                
                // Pack bits into bytes
                if bit_buffer.len() == 8 {
                    let mut byte = 0u8;
                    for (j, &bit) in bit_buffer.iter().enumerate() {
                        if bit != 0 {
                            byte |= 1 << (7 - j);
                        }
                    }
                    output.push(byte);
                    bit_buffer.clear();
                }
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
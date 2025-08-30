//! Common modulation traits and utilities

use crate::{ModemError, Result};
use openham_core::buffer::Complex;
use serde::{Deserialize, Serialize};

/// Generic modulator trait
pub trait Modulator {
    /// Modulate bits to complex samples
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()>;
    
    /// Get samples per symbol
    fn samples_per_symbol(&self) -> usize;
    
    /// Get symbol rate
    fn symbol_rate(&self) -> f64;
    
    /// Reset modulator state
    fn reset(&mut self);
}

/// Generic demodulator trait
pub trait Demodulator {
    /// Demodulate complex samples to bits
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()>;
    
    /// Check if synchronized to signal
    fn is_synchronized(&self) -> bool;
    
    /// Get signal quality metrics
    fn signal_quality(&self) -> SignalQuality;
    
    /// Reset demodulator state
    fn reset(&mut self);
}

/// Signal quality metrics
#[derive(Debug, Clone, Default)]
pub struct SignalQuality {
    pub snr_db: f64,
    pub evm_percent: f64,
    pub frequency_offset_hz: f64,
    pub timing_offset_samples: f64,
    pub phase_error_deg: f64,
}

/// Common modulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulationConfig {
    pub sample_rate: f64,
    pub symbol_rate: f64,
    pub carrier_frequency: f64,
    pub rolloff_factor: f64,
    pub filter_length: usize,
}

impl ModulationConfig {
    /// Create a new modulation configuration
    pub fn new(
        sample_rate: f64,
        symbol_rate: f64,
        carrier_frequency: f64,
    ) -> Result<Self> {
        if sample_rate <= 0.0 {
            return Err(ModemError::InvalidParameters {
                msg: format!("Invalid sample rate: {}", sample_rate),
            });
        }
        
        if symbol_rate <= 0.0 || symbol_rate > sample_rate / 2.0 {
            return Err(ModemError::InvalidParameters {
                msg: format!("Invalid symbol rate: {}", symbol_rate),
            });
        }
        
        Ok(Self {
            sample_rate,
            symbol_rate,
            carrier_frequency,
            rolloff_factor: 0.35,
            filter_length: 101,
        })
    }
    
    /// Get samples per symbol
    pub fn samples_per_symbol(&self) -> f64 {
        self.sample_rate / self.symbol_rate
    }
    
    /// Set rolloff factor for pulse shaping
    pub fn with_rolloff(mut self, rolloff: f64) -> Result<Self> {
        if !(0.0..=1.0).contains(&rolloff) {
            return Err(ModemError::InvalidParameters {
                msg: format!("Invalid rolloff factor: {}", rolloff),
            });
        }
        self.rolloff_factor = rolloff;
        Ok(self)
    }
    
    /// Set filter length for pulse shaping
    pub fn with_filter_length(mut self, length: usize) -> Result<Self> {
        if length == 0 || length % 2 == 0 {
            return Err(ModemError::InvalidParameters {
                msg: "Filter length must be odd and greater than 0".to_string(),
            });
        }
        self.filter_length = length;
        Ok(self)
    }
}

/// Pulse shaping filter design
pub struct PulseShaper {
    filter_taps: Vec<f64>,
    delay_line: Vec<Complex>,
    index: usize,
}

impl PulseShaper {
    /// Create a root raised cosine pulse shaping filter
    pub fn root_raised_cosine(
        samples_per_symbol: f64,
        rolloff: f64,
        length: usize,
    ) -> Result<Self> {
        if length == 0 || length % 2 == 0 {
            return Err(ModemError::InvalidParameters {
                msg: "Filter length must be odd and greater than 0".to_string(),
            });
        }
        
        let mut filter_taps = Vec::with_capacity(length);
        let center = (length - 1) as f64 / 2.0;
        
        for i in 0..length {
            let t = (i as f64 - center) / samples_per_symbol;
            let tap = Self::rrc_impulse_response(t, rolloff);
            filter_taps.push(tap);
        }
        
        // Normalize
        let sum: f64 = filter_taps.iter().sum();
        if sum != 0.0 {
            for tap in &mut filter_taps {
                *tap /= sum;
            }
        }
        
        Ok(Self {
            filter_taps,
            delay_line: vec![Complex::default(); length],
            index: 0,
        })
    }
    
    /// Root raised cosine impulse response
    fn rrc_impulse_response(t: f64, rolloff: f64) -> f64 {
        if t == 0.0 {
            return 1.0 - rolloff + 4.0 * rolloff / std::f64::consts::PI;
        }
        
        let abs_t = t.abs();
        
        // Check for special case where denominator would be zero
        if (4.0 * rolloff * abs_t - 1.0).abs() < 1e-10 {
            let numerator = rolloff * (
                (1.0 + 2.0 / std::f64::consts::PI) * (std::f64::consts::PI / 4.0).sin() +
                (1.0 - 2.0 / std::f64::consts::PI) * (std::f64::consts::PI / 4.0).cos()
            );
            return numerator / (2.0_f64).sqrt();
        }
        
        let numerator = (std::f64::consts::PI * abs_t * (1.0 - rolloff)).sin() +
                       4.0 * rolloff * abs_t * (std::f64::consts::PI * abs_t * (1.0 + rolloff)).cos();
        
        let denominator = std::f64::consts::PI * abs_t * (1.0 - (4.0 * rolloff * abs_t).powi(2));
        
        numerator / denominator
    }
    
    /// Filter a complex sample
    pub fn filter(&mut self, input: Complex) -> Complex {
        // Store input in delay line
        self.delay_line[self.index] = input;
        
        // Compute convolution
        let mut output = Complex::new(0.0, 0.0);
        for (i, &tap) in self.filter_taps.iter().enumerate() {
            let delay_index = (self.index + self.delay_line.len() - i) % self.delay_line.len();
            let sample = self.delay_line[delay_index];
            output.real += tap * sample.real;
            output.imag += tap * sample.imag;
        }
        
        // Update delay line index
        self.index = (self.index + 1) % self.delay_line.len();
        
        output
    }
    
    /// Reset filter state
    pub fn reset(&mut self) {
        self.delay_line.fill(Complex::default());
        self.index = 0;
    }
}

/// Symbol timing recovery using Gardner algorithm
pub struct GardnerTimingRecovery {
    samples_per_symbol: f64,
    loop_bandwidth: f64,
    damping_factor: f64,
    phase: f64,
    freq: f64,
    prev_sample: Complex,
    prev_error: f64,
}

impl GardnerTimingRecovery {
    /// Create a new Gardner timing recovery loop
    pub fn new(
        samples_per_symbol: f64,
        loop_bandwidth: f64,
        damping_factor: f64,
    ) -> Self {
        Self {
            samples_per_symbol,
            loop_bandwidth,
            damping_factor,
            phase: 0.0,
            freq: samples_per_symbol,
            prev_sample: Complex::default(),
            prev_error: 0.0,
        }
    }
    
    /// Process samples and return interpolated symbols
    pub fn process(&mut self, samples: &[Complex], symbols: &mut Vec<Complex>) -> Result<()> {
        symbols.clear();
        
        for &sample in samples {
            self.phase += 1.0;
            
            if self.phase >= self.freq {
                // Symbol sampling point
                self.phase -= self.freq;
                
                // Interpolate symbol (simple linear interpolation)
                let frac = self.phase / self.freq;
                let symbol = Complex::new(
                    self.prev_sample.real * (1.0 - frac) + sample.real * frac,
                    self.prev_sample.imag * (1.0 - frac) + sample.imag * frac,
                );
                symbols.push(symbol);
                
                // Compute timing error (Gardner algorithm)
                let error = (self.prev_sample.real * sample.real + self.prev_sample.imag * sample.imag) * 
                           (sample.magnitude() - self.prev_sample.magnitude());
                
                // Update frequency based on error
                let freq_update = self.loop_bandwidth * error;
                self.freq += freq_update;
                
                // Clamp frequency to reasonable range
                self.freq = self.freq.clamp(
                    self.samples_per_symbol * 0.9,
                    self.samples_per_symbol * 1.1,
                );
                
                self.prev_error = error;
            }
            
            self.prev_sample = sample;
        }
        
        Ok(())
    }
    
    /// Reset timing recovery state
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.freq = self.samples_per_symbol;
        self.prev_sample = Complex::default();
        self.prev_error = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modulation_config() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        assert_eq!(config.sample_rate, 48000.0);
        assert_eq!(config.symbol_rate, 1000.0);
        assert_eq!(config.samples_per_symbol(), 48.0);
    }

    #[test]
    fn test_pulse_shaper_creation() {
        let shaper = PulseShaper::root_raised_cosine(4.0, 0.35, 41).unwrap();
        assert_eq!(shaper.filter_taps.len(), 41);
    }

    #[test]
    fn test_invalid_config() {
        assert!(ModulationConfig::new(-1.0, 1000.0, 1500.0).is_err());
        assert!(ModulationConfig::new(48000.0, 50000.0, 1500.0).is_err());
    }
}
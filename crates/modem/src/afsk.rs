//! Audio Frequency Shift Keying (AFSK) implementation
//! 
//! AFSK is commonly used in amateur radio packet systems (like AX.25).
//! It modulates digital data by shifting between two audio frequencies.

use crate::{ModemError, Result};
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality};
use openham_core::buffer::Complex;
use std::f64::consts::PI;

/// AFSK configuration parameters
#[derive(Debug, Clone)]
pub struct AfskConfig {
    pub mark_frequency: f64,    // Frequency for '1' bits (typically 1200 Hz)
    pub space_frequency: f64,   // Frequency for '0' bits (typically 2200 Hz)
    pub baud_rate: f64,         // Symbol rate (typically 1200 baud)
    pub filter_bandwidth: f64,  // Audio filter bandwidth
}

impl AfskConfig {
    /// Create Bell 202 compatible AFSK configuration (1200 baud)
    pub fn bell_202() -> Self {
        Self {
            mark_frequency: 1200.0,
            space_frequency: 2200.0,
            baud_rate: 1200.0,
            filter_bandwidth: 2400.0,
        }
    }
    
    /// Create Bell 103 compatible AFSK configuration (300 baud)
    pub fn bell_103() -> Self {
        Self {
            mark_frequency: 1270.0,
            space_frequency: 1070.0,
            baud_rate: 300.0,
            filter_bandwidth: 600.0,
        }
    }
    
    /// Create VHF packet configuration (1200 baud)
    pub fn vhf_packet() -> Self {
        Self {
            mark_frequency: 1200.0,
            space_frequency: 2200.0,
            baud_rate: 1200.0,
            filter_bandwidth: 3000.0,
        }
    }
    
    /// Create HF packet configuration (300 baud)
    pub fn hf_packet() -> Self {
        Self {
            mark_frequency: 1600.0,
            space_frequency: 1800.0,
            baud_rate: 300.0,
            filter_bandwidth: 500.0,
        }
    }
}

/// AFSK modulator
pub struct AfskModulator {
    config: ModulationConfig,
    afsk_config: AfskConfig,
    phase_mark: f64,
    phase_space: f64,
    sample_counter: f64,
    bit_duration: f64,
    current_bit_samples: f64,
    current_bit: u8,
    bit_buffer: Vec<u8>,
    bit_index: usize,
}

impl AfskModulator {
    /// Create a new AFSK modulator
    pub fn new(config: ModulationConfig, afsk_config: AfskConfig) -> Result<Self> {
        let bit_duration = config.sample_rate / afsk_config.baud_rate;
        
        Ok(Self {
            config,
            afsk_config,
            phase_mark: 0.0,
            phase_space: 0.0,
            sample_counter: 0.0,
            bit_duration,
            current_bit_samples: 0.0,
            current_bit: 0,
            bit_buffer: Vec::new(),
            bit_index: 0,
        })
    }
    
    /// Generate AFSK sample for current bit
    fn generate_sample(&mut self) -> f64 {
        let frequency = if self.current_bit == 1 {
            self.afsk_config.mark_frequency
        } else {
            self.afsk_config.space_frequency
        };
        
        let omega = 2.0 * PI * frequency / self.config.sample_rate;
        let phase = omega * self.sample_counter;
        
        self.sample_counter += 1.0;
        
        phase.sin()
    }
    
    /// Get next bit from buffer
    fn get_next_bit(&mut self) -> Option<u8> {
        if self.bit_index >= self.bit_buffer.len() * 8 {
            return None;
        }
        
        let byte_index = self.bit_index / 8;
        let bit_position = 7 - (self.bit_index % 8); // MSB first
        let bit = (self.bit_buffer[byte_index] >> bit_position) & 1;
        
        self.bit_index += 1;
        Some(bit)
    }
}

impl Modulator for AfskModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        // Store bits for processing
        self.bit_buffer = bits.to_vec();
        self.bit_index = 0;
        self.current_bit_samples = 0.0;
        
        // Get first bit
        if let Some(bit) = self.get_next_bit() {
            self.current_bit = bit;
        } else {
            return Ok(()); // No data to modulate
        }
        
        // Generate samples
        while self.bit_index <= self.bit_buffer.len() * 8 {
            // Check if we need to advance to next bit
            if self.current_bit_samples >= self.bit_duration {
                self.current_bit_samples = 0.0;
                if let Some(bit) = self.get_next_bit() {
                    self.current_bit = bit;
                } else {
                    break;
                }
            }
            
            // Generate sample for current bit
            let sample = self.generate_sample();
            output.push(Complex::new(sample, 0.0));
            
            self.current_bit_samples += 1.0;
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        self.bit_duration as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        self.afsk_config.baud_rate
    }
    
    fn reset(&mut self) {
        self.phase_mark = 0.0;
        self.phase_space = 0.0;
        self.sample_counter = 0.0;
        self.current_bit_samples = 0.0;
        self.current_bit = 0;
        self.bit_buffer.clear();
        self.bit_index = 0;
    }
}

/// AFSK demodulator using dual tone detection
pub struct AfskDemodulator {
    config: ModulationConfig,
    afsk_config: AfskConfig,
    mark_correlator: ToneDetector,
    space_correlator: ToneDetector,
    bit_duration: f64,
    sample_counter: f64,
    bit_samples: f64,
    sync_detected: bool,
    signal_quality: SignalQuality,
}

impl AfskDemodulator {
    /// Create a new AFSK demodulator
    pub fn new(config: ModulationConfig, afsk_config: AfskConfig) -> Result<Self> {
        let bit_duration = config.sample_rate / afsk_config.baud_rate;
        
        let mark_correlator = ToneDetector::new(
            afsk_config.mark_frequency,
            config.sample_rate,
            64, // correlation window
        )?;
        
        let space_correlator = ToneDetector::new(
            afsk_config.space_frequency,
            config.sample_rate,
            64,
        )?;
        
        Ok(Self {
            config,
            afsk_config,
            mark_correlator,
            space_correlator,
            bit_duration,
            sample_counter: 0.0,
            bit_samples: 0.0,
            sync_detected: false,
            signal_quality: SignalQuality::default(),
        })
    }
    
    /// Detect bit based on tone correlation
    fn detect_bit(&mut self, sample: f64) -> Option<u8> {
        let mark_level = self.mark_correlator.process(sample);
        let space_level = self.space_correlator.process(sample);
        
        self.bit_samples += 1.0;
        
        // Sample at middle of bit period
        if self.bit_samples >= self.bit_duration {
            self.bit_samples = 0.0;
            
            // Update signal quality metrics
            let total_power = mark_level + space_level;
            if total_power > 0.0 {
                let snr = if mark_level > space_level {
                    20.0 * (mark_level / space_level).log10()
                } else {
                    20.0 * (space_level / mark_level).log10()
                };
                self.signal_quality.snr_db = snr;
            }
            
            // Determine bit value
            Some(if mark_level > space_level { 1 } else { 0 })
        } else {
            None
        }
    }
}

impl Demodulator for AfskDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();
        
        let mut bits = Vec::new();
        
        for &sample in samples {
            if let Some(bit) = self.detect_bit(sample.real) {
                bits.push(bit);
                
                // Start sync detection after getting some bits
                if !self.sync_detected && bits.len() > 16 {
                    self.sync_detected = true; // Simplified sync detection
                }
            }
        }
        
        // Pack bits into bytes
        let mut byte_value = 0u8;
        let mut bit_count = 0;
        
        for bit in bits {
            byte_value = (byte_value << 1) | bit;
            bit_count += 1;
            
            if bit_count == 8 {
                output.push(byte_value);
                byte_value = 0;
                bit_count = 0;
            }
        }
        
        Ok(())
    }
    
    fn is_synchronized(&self) -> bool {
        self.sync_detected
    }
    
    fn signal_quality(&self) -> SignalQuality {
        self.signal_quality.clone()
    }
    
    fn reset(&mut self) {
        self.mark_correlator.reset();
        self.space_correlator.reset();
        self.sample_counter = 0.0;
        self.bit_samples = 0.0;
        self.sync_detected = false;
        self.signal_quality = SignalQuality::default();
    }
}

/// Simple tone detector using correlation
struct ToneDetector {
    frequency: f64,
    sample_rate: f64,
    samples: Vec<f64>,
    cos_ref: Vec<f64>,
    sin_ref: Vec<f64>,
    index: usize,
}

impl ToneDetector {
    fn new(frequency: f64, sample_rate: f64, window_size: usize) -> Result<Self> {
        let mut cos_ref = Vec::with_capacity(window_size);
        let mut sin_ref = Vec::with_capacity(window_size);
        
        for i in 0..window_size {
            let phase = 2.0 * PI * frequency * i as f64 / sample_rate;
            cos_ref.push(phase.cos());
            sin_ref.push(phase.sin());
        }
        
        Ok(Self {
            frequency,
            sample_rate,
            samples: vec![0.0; window_size],
            cos_ref,
            sin_ref,
            index: 0,
        })
    }
    
    fn process(&mut self, sample: f64) -> f64 {
        // Store sample in circular buffer
        self.samples[self.index] = sample;
        self.index = (self.index + 1) % self.samples.len();
        
        // Compute correlation with reference signals
        let mut i_sum = 0.0;
        let mut q_sum = 0.0;
        
        for i in 0..self.samples.len() {
            let sample_idx = (self.index + i) % self.samples.len();
            i_sum += self.samples[sample_idx] * self.cos_ref[i];
            q_sum += self.samples[sample_idx] * self.sin_ref[i];
        }
        
        // Return magnitude
        (i_sum * i_sum + q_sum * q_sum).sqrt()
    }
    
    fn reset(&mut self) {
        self.samples.fill(0.0);
        self.index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_afsk_config_creation() {
        let config = AfskConfig::bell_202();
        assert_eq!(config.mark_frequency, 1200.0);
        assert_eq!(config.space_frequency, 2200.0);
        assert_eq!(config.baud_rate, 1200.0);
    }

    #[test]
    fn test_afsk_modulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1200.0, 1700.0).unwrap();
        let afsk_config = AfskConfig::bell_202();
        let _modulator = AfskModulator::new(mod_config, afsk_config).unwrap();
    }

    #[test]
    fn test_afsk_demodulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1200.0, 1700.0).unwrap();
        let afsk_config = AfskConfig::bell_202();
        let _demodulator = AfskDemodulator::new(mod_config, afsk_config).unwrap();
    }

    #[test]
    fn test_tone_detector() {
        let mut detector = ToneDetector::new(1000.0, 8000.0, 32).unwrap();
        
        // Test with 1000 Hz tone
        for i in 0..100 {
            let phase = 2.0 * PI * 1000.0 * i as f64 / 8000.0;
            let sample = phase.sin();
            let level = detector.process(sample);
            
            // Should detect significant correlation after enough samples
            if i > 50 {
                assert!(level > 0.1);
            }
        }
    }

    #[test]
    fn test_afsk_modulation() {
        let mod_config = ModulationConfig::new(48000.0, 1200.0, 1700.0).unwrap();
        let afsk_config = AfskConfig::bell_202();
        let mut modulator = AfskModulator::new(mod_config, afsk_config).unwrap();
        
        let data = vec![0b10101010]; // Alternating bits
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        
        // Should generate samples
        assert!(!output.is_empty());
        
        // Should have roughly correct number of samples
        let expected_samples = 8 * modulator.samples_per_symbol();
        assert!(output.len() >= expected_samples);
    }
}
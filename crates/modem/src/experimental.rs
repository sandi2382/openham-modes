//! Experimental digital mode implementations
//! 
//! This module contains experimental encoding schemes that map bits into
//! tones, symbols, or waveforms in non-standard ways for special applications.

use crate::{ModemError, Result};
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality};
use openham_core::buffer::Complex;
use std::f64::consts::PI;

/// Multi-tone frequency mapping encoder
/// Maps groups of bits to specific frequency combinations
#[derive(Debug, Clone)]
pub struct MultiToneConfig {
    pub base_frequency: f64,      // Base frequency in Hz
    pub tone_spacing: f64,        // Spacing between tones in Hz
    pub num_tones: usize,         // Number of available tones
    pub tones_per_symbol: usize,  // Number of simultaneous tones per symbol
    pub symbol_duration: f64,     // Symbol duration in seconds
}

impl MultiToneConfig {
    /// Create a configuration for 16-tone MFSK-like system
    pub fn sixteen_tone() -> Self {
        Self {
            base_frequency: 1000.0,
            tone_spacing: 15.625,  // 16 tones in 250 Hz bandwidth
            num_tones: 16,
            tones_per_symbol: 1,   // Traditional MFSK
            symbol_duration: 0.032, // 32ms symbols
        }
    }
    
    /// Create a multi-tone configuration for parallel transmission
    pub fn parallel_four_tone() -> Self {
        Self {
            base_frequency: 1000.0,
            tone_spacing: 50.0,    // Wide spacing for parallel tones
            num_tones: 8,          // 8 available tones
            tones_per_symbol: 4,   // 4 simultaneous tones
            symbol_duration: 0.064, // 64ms symbols for robustness
        }
    }
}

/// Chaos-based spread spectrum encoder
/// Uses chaotic sequences for spreading and encoding
pub struct ChaosModulator {
    config: ModulationConfig,
    chaos_config: ChaosConfig,
    chaos_state: f64,
    phase_accumulator: f64,
}

#[derive(Debug, Clone)]
pub struct ChaosConfig {
    pub logistic_parameter: f64,  // Logistic map parameter (3.8-4.0)
    pub initial_condition: f64,   // Initial chaos state
    pub chips_per_bit: usize,     // Spreading factor
    pub frequency_deviation: f64, // Max frequency deviation
}

impl ChaosConfig {
    pub fn default() -> Self {
        Self {
            logistic_parameter: 3.9,
            initial_condition: 0.123456789,
            chips_per_bit: 16,
            frequency_deviation: 200.0,
        }
    }
}

impl ChaosModulator {
    pub fn new(config: ModulationConfig, chaos_config: ChaosConfig) -> Self {
        Self {
            config,
            chaos_state: chaos_config.initial_condition,
            chaos_config,
            phase_accumulator: 0.0,
        }
    }
    
    /// Generate next chaotic value using logistic map
    fn next_chaos_value(&mut self) -> f64 {
        self.chaos_state = self.chaos_config.logistic_parameter * 
            self.chaos_state * (1.0 - self.chaos_state);
        self.chaos_state
    }
    
    /// Generate spreading sequence for a bit
    fn generate_spreading_sequence(&mut self, bit: u8) -> Vec<f64> {
        let mut sequence = Vec::new();
        
        for _ in 0..self.chaos_config.chips_per_bit {
            let chaos_val = self.next_chaos_value();
            
            // Map chaos value and bit to frequency
            let freq_offset = if bit == 1 {
                (chaos_val - 0.5) * self.chaos_config.frequency_deviation
            } else {
                -(chaos_val - 0.5) * self.chaos_config.frequency_deviation
            };
            
            sequence.push(freq_offset);
        }
        
        sequence
    }
}

impl Modulator for ChaosModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let samples_per_chip = (self.config.sample_rate / 
            (self.symbol_rate() * self.chaos_config.chips_per_bit as f64)) as usize;
        
        // Convert bytes to bits
        let mut bit_stream = Vec::new();
        for &byte in bits {
            for i in (0..8).rev() {
                bit_stream.push((byte >> i) & 1);
            }
        }
        
        for bit in bit_stream {
            // Generate spreading sequence
            let freq_sequence = self.generate_spreading_sequence(bit);
            
            // Generate samples for each chip
            for freq_offset in freq_sequence {
                let instantaneous_freq = self.config.carrier_frequency + freq_offset;
                
                for _ in 0..samples_per_chip {
                    let sample = Complex::new(
                        (2.0 * PI * instantaneous_freq * self.phase_accumulator / self.config.sample_rate).cos(),
                        (2.0 * PI * instantaneous_freq * self.phase_accumulator / self.config.sample_rate).sin(),
                    );
                    
                    output.push(sample);
                    self.phase_accumulator += 1.0;
                    
                    if self.phase_accumulator >= self.config.sample_rate {
                        self.phase_accumulator -= self.config.sample_rate;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        (self.config.sample_rate / self.symbol_rate()) as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        self.config.symbol_rate
    }
    
    fn reset(&mut self) {
        self.chaos_state = self.chaos_config.initial_condition;
        self.phase_accumulator = 0.0;
    }
}

/// Constellation rotation encoder
/// Dynamically rotates constellation based on data
pub struct RotatingConstellationModulator {
    config: ModulationConfig,
    rotation_angle: f64,
    rotation_step: f64,
    phase_accumulator: f64,
}

impl RotatingConstellationModulator {
    pub fn new(config: ModulationConfig, rotation_step_degrees: f64) -> Self {
        Self {
            config,
            rotation_angle: 0.0,
            rotation_step: rotation_step_degrees * PI / 180.0,
            phase_accumulator: 0.0,
        }
    }
    
    /// Rotate a complex point by the current rotation angle
    fn rotate_point(&self, point: Complex) -> Complex {
        Complex::new(
            point.real * self.rotation_angle.cos() - point.imag * self.rotation_angle.sin(),
            point.real * self.rotation_angle.sin() + point.imag * self.rotation_angle.cos(),
        )
    }
    
    /// Update rotation angle based on previous symbol
    fn update_rotation(&mut self, symbol_value: u8) {
        // Rotate based on symbol value
        self.rotation_angle += self.rotation_step * (symbol_value as f64 + 1.0);
        
        // Keep angle in [0, 2π]
        while self.rotation_angle >= 2.0 * PI {
            self.rotation_angle -= 2.0 * PI;
        }
    }
}

impl Modulator for RotatingConstellationModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let samples_per_symbol = self.samples_per_symbol();
        
        // Convert bytes to 4-bit symbols (16-QAM)
        let mut symbols = Vec::new();
        for &byte in bits {
            symbols.push((byte >> 4) & 0x0F);  // Upper nibble
            symbols.push(byte & 0x0F);         // Lower nibble
        }
        
        for symbol in symbols {
            // Base 16-QAM constellation
            let base_point = match symbol {
                0 => Complex::new(-3.0, -3.0),
                1 => Complex::new(-3.0, -1.0),
                2 => Complex::new(-3.0, 1.0),
                3 => Complex::new(-3.0, 3.0),
                4 => Complex::new(-1.0, -3.0),
                5 => Complex::new(-1.0, -1.0),
                6 => Complex::new(-1.0, 1.0),
                7 => Complex::new(-1.0, 3.0),
                8 => Complex::new(1.0, -3.0),
                9 => Complex::new(1.0, -1.0),
                10 => Complex::new(1.0, 1.0),
                11 => Complex::new(1.0, 3.0),
                12 => Complex::new(3.0, -3.0),
                13 => Complex::new(3.0, -1.0),
                14 => Complex::new(3.0, 1.0),
                15 => Complex::new(3.0, 3.0),
                _ => Complex::new(0.0, 0.0),
            };
            
            // Rotate the constellation point
            let rotated_point = self.rotate_point(base_point);
            
            // Generate samples for this symbol
            for i in 0..samples_per_symbol {
                let t = i as f64 / self.config.sample_rate;
                let carrier_phase = 2.0 * PI * self.config.carrier_frequency * t;
                
                // Apply pulse shaping (raised cosine approximation)
                let pulse_shape = if samples_per_symbol > 1 {
                    let alpha = 0.5;
                    let t_norm = (i as f64 / samples_per_symbol as f64) - 0.5;
                    if t_norm.abs() < 1e-6 {
                        1.0
                    } else {
                        let num = (PI * t_norm).sin() * (PI * alpha * t_norm).cos();
                        let den = PI * t_norm * (1.0 - (2.0 * alpha * t_norm).powi(2));
                        if den.abs() > 1e-6 { num / den } else { 0.0 }
                    }
                } else {
                    1.0
                };
                
                let modulated_sample = Complex::new(
                    pulse_shape * (rotated_point.real * carrier_phase.cos() - rotated_point.imag * carrier_phase.sin()),
                    pulse_shape * (rotated_point.real * carrier_phase.sin() + rotated_point.imag * carrier_phase.cos()),
                );
                
                output.push(modulated_sample);
            }
            
            // Update rotation for next symbol
            self.update_rotation(symbol);
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        (self.config.sample_rate / self.symbol_rate()) as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        self.config.symbol_rate
    }
    
    fn reset(&mut self) {
        self.rotation_angle = 0.0;
        self.phase_accumulator = 0.0;
    }
}

/// Frequency hopping spread spectrum
pub struct FrequencyHoppingModulator {
    config: ModulationConfig,
    hop_sequence: Vec<f64>,
    hop_index: usize,
    hop_duration: f64,
    time_in_hop: f64,
    phase_accumulator: f64,
}

impl FrequencyHoppingModulator {
    pub fn new(config: ModulationConfig, hop_frequencies: Vec<f64>, hop_duration: f64) -> Self {
        Self {
            config,
            hop_sequence: hop_frequencies,
            hop_index: 0,
            hop_duration,
            time_in_hop: 0.0,
            phase_accumulator: 0.0,
        }
    }
    
    /// Generate default hop sequence
    pub fn with_default_hops(config: ModulationConfig) -> Self {
        let base_freq = config.carrier_frequency;
        let hop_frequencies = vec![
            base_freq - 150.0,
            base_freq - 50.0,
            base_freq + 50.0,
            base_freq + 150.0,
            base_freq - 100.0,
            base_freq + 100.0,
            base_freq - 200.0,
            base_freq + 200.0,
        ];
        
        Self::new(config, hop_frequencies, 0.1) // 100ms hops
    }
    
    fn current_frequency(&self) -> f64 {
        if self.hop_sequence.is_empty() {
            self.config.carrier_frequency
        } else {
            self.hop_sequence[self.hop_index]
        }
    }
    
    fn advance_hop(&mut self) {
        if !self.hop_sequence.is_empty() {
            self.hop_index = (self.hop_index + 1) % self.hop_sequence.len();
        }
        self.time_in_hop = 0.0;
    }
}

impl Modulator for FrequencyHoppingModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let samples_per_symbol = self.samples_per_symbol();
        let sample_duration = 1.0 / self.config.sample_rate;
        
        // Simple FSK modulation with frequency hopping
        for &byte in bits {
            for bit_pos in (0..8).rev() {
                let bit = (byte >> bit_pos) & 1;
                let freq_offset = if bit == 1 { 100.0 } else { -100.0 }; // FSK deviation
                
                for _ in 0..samples_per_symbol {
                    // Check if we need to hop
                    if self.time_in_hop >= self.hop_duration {
                        self.advance_hop();
                    }
                    
                    let current_freq = self.current_frequency() + freq_offset;
                    
                    let sample = Complex::new(
                        (2.0 * PI * current_freq * self.phase_accumulator / self.config.sample_rate).cos(),
                        (2.0 * PI * current_freq * self.phase_accumulator / self.config.sample_rate).sin(),
                    );
                    
                    output.push(sample);
                    
                    self.phase_accumulator += 1.0;
                    if self.phase_accumulator >= self.config.sample_rate {
                        self.phase_accumulator -= self.config.sample_rate;
                    }
                    
                    self.time_in_hop += sample_duration;
                }
            }
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        (self.config.sample_rate / self.symbol_rate()) as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        self.config.symbol_rate
    }
    
    fn reset(&mut self) {
        self.hop_index = 0;
        self.time_in_hop = 0.0;
        self.phase_accumulator = 0.0;
    }
}

/// Waterfall encoding - maps data to visual frequency sweeps
pub struct WaterfallModulator {
    config: ModulationConfig,
    sweep_range: f64,
    sweep_duration: f64,
    phase_accumulator: f64,
}

impl WaterfallModulator {
    pub fn new(config: ModulationConfig, sweep_range: f64, sweep_duration: f64) -> Self {
        Self {
            config,
            sweep_range,
            sweep_duration,
            phase_accumulator: 0.0,
        }
    }
    
    /// Map byte value to frequency sweep pattern
    fn byte_to_sweep_pattern(&self, byte: u8) -> Vec<f64> {
        let base_freq = self.config.carrier_frequency;
        let mut pattern = Vec::new();
        
        // Create unique sweep pattern for each possible byte value
        let pattern_type = byte % 8;
        let amplitude_factor = ((byte >> 3) as f64 + 1.0) / 32.0; // 0.03125 to 1.0
        
        let steps = 32; // Number of frequency steps in sweep
        for i in 0..steps {
            let t = i as f64 / (steps - 1) as f64; // 0 to 1
            
            let freq_offset = match pattern_type {
                0 => amplitude_factor * self.sweep_range * (2.0 * t - 1.0), // Linear sweep
                1 => amplitude_factor * self.sweep_range * (2.0 * t - 1.0).powi(3), // Cubic sweep
                2 => amplitude_factor * self.sweep_range * (2.0 * PI * t).sin(), // Sine sweep
                3 => amplitude_factor * self.sweep_range * (4.0 * PI * t).sin(), // Double sine
                4 => amplitude_factor * self.sweep_range * if t < 0.5 { 2.0 * t - 1.0 } else { 3.0 - 2.0 * t }, // Triangle
                5 => amplitude_factor * self.sweep_range * (1.0 - 2.0 * t), // Reverse linear
                6 => amplitude_factor * self.sweep_range * ((6.28 * t).cos() - 1.0) / 2.0, // Cosine dip
                7 => amplitude_factor * self.sweep_range * if (t * 4.0) as i32 % 2 == 0 { 0.5 } else { -0.5 }, // Square
                _ => 0.0,
            };
            
            pattern.push(base_freq + freq_offset);
        }
        
        pattern
    }
}

impl Modulator for WaterfallModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let samples_per_step = (self.config.sample_rate * self.sweep_duration / 32.0) as usize;
        
        for &byte in bits {
            let sweep_pattern = self.byte_to_sweep_pattern(byte);
            
            for frequency in sweep_pattern {
                for _ in 0..samples_per_step {
                    let sample = Complex::new(
                        (2.0 * PI * frequency * self.phase_accumulator / self.config.sample_rate).cos(),
                        (2.0 * PI * frequency * self.phase_accumulator / self.config.sample_rate).sin(),
                    );
                    
                    output.push(sample);
                    
                    self.phase_accumulator += 1.0;
                    if self.phase_accumulator >= self.config.sample_rate {
                        self.phase_accumulator -= self.config.sample_rate;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        (self.config.sample_rate * self.sweep_duration) as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        1.0 / self.sweep_duration
    }
    
    fn reset(&mut self) {
        self.phase_accumulator = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_modulator() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let chaos_config = ChaosConfig::default();
        let mut modulator = ChaosModulator::new(config, chaos_config);
        
        let data = vec![0xAA, 0x55]; // Alternating pattern
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_rotating_constellation() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let mut modulator = RotatingConstellationModulator::new(config, 15.0);
        
        let data = vec![0x12, 0x34];
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_frequency_hopping() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let mut modulator = FrequencyHoppingModulator::with_default_hops(config);
        
        let data = vec![0xFF];
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_waterfall_modulator() {
        let config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let mut modulator = WaterfallModulator::new(config, 500.0, 0.1);
        
        let data = vec![0x00, 0x7F, 0xFF];
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        assert!(!output.is_empty());
        
        // Should generate different patterns for different bytes
        let pattern_length = modulator.samples_per_symbol();
        assert!(output.len() >= 3 * pattern_length);
    }
}
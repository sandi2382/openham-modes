//! FFT processing wrapper

use crate::{buffer::Complex, CoreError, Result};
use rustfft::{FftPlanner, num_complex::Complex64};
use std::sync::Arc;

/// FFT configuration
#[derive(Debug, Clone)]
pub struct FftConfig {
    pub size: usize,
    pub sample_rate: f64,
}

impl FftConfig {
    pub fn new(size: usize, sample_rate: f64) -> Result<Self> {
        if size == 0 || !size.is_power_of_two() {
            return Err(CoreError::FftError {
                msg: format!("FFT size must be a power of 2, got {}", size),
            });
        }
        
        if sample_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: sample_rate });
        }
        
        Ok(Self { size, sample_rate })
    }
    
    /// Get frequency resolution (Hz per bin)
    pub fn frequency_resolution(&self) -> f64 {
        self.sample_rate / self.size as f64
    }
    
    /// Convert bin index to frequency in Hz
    pub fn bin_to_frequency(&self, bin: usize) -> f64 {
        bin as f64 * self.frequency_resolution()
    }
    
    /// Convert frequency in Hz to bin index
    pub fn frequency_to_bin(&self, frequency: f64) -> usize {
        (frequency / self.frequency_resolution()).round() as usize
    }
}

/// FFT processor for signal analysis and processing
pub struct FftProcessor {
    config: FftConfig,
    fft: Arc<dyn rustfft::Fft<f64>>,
    ifft: Arc<dyn rustfft::Fft<f64>>,
    scratch: Vec<Complex64>,
}

impl FftProcessor {
    /// Create a new FFT processor
    pub fn new(config: FftConfig) -> Result<Self> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(config.size);
        let ifft = planner.plan_fft_inverse(config.size);
        let scratch = vec![Complex64::new(0.0, 0.0); config.size];
        
        Ok(Self {
            config,
            fft,
            ifft,
            scratch,
        })
    }
    
    /// Get the FFT configuration
    pub fn config(&self) -> &FftConfig {
        &self.config
    }
    
    /// Perform forward FFT
    pub fn fft(&mut self, input: &[Complex], output: &mut [Complex]) -> Result<()> {
        if input.len() != self.config.size || output.len() != self.config.size {
            return Err(CoreError::BufferSizeMismatch {
                expected: self.config.size,
                actual: if input.len() != self.config.size { input.len() } else { output.len() },
            });
        }
        
        // Convert to rustfft format
        let mut buffer: Vec<Complex64> = input.iter()
            .map(|c| Complex64::new(c.real, c.imag))
            .collect();
        
        // Perform FFT
        self.fft.process(&mut buffer);
        
        // Convert back to our format
        for (i, c) in buffer.iter().enumerate() {
            output[i] = Complex::new(c.re, c.im);
        }
        
        Ok(())
    }
    
    /// Perform inverse FFT
    pub fn ifft(&mut self, input: &[Complex], output: &mut [Complex]) -> Result<()> {
        if input.len() != self.config.size || output.len() != self.config.size {
            return Err(CoreError::BufferSizeMismatch {
                expected: self.config.size,
                actual: if input.len() != self.config.size { input.len() } else { output.len() },
            });
        }
        
        // Convert to rustfft format
        let mut buffer: Vec<Complex64> = input.iter()
            .map(|c| Complex64::new(c.real, c.imag))
            .collect();
        
        // Perform IFFT
        self.ifft.process(&mut buffer);
        
        // Convert back to our format and normalize
        let scale = 1.0 / self.config.size as f64;
        for (i, c) in buffer.iter().enumerate() {
            output[i] = Complex::new(c.re * scale, c.im * scale);
        }
        
        Ok(())
    }
    
    /// Compute power spectral density
    pub fn power_spectrum(&mut self, input: &[Complex], output: &mut [f64]) -> Result<()> {
        if input.len() != self.config.size {
            return Err(CoreError::BufferSizeMismatch {
                expected: self.config.size,
                actual: input.len(),
            });
        }
        
        if output.len() != self.config.size / 2 + 1 {
            return Err(CoreError::BufferSizeMismatch {
                expected: self.config.size / 2 + 1,
                actual: output.len(),
            });
        }
        
        let mut fft_output = vec![Complex::default(); self.config.size];
        self.fft(input, &mut fft_output)?;
        
        // Compute power for positive frequencies only
        for i in 0..output.len() {
            let magnitude = fft_output[i].magnitude();
            output[i] = magnitude * magnitude;
            
            // Scale appropriately (double for non-DC and non-Nyquist bins)
            if i > 0 && i < self.config.size / 2 {
                output[i] *= 2.0;
            }
        }
        
        Ok(())
    }
}

/// Windowing functions for FFT processing
pub mod window {
    /// Apply Hamming window to signal
    pub fn hamming(signal: &mut [f64]) {
        let n = signal.len();
        for (i, sample) in signal.iter_mut().enumerate() {
            let window_val = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos();
            *sample *= window_val;
        }
    }
    
    /// Apply Hanning window to signal
    pub fn hanning(signal: &mut [f64]) {
        let n = signal.len();
        for (i, sample) in signal.iter_mut().enumerate() {
            let window_val = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos());
            *sample *= window_val;
        }
    }
    
    /// Apply Blackman window to signal
    pub fn blackman(signal: &mut [f64]) {
        let n = signal.len();
        let a0 = 0.42;
        let a1 = 0.5;
        let a2 = 0.08;
        
        for (i, sample) in signal.iter_mut().enumerate() {
            let phase = 2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64;
            let window_val = a0 - a1 * phase.cos() + a2 * (2.0 * phase).cos();
            *sample *= window_val;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_config_creation() {
        let config = FftConfig::new(1024, 48000.0).unwrap();
        assert_eq!(config.size, 1024);
        assert_eq!(config.sample_rate, 48000.0);
        assert!((config.frequency_resolution() - 46.875).abs() < 1e-10);
    }

    #[test]
    fn test_fft_config_invalid_size() {
        let result = FftConfig::new(1000, 48000.0); // Not power of 2
        assert!(result.is_err());
    }

    #[test]
    fn test_fft_processor_creation() {
        let config = FftConfig::new(64, 1000.0).unwrap();
        let processor = FftProcessor::new(config).unwrap();
        assert_eq!(processor.config().size, 64);
    }

    #[test]
    fn test_fft_roundtrip() {
        let config = FftConfig::new(8, 1000.0).unwrap();
        let mut processor = FftProcessor::new(config).unwrap();
        
        // Create a simple test signal
        let input = vec![
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
        ];
        
        let mut fft_output = vec![Complex::default(); 8];
        let mut ifft_output = vec![Complex::default(); 8];
        
        processor.fft(&input, &mut fft_output).unwrap();
        processor.ifft(&fft_output, &mut ifft_output).unwrap();
        
        // Check that we get back the original signal (within tolerance)
        for (original, recovered) in input.iter().zip(ifft_output.iter()) {
            assert!((original.real - recovered.real).abs() < 1e-10);
            assert!((original.imag - recovered.imag).abs() < 1e-10);
        }
    }
}
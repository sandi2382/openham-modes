//! Sample rate conversion and resampling

use crate::{CoreError, Result};

/// Sample rate converter/resampler
pub struct Resampler {
    input_rate: f64,
    output_rate: f64,
    ratio: f64,
    buffer: Vec<f64>,
    phase: f64,
}

impl Resampler {
    /// Create a new resampler
    pub fn new(input_rate: f64, output_rate: f64) -> Result<Self> {
        if input_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: input_rate });
        }
        if output_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: output_rate });
        }
        
        let ratio = input_rate / output_rate;
        
        Ok(Self {
            input_rate,
            output_rate,
            ratio,
            buffer: Vec::new(),
            phase: 0.0,
        })
    }
    
    /// Get the input sample rate
    pub fn input_rate(&self) -> f64 {
        self.input_rate
    }
    
    /// Get the output sample rate
    pub fn output_rate(&self) -> f64 {
        self.output_rate
    }
    
    /// Get the resampling ratio
    pub fn ratio(&self) -> f64 {
        self.ratio
    }
    
    /// Process a buffer of samples with linear interpolation
    pub fn process(&mut self, input: &[f64], output: &mut Vec<f64>) -> Result<()> {
        output.clear();
        
        // Add new input samples to buffer
        self.buffer.extend_from_slice(input);
        
        // Generate output samples
        while self.phase < self.buffer.len() as f64 - 1.0 {
            let index = self.phase as usize;
            let frac = self.phase - index as f64;
            
            // Linear interpolation between samples
            let sample = if index + 1 < self.buffer.len() {
                self.buffer[index] * (1.0 - frac) + self.buffer[index + 1] * frac
            } else {
                self.buffer[index]
            };
            
            output.push(sample);
            self.phase += self.ratio;
        }
        
        // Remove consumed samples from buffer
        let consumed = self.phase as usize;
        if consumed > 0 {
            self.buffer.drain(0..consumed.min(self.buffer.len()));
            self.phase -= consumed as f64;
        }
        
        Ok(())
    }
    
    /// Reset the resampler state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.phase = 0.0;
    }
    
    /// Calculate the expected output length for a given input length
    pub fn output_length(&self, input_length: usize) -> usize {
        ((input_length as f64 / self.ratio).ceil() as usize).max(1)
    }
}

/// Rational resampler for exact integer ratios
pub struct RationalResampler {
    upsampling_factor: usize,
    downsampling_factor: usize,
    buffer: Vec<f64>,
    phase: usize,
}

impl RationalResampler {
    /// Create a new rational resampler
    pub fn new(upsampling_factor: usize, downsampling_factor: usize) -> Result<Self> {
        if upsampling_factor == 0 {
            return Err(CoreError::ResampleError {
                msg: "Upsampling factor must be greater than 0".to_string(),
            });
        }
        if downsampling_factor == 0 {
            return Err(CoreError::ResampleError {
                msg: "Downsampling factor must be greater than 0".to_string(),
            });
        }
        
        Ok(Self {
            upsampling_factor,
            downsampling_factor,
            buffer: Vec::new(),
            phase: 0,
        })
    }
    
    /// Create a resampler from sample rates (finds best rational approximation)
    pub fn from_rates(input_rate: f64, output_rate: f64, max_factor: usize) -> Result<Self> {
        if input_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: input_rate });
        }
        if output_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: output_rate });
        }
        
        let ratio = output_rate / input_rate;
        let (up, down) = rational_approximation(ratio, max_factor);
        
        Self::new(up, down)
    }
    
    /// Process samples with exact rational resampling
    pub fn process(&mut self, input: &[f64], output: &mut Vec<f64>) -> Result<()> {
        output.clear();
        
        for &sample in input {
            // Upsample by inserting zeros
            for i in 0..self.upsampling_factor {
                let upsampled = if i == 0 { sample } else { 0.0 };
                
                // Downsample by taking every Nth sample
                if self.phase == 0 {
                    output.push(upsampled);
                }
                
                self.phase = (self.phase + 1) % self.downsampling_factor;
            }
        }
        
        Ok(())
    }
    
    /// Reset the resampler state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.phase = 0;
    }
}

/// Find rational approximation using continued fractions
fn rational_approximation(x: f64, max_denominator: usize) -> (usize, usize) {
    if x <= 0.0 {
        return (0, 1);
    }
    
    let mut x = x;
    let mut a = x.floor() as usize;
    let mut p0 = 1;
    let mut q0 = 0;
    let mut p1 = a;
    let mut q1 = 1;
    
    while q1 <= max_denominator && (x - a as f64).abs() > 1e-15 {
        x = 1.0 / (x - a as f64);
        a = x.floor() as usize;
        
        let p2 = a * p1 + p0;
        let q2 = a * q1 + q0;
        
        if q2 > max_denominator {
            break;
        }
        
        p0 = p1;
        q0 = q1;
        p1 = p2;
        q1 = q2;
    }
    
    (p1, q1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_creation() {
        let resampler = Resampler::new(48000.0, 44100.0).unwrap();
        assert_eq!(resampler.input_rate(), 48000.0);
        assert_eq!(resampler.output_rate(), 44100.0);
        assert!((resampler.ratio() - 48000.0 / 44100.0).abs() < 1e-10);
    }

    #[test]
    fn test_rational_resampler_creation() {
        let resampler = RationalResampler::new(3, 2).unwrap();
        assert_eq!(resampler.upsampling_factor, 3);
        assert_eq!(resampler.downsampling_factor, 2);
    }

    #[test]
    fn test_rational_approximation() {
        let (num, den) = rational_approximation(1.5, 100);
        assert_eq!(num, 3);
        assert_eq!(den, 2);
        
        let (num, den) = rational_approximation(22050.0 / 48000.0, 1000);
        // Should find a reasonable approximation
        assert!(num > 0 && den > 0);
        assert!((num as f64 / den as f64 - 22050.0 / 48000.0).abs() < 0.01);
    }

    #[test]
    fn test_resampler_processing() {
        let mut resampler = Resampler::new(2000.0, 1000.0).unwrap(); // 2:1 downsampling
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let mut output = Vec::new();
        
        resampler.process(&input, &mut output).unwrap();
        
        // Should produce approximately half the samples
        assert!(!output.is_empty());
        assert!(output.len() <= input.len());
    }
}
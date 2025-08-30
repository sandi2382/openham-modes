//! Digital filter implementations

use crate::{CoreError, Result};

/// Generic filter trait
pub trait Filter<T: Copy> {
    /// Process a single sample
    fn process_sample(&mut self, input: T) -> T;
    
    /// Process a buffer of samples
    fn process_buffer(&mut self, input: &[T], output: &mut [T]) -> Result<()> {
        if input.len() != output.len() {
            return Err(CoreError::BufferSizeMismatch {
                expected: input.len(),
                actual: output.len(),
            });
        }
        
        for (i, sample) in input.iter().enumerate() {
            output[i] = self.process_sample(*sample);
        }
        
        Ok(())
    }
    
    /// Reset the filter state
    fn reset(&mut self);
}

/// Finite Impulse Response (FIR) filter
#[derive(Debug, Clone)]
pub struct FirFilter {
    coefficients: Vec<f64>,
    delay_line: Vec<f64>,
    index: usize,
}

impl FirFilter {
    /// Create a new FIR filter with given coefficients
    pub fn new(coefficients: Vec<f64>) -> Result<Self> {
        if coefficients.is_empty() {
            return Err(CoreError::InvalidFilterParameters {
                msg: "FIR filter must have at least one coefficient".to_string(),
            });
        }
        
        let delay_line = vec![0.0; coefficients.len()];
        
        Ok(Self {
            coefficients,
            delay_line,
            index: 0,
        })
    }
    
    /// Create a simple low-pass FIR filter
    pub fn lowpass(cutoff_freq: f64, sample_rate: f64, num_taps: usize) -> Result<Self> {
        if cutoff_freq <= 0.0 || cutoff_freq >= sample_rate / 2.0 {
            return Err(CoreError::InvalidFilterParameters {
                msg: format!("Invalid cutoff frequency: {}", cutoff_freq),
            });
        }
        
        if num_taps == 0 {
            return Err(CoreError::InvalidFilterParameters {
                msg: "Number of taps must be greater than 0".to_string(),
            });
        }
        
        // Simple windowed sinc filter design
        let mut coefficients = Vec::with_capacity(num_taps);
        let normalized_cutoff = 2.0 * cutoff_freq / sample_rate;
        let center = (num_taps - 1) as f64 / 2.0;
        
        for i in 0..num_taps {
            let n = i as f64 - center;
            let coeff = if n == 0.0 {
                normalized_cutoff
            } else {
                (std::f64::consts::PI * normalized_cutoff * n).sin() / (std::f64::consts::PI * n)
            };
            
            // Apply Hamming window
            let window = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (num_taps - 1) as f64).cos();
            coefficients.push(coeff * window);
        }
        
        Self::new(coefficients)
    }
}

impl Filter<f64> for FirFilter {
    fn process_sample(&mut self, input: f64) -> f64 {
        // Store input in delay line
        self.delay_line[self.index] = input;
        
        // Compute convolution
        let mut output = 0.0;
        for (i, &coeff) in self.coefficients.iter().enumerate() {
            let delay_index = (self.index + self.delay_line.len() - i) % self.delay_line.len();
            output += coeff * self.delay_line[delay_index];
        }
        
        // Update delay line index
        self.index = (self.index + 1) % self.delay_line.len();
        
        output
    }
    
    fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.index = 0;
    }
}

/// Infinite Impulse Response (IIR) filter
#[derive(Debug, Clone)]
pub struct IirFilter {
    b_coeffs: Vec<f64>, // Feedforward coefficients
    a_coeffs: Vec<f64>, // Feedback coefficients
    x_history: Vec<f64>, // Input history
    y_history: Vec<f64>, // Output history
}

impl IirFilter {
    /// Create a new IIR filter with given coefficients
    pub fn new(b_coeffs: Vec<f64>, a_coeffs: Vec<f64>) -> Result<Self> {
        if b_coeffs.is_empty() || a_coeffs.is_empty() {
            return Err(CoreError::InvalidFilterParameters {
                msg: "IIR filter must have at least one coefficient in each array".to_string(),
            });
        }
        
        if a_coeffs[0] == 0.0 {
            return Err(CoreError::InvalidFilterParameters {
                msg: "First feedback coefficient (a[0]) cannot be zero".to_string(),
            });
        }
        
        let x_history = vec![0.0; b_coeffs.len()];
        let y_history = vec![0.0; a_coeffs.len()];
        
        Ok(Self {
            b_coeffs,
            a_coeffs,
            x_history,
            y_history,
        })
    }
    
    /// Create a simple first-order low-pass IIR filter
    pub fn lowpass_1st_order(cutoff_freq: f64, sample_rate: f64) -> Result<Self> {
        if cutoff_freq <= 0.0 || cutoff_freq >= sample_rate / 2.0 {
            return Err(CoreError::InvalidFilterParameters {
                msg: format!("Invalid cutoff frequency: {}", cutoff_freq),
            });
        }
        
        let rc = 1.0 / (2.0 * std::f64::consts::PI * cutoff_freq);
        let dt = 1.0 / sample_rate;
        let alpha = dt / (rc + dt);
        
        let b_coeffs = vec![alpha];
        let a_coeffs = vec![1.0, -(1.0 - alpha)];
        
        Self::new(b_coeffs, a_coeffs)
    }
}

impl Filter<f64> for IirFilter {
    fn process_sample(&mut self, input: f64) -> f64 {
        // Shift input history
        for i in (1..self.x_history.len()).rev() {
            self.x_history[i] = self.x_history[i - 1];
        }
        self.x_history[0] = input;
        
        // Compute output
        let mut output = 0.0;
        
        // Feedforward terms
        for (i, &coeff) in self.b_coeffs.iter().enumerate() {
            if i < self.x_history.len() {
                output += coeff * self.x_history[i];
            }
        }
        
        // Feedback terms (skip a[0])
        for (i, &coeff) in self.a_coeffs.iter().skip(1).enumerate() {
            if i < self.y_history.len() - 1 {
                output -= coeff * self.y_history[i];
            }
        }
        
        // Normalize by a[0]
        output /= self.a_coeffs[0];
        
        // Shift output history
        for i in (1..self.y_history.len()).rev() {
            self.y_history[i] = self.y_history[i - 1];
        }
        self.y_history[0] = output;
        
        output
    }
    
    fn reset(&mut self) {
        self.x_history.fill(0.0);
        self.y_history.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fir_filter_creation() {
        let coeffs = vec![0.1, 0.2, 0.4, 0.2, 0.1];
        let filter = FirFilter::new(coeffs).unwrap();
        assert_eq!(filter.coefficients.len(), 5);
    }

    #[test]
    fn test_fir_lowpass_creation() {
        let filter = FirFilter::lowpass(1000.0, 48000.0, 51).unwrap();
        assert_eq!(filter.coefficients.len(), 51);
    }

    #[test]
    fn test_iir_filter_creation() {
        let b_coeffs = vec![0.5];
        let a_coeffs = vec![1.0, -0.5];
        let filter = IirFilter::new(b_coeffs, a_coeffs).unwrap();
        assert_eq!(filter.b_coeffs.len(), 1);
        assert_eq!(filter.a_coeffs.len(), 2);
    }

    #[test]
    fn test_filter_processing() {
        let mut filter = FirFilter::new(vec![0.5, 0.5]).unwrap();
        
        // Test impulse response
        let output1 = filter.process_sample(1.0);
        let output2 = filter.process_sample(0.0);
        let output3 = filter.process_sample(0.0);
        
        assert_eq!(output1, 0.5);
        assert_eq!(output2, 0.5);
        assert_eq!(output3, 0.0);
    }
}
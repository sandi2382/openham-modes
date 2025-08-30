//! Sample buffer management and operations

use crate::{CoreError, Result};
use std::ops::{Index, IndexMut};

/// Generic sample buffer for audio data
#[derive(Debug, Clone)]
pub struct SampleBuffer<T> {
    data: Vec<T>,
    sample_rate: f64,
}

impl<T> SampleBuffer<T>
where
    T: Clone + Default,
{
    /// Create a new sample buffer with specified capacity and sample rate
    pub fn new(capacity: usize, sample_rate: f64) -> Result<Self> {
        if sample_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: sample_rate });
        }
        
        Ok(Self {
            data: vec![T::default(); capacity],
            sample_rate,
        })
    }
    
    /// Create a buffer from existing data
    pub fn from_data(data: Vec<T>, sample_rate: f64) -> Result<Self> {
        if sample_rate <= 0.0 {
            return Err(CoreError::InvalidSampleRate { rate: sample_rate });
        }
        
        Ok(Self { data, sample_rate })
    }
    
    /// Get the sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
    
    /// Get the number of samples
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Get a reference to the underlying data
    pub fn data(&self) -> &[T] {
        &self.data
    }
    
    /// Get a mutable reference to the underlying data
    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
    }
    
    /// Resize the buffer
    pub fn resize(&mut self, new_len: usize) {
        self.data.resize(new_len, T::default());
    }
}

impl<T> Index<usize> for SampleBuffer<T> {
    type Output = T;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> IndexMut<usize> for SampleBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

/// Complex number representation for IQ data
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

impl Complex {
    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }
    
    pub fn magnitude(&self) -> f64 {
        (self.real * self.real + self.imag * self.imag).sqrt()
    }
    
    pub fn norm(&self) -> f64 {
        self.magnitude()
    }
    
    pub fn norm_sqr(&self) -> f64 {
        self.real * self.real + self.imag * self.imag
    }
    
    pub fn phase(&self) -> f64 {
        self.imag.atan2(self.real)
    }
}

impl std::ops::Mul<f64> for Complex {
    type Output = Complex;
    
    fn mul(self, rhs: f64) -> Self::Output {
        Complex::new(self.real * rhs, self.imag * rhs)
    }
}

/// Type alias for complex sample buffers (IQ data)
pub type ComplexBuffer = SampleBuffer<Complex>;

/// Type alias for real sample buffers (audio data)
pub type AudioBuffer = SampleBuffer<f64>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_buffer_creation() {
        let buffer = SampleBuffer::<f64>::new(1024, 48000.0).unwrap();
        assert_eq!(buffer.len(), 1024);
        assert_eq!(buffer.sample_rate(), 48000.0);
    }

    #[test]
    fn test_invalid_sample_rate() {
        let result = SampleBuffer::<f64>::new(1024, -1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_operations() {
        let c = Complex::new(3.0, 4.0);
        assert_eq!(c.magnitude(), 5.0);
        assert!((c.phase() - 0.9272952180016122).abs() < 1e-10);
    }
}
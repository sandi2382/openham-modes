//! Error types for OpenHam Core

use thiserror::Error;

/// Core error types
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid sample rate: {rate}")]
    InvalidSampleRate { rate: f64 },
    
    #[error("Buffer size mismatch: expected {expected}, got {actual}")]
    BufferSizeMismatch { expected: usize, actual: usize },
    
    #[error("Invalid filter parameters: {msg}")]
    InvalidFilterParameters { msg: String },
    
    #[error("FFT error: {msg}")]
    FftError { msg: String },
    
    #[error("Resampling error: {msg}")]
    ResampleError { msg: String },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for OpenHam Core operations
pub type Result<T> = std::result::Result<T, CoreError>;
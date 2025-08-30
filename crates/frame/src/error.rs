//! Error types for OpenHam Frame

use thiserror::Error;

/// Frame processing error types
#[derive(Error, Debug)]
pub enum FrameError {
    #[error("Invalid frame format: {msg}")]
    InvalidFormat { msg: String },
    
    #[error("Frame size mismatch: expected {expected}, got {actual}")]
    SizeMismatch { expected: usize, actual: usize },
    
    #[error("FEC decoding failed: {msg}")]
    FecDecodingFailed { msg: String },
    
    #[error("Invalid FEC parameters: {msg}")]
    InvalidFecParameters { msg: String },
    
    #[error("Interleaving error: {msg}")]
    InterleavingError { msg: String },
    
    #[error("Core error: {0}")]
    Core(#[from] openham_core::CoreError),
}

/// Result type for OpenHam Frame operations
pub type Result<T> = std::result::Result<T, FrameError>;
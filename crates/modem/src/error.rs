//! Error types for OpenHam Modem

use thiserror::Error;

/// Modem error types
#[derive(Error, Debug)]
pub enum ModemError {
    #[error("Unsupported modulation: {name}")]
    UnsupportedModulation { name: String },
    
    #[error("Modulation failed: {msg}")]
    ModulationFailed { msg: String },
    
    #[error("Demodulation failed: {msg}")]
    DemodulationFailed { msg: String },
    
    #[error("Invalid modulation parameters: {msg}")]
    InvalidParameters { msg: String },
    
    #[error("Synchronization failed: {msg}")]
    SynchronizationFailed { msg: String },
    
    #[error("Core error: {0}")]
    Core(#[from] openham_core::CoreError),
}

/// Result type for OpenHam Modem operations
pub type Result<T> = std::result::Result<T, ModemError>;
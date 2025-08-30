//! Error types for OpenHam Codecs

use thiserror::Error;

/// Codec error types
#[derive(Error, Debug)]
pub enum CodecError {
    #[error("Unsupported codec: {name}")]
    UnsupportedCodec { name: String },
    
    #[error("Encoding failed: {msg}")]
    EncodingFailed { msg: String },
    
    #[error("Decoding failed: {msg}")]
    DecodingFailed { msg: String },
    
    #[error("Invalid codec parameters: {msg}")]
    InvalidParameters { msg: String },
    
    #[error("Codec not initialized")]
    NotInitialized,
    
    #[error("Frame error: {0}")]
    Frame(#[from] openham_frame::FrameError),
    
    #[error("Core error: {0}")]
    Core(#[from] openham_core::CoreError),
}

/// Result type for OpenHam Codec operations
pub type Result<T> = std::result::Result<T, CodecError>;
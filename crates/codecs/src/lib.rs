//! OpenHam Codecs - Text and voice codecs for digital modes
//!
//! This crate provides encoding and decoding for various data types
//! used in OpenHam digital modes.

pub mod text;
pub mod voice;
pub mod registry;
pub mod error;

pub use error::{CodecError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        text::{TextCodec, HuffmanCodec},
        voice::{VoiceCodec, OpusCodec},
        registry::{CodecRegistry, CodecInfo},
        error::{CodecError, Result},
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
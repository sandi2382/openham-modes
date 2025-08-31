//! OpenHam Codecs - Text and voice codecs for digital modes
//!
//! This crate provides encoding and decoding for various data types
//! used in OpenHam digital modes.

pub mod text;
pub mod voice;
pub mod registry;
pub mod cw;
pub mod voice_announce;
pub mod audio_utils;
pub mod transmission_announce;
pub mod error;

pub use error::{CodecError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        text::{TextCodec, HuffmanCodec},
        voice::{VoiceCodec, OpusCodec},
        cw::{CwGenerator, CwConfig, MorseElement},
        voice_announce::VoiceAnnouncer,
        audio_utils::{AudioWriter, AudioFormat, AudioFormatInfo},
        transmission_announce::TransmissionAnnouncer,
        registry::{CodecRegistry, CodecInfo},
        error::{CodecError, Result},
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
//! OpenHam Frame - Framing, FEC, and error correction
//!
//! This crate provides framing protocols, forward error correction (FEC),
//! and interleaving for reliable digital communications.

pub mod frame;
pub mod fec;
pub mod interleave;
pub mod multimedia;
pub mod error;

pub use error::{FrameError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        frame::{Frame, FrameBuilder, FrameHeader},
        fec::{FecEncoder, FecDecoder, ReedSolomon, Convolutional},
        interleave::{Interleaver, BlockInterleaver, ConvolutionalInterleaver},
        multimedia::{
            MultimediaHeader, MultimediaFrame, MediaType, CompressionType,
            FrameSplitter, FrameAssembler, TransmissionFrame,
        },
        error::{FrameError, Result},
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
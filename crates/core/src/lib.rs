//! OpenHam Core - DSP primitives and signal processing
//!
//! This crate provides fundamental DSP operations, sample buffers,
//! resampling, filtering, and FFT wrappers for OpenHam digital modes.

pub mod buffer;
pub mod filter;
pub mod fft;
pub mod resample;
pub mod error;

pub use error::{CoreError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        buffer::{SampleBuffer, ComplexBuffer},
        filter::{Filter, FirFilter, IirFilter},
        fft::{FftProcessor, FftConfig},
        resample::Resampler,
        error::{CoreError, Result},
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
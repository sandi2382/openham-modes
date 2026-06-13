//! OpenHam Core - DSP primitives and signal processing
//!
//! This crate provides fundamental DSP operations, sample buffers,
//! resampling, filtering, and FFT wrappers for OpenHam digital modes.

pub mod buffer;
pub mod channel;
pub mod filter;
pub mod fft;
pub mod metrics;
pub mod resample;
pub mod error;

pub use error::{CoreError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        buffer::{SampleBuffer, ComplexBuffer},
        channel::{
            add_awgn_ebn0, add_awgn_real_snr, add_awgn_snr, apply_freq_offset, apply_multipath,
            apply_phase_offset, awgn_at_snr, measure_snr_db, signal_power, Awgn, Channel,
            ChannelChain, FreqOffset, Multipath, MultipathTap, PhaseOffset,
        },
        filter::{Filter, FirFilter, IirFilter},
        fft::{FftProcessor, FftConfig},
        metrics::{
            bpsk_ber_theory, count_bit_errors, ebn0_sweep, ebn0_sweep_to_csv, is_frame_error,
            snr_sweep, sweep_to_csv, BitErrors, Ebn0Point, SweepPoint,
        },
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
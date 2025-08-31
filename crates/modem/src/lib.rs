//! OpenHam Modem - Modulation and demodulation primitives
//!
//! This crate provides digital modulation and demodulation functions
//! for various schemes used in amateur radio digital modes.

pub mod bpsk;
pub mod fsk;
pub mod afsk;
pub mod psk;
pub mod qam;
pub mod ofdm;
pub mod experimental;
pub mod common;
pub mod error;

pub use error::{ModemError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        bpsk::{BpskModulator, BpskDemodulator},
        fsk::{FskModulator, FskDemodulator},
        afsk::{AfskModulator, AfskDemodulator},
        psk::{PskModulator, PskDemodulator, PskConfig},
        qam::{QamModulator, QamDemodulator, QamConfig},
        ofdm::{OfdmModulator, OfdmDemodulator, OfdmConfig},
        experimental::{
            ChaosModulator, RotatingConstellationModulator,
            FrequencyHoppingModulator, WaterfallModulator,
            MultiToneConfig, ChaosConfig,
        },
        common::{Modulator, Demodulator, ModulationConfig},
        error::{ModemError, Result},
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
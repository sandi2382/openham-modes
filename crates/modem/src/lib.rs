//! OpenHam Modem - Modulation and demodulation primitives
//!
//! This crate provides digital modulation and demodulation functions
//! for various schemes used in amateur radio digital modes.

pub mod bpsk;
pub mod fsk;
pub mod ofdm;
pub mod common;
pub mod error;

pub use error::{ModemError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        bpsk::{BpskModulator, BpskDemodulator},
        fsk::{FskModulator, FskDemodulator},
        ofdm::{OfdmModulator, OfdmDemodulator},
        common::{Modulator, Demodulator, ModulationConfig},
        error::{ModemError, Result},
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
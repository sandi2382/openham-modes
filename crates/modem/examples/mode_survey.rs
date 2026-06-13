//! Mode survey: run every modulation through a clean round-trip and an
//! Eb/N0 sweep, at the raw modem layer (no codec, no outer framing), to find
//! which modes actually work vs. which are broken or noise-sensitive.
//!
//! Run with:  cargo run -p openham-modem --example mode_survey
//!
//! This is a diagnostic tool, not a pass/fail test. It prints a punch-list.

use openham_core::buffer::Complex;
use openham_core::metrics::{count_bit_errors, ebn0_sweep};
use openham_modem::afsk::{AfskConfig, AfskDemodulator, AfskModulator};
use openham_modem::bpsk::{BpskDemodulator, BpskModulator};
use openham_modem::common::{Demodulator, ModulationConfig, Modulator};
use openham_modem::fsk::{FskDemodulator, FskModulator};
use openham_modem::ofdm::{OfdmConfig, OfdmDemodulator, OfdmModulator};
use openham_modem::psk::{PskConfig, PskDemodulator, PskModulator};
use openham_modem::qam::{QamConfig, QamDemodulator, QamModulator};

type MakeMod = Box<dyn Fn() -> Box<dyn Modulator>>;
type MakeDemod = Box<dyn Fn() -> Box<dyn Demodulator>>;

struct Mode {
    name: &'static str,
    make_mod: MakeMod,
    make_demod: MakeDemod,
    k: f64,   // bits per symbol (for Eb/N0 labeling)
    sps: f64, // samples per symbol
}

/// Standard config: 48 kHz, given baud, 1500 Hz carrier.
fn cfg(baud: f64) -> ModulationConfig {
    ModulationConfig::new(48_000.0, baud, 1_500.0).unwrap()
}

fn modes() -> Vec<Mode> {
    let sps = 48_000.0 / 1_200.0; // 40 for the 1200-baud modes
    vec![
        Mode {
            name: "bpsk",
            make_mod: Box::new(|| Box::new(BpskModulator::new(cfg(1_200.0)).unwrap())),
            make_demod: Box::new(|| Box::new(BpskDemodulator::new(cfg(1_200.0)).unwrap())),
            k: 1.0,
            sps,
        },
        Mode {
            name: "fsk@1200",
            make_mod: Box::new(|| Box::new(FskModulator::new(cfg(1_200.0)).unwrap())),
            make_demod: Box::new(|| Box::new(FskDemodulator::new(cfg(1_200.0)).unwrap())),
            k: 1.0,
            sps,
        },
        Mode {
            name: "fsk@250",
            make_mod: Box::new(|| Box::new(FskModulator::new(cfg(250.0)).unwrap())),
            make_demod: Box::new(|| Box::new(FskDemodulator::new(cfg(250.0)).unwrap())),
            k: 1.0,
            sps: 48_000.0 / 250.0,
        },
        Mode {
            name: "fsk@125",
            make_mod: Box::new(|| Box::new(FskModulator::new(cfg(125.0)).unwrap())),
            make_demod: Box::new(|| Box::new(FskDemodulator::new(cfg(125.0)).unwrap())),
            k: 1.0,
            sps: 48_000.0 / 125.0,
        },
        Mode {
            name: "afsk",
            // AFSK Bell-202 is 1200 baud; match the modem config baud to it.
            make_mod: Box::new(|| {
                Box::new(AfskModulator::new(cfg(1_200.0), AfskConfig::bell_202()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(AfskDemodulator::new(cfg(1_200.0), AfskConfig::bell_202()).unwrap())
            }),
            k: 1.0,
            sps,
        },
        Mode {
            name: "psk4(qpsk)",
            make_mod: Box::new(|| {
                Box::new(PskModulator::new(cfg(1_200.0), PskConfig::qpsk()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(PskDemodulator::new(cfg(1_200.0), PskConfig::qpsk()).unwrap())
            }),
            k: 2.0,
            sps,
        },
        Mode {
            name: "psk8",
            make_mod: Box::new(|| {
                Box::new(PskModulator::new(cfg(1_200.0), PskConfig::psk8()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(PskDemodulator::new(cfg(1_200.0), PskConfig::psk8()).unwrap())
            }),
            k: 3.0,
            sps,
        },
        Mode {
            name: "qam16",
            make_mod: Box::new(|| {
                Box::new(QamModulator::new(cfg(1_200.0), QamConfig::qam16()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(QamDemodulator::new(cfg(1_200.0), QamConfig::qam16()).unwrap())
            }),
            k: 4.0,
            sps,
        },
        Mode {
            name: "qam64",
            make_mod: Box::new(|| {
                Box::new(QamModulator::new(cfg(1_200.0), QamConfig::qam64()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(QamDemodulator::new(cfg(1_200.0), QamConfig::qam64()).unwrap())
            }),
            k: 6.0,
            sps,
        },
        Mode {
            name: "ofdm64",
            make_mod: Box::new(|| {
                Box::new(OfdmModulator::new(cfg(1_200.0), OfdmConfig::amateur_radio_64()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(OfdmDemodulator::new(cfg(1_200.0), OfdmConfig::amateur_radio_64()).unwrap())
            }),
            // OFDM Eb/N0 labeling is approximate (multi-carrier); curve still informative.
            k: 1.0,
            sps,
        },
    ]
}

fn main() {
    let payload = b"OPENHAM MODE SURVEY 0123456789 de S56SPZ";

    println!("Raw-modem survey (no codec / no outer frame), payload = {} bytes\n", payload.len());
    println!(
        "{:<12} {:>10} {:>10}   {:<28} {:>10} {:>10} {:>10}",
        "mode", "clean_ber", "rx/tx", "clean status", "ber@2dB", "ber@6dB", "ber@10dB"
    );
    println!("{}", "-".repeat(100));

    for m in modes() {
        // 1. Modulate.
        let clean = {
            let mut modu = (m.make_mod)();
            let mut out = Vec::new();
            match modu.modulate(payload, &mut out) {
                Ok(()) => out,
                Err(e) => {
                    println!("{:<12} {:>10} {:>10}   MODULATE ERROR: {e}", m.name, "-", "-");
                    continue;
                }
            }
        };
        if clean.is_empty() {
            println!("{:<12} {:>10} {:>10}   modulator produced 0 samples", m.name, "-", "-");
            continue;
        }

        // 2. Clean round-trip.
        let rx_clean = {
            let mut d = (m.make_demod)();
            let mut out = Vec::new();
            let _ = d.demodulate(&clean, &mut out);
            out
        };
        let clean_ber = count_bit_errors(payload, &rx_clean).ber();
        let status = if clean_ber == 0.0 {
            "OK (exact)".to_string()
        } else if rx_clean.is_empty() {
            "BROKEN (no frame recovered)".to_string()
        } else if clean_ber < 0.45 {
            format!("partial ({} corrupt bits)", count_bit_errors(payload, &rx_clean).errors)
        } else {
            "BROKEN (garbage out)".to_string()
        };

        // 3. Eb/N0 sweep only if the clean channel is at least partially working.
        let (b2, b6, b10) = if clean_ber < 0.45 {
            let pts = ebn0_sweep(
                payload,
                &clean,
                &[2.0, 6.0, 10.0],
                m.k,
                m.sps,
                40,
                2024,
                |s| {
                    let mut d = (m.make_demod)();
                    let mut out = Vec::new();
                    let _ = d.demodulate(s, &mut out);
                    out
                },
            );
            (
                format!("{:.2e}", pts[0].ber),
                format!("{:.2e}", pts[1].ber),
                format!("{:.2e}", pts[2].ber),
            )
        } else {
            ("-".into(), "-".into(), "-".into())
        };

        println!(
            "{:<12} {:>10.4} {:>10}   {:<28} {:>10} {:>10} {:>10}",
            m.name,
            clean_ber,
            format!("{}/{}", rx_clean.len(), payload.len()),
            status,
            b2,
            b6,
            b10
        );
    }
    println!("\n(experimental modes chaos/freq-hop/rotating/waterfall not surveyed — demods pending)");
}

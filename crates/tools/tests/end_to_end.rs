//! End-to-end regression tests for the modes that currently round-trip.
//!
//! Mirrors the CLI tx/rx pipeline at the frame level: build a real `Frame`,
//! modulate it, emulate the mono-WAV channel (only the real part survives),
//! optionally add AWGN, demodulate, and parse the `Frame` back. This locks in
//! the working modes (bpsk, fsk, afsk, psk4, ofdm64) so they cannot silently
//! regress, and prints a FER-vs-SNR curve for each.

use openham_core::buffer::Complex;
use openham_core::channel::add_awgn_real_snr;
use openham_frame::frame::{frame_flags, frame_types, Frame};
use openham_frame::framing::{add_preamble_sync, Acquisition};
use openham_modem::afsk::{AfskConfig, AfskDemodulator, AfskModulator};
use openham_modem::bpsk::{BpskDemodulator, BpskModulator};
use openham_modem::common::{BitDemodulator, Demodulator, ModulationConfig, Modulator};
use openham_modem::fsk::{FskDemodulator, FskModulator};
use openham_modem::ofdm::{OfdmConfig, OfdmDemodulator, OfdmModulator};
use openham_modem::psk::{PskConfig, PskDemodulator, PskModulator};
use rand::rngs::StdRng;
use rand::SeedableRng;

type MakeMod = Box<dyn Fn() -> Box<dyn Modulator>>;
type MakeDemod = Box<dyn Fn() -> Box<dyn Demodulator>>;

struct Mode {
    name: &'static str,
    make_mod: MakeMod,
    make_demod: MakeDemod,
}

/// Same defaults the CLI uses: 48 kHz, 125 baud, 1500 Hz carrier.
fn cfg() -> ModulationConfig {
    ModulationConfig::new(48_000.0, 125.0, 1_500.0).unwrap()
}

fn working_modes() -> Vec<Mode> {
    vec![
        Mode {
            name: "bpsk",
            make_mod: Box::new(|| Box::new(BpskModulator::new(cfg()).unwrap())),
            make_demod: Box::new(|| Box::new(BpskDemodulator::new(cfg()).unwrap())),
        },
        Mode {
            name: "fsk",
            make_mod: Box::new(|| Box::new(FskModulator::new(cfg()).unwrap())),
            make_demod: Box::new(|| Box::new(FskDemodulator::new(cfg()).unwrap())),
        },
        Mode {
            name: "afsk",
            make_mod: Box::new(|| Box::new(AfskModulator::new(cfg(), AfskConfig::bell_202()).unwrap())),
            make_demod: Box::new(|| {
                Box::new(AfskDemodulator::new(cfg(), AfskConfig::bell_202()).unwrap())
            }),
        },
        Mode {
            name: "psk4",
            make_mod: Box::new(|| Box::new(PskModulator::new(cfg(), PskConfig::qpsk()).unwrap())),
            make_demod: Box::new(|| Box::new(PskDemodulator::new(cfg(), PskConfig::qpsk()).unwrap())),
        },
        Mode {
            name: "ofdm64",
            make_mod: Box::new(|| {
                Box::new(OfdmModulator::new(cfg(), OfdmConfig::amateur_radio_64()).unwrap())
            }),
            make_demod: Box::new(|| {
                Box::new(OfdmDemodulator::new(cfg(), OfdmConfig::amateur_radio_64()).unwrap())
            }),
        },
    ]
}

/// Emulate the mono-WAV channel: only the real part is transmitted, and the
/// receiver reads it back with a zero imaginary component.
fn through_wav(samples: &[Complex]) -> Vec<Complex> {
    samples.iter().map(|s| Complex::new(s.real, 0.0)).collect()
}

fn modulate(m: &Mode, bytes: &[u8]) -> Vec<Complex> {
    let mut modu = (m.make_mod)();
    let mut out = Vec::new();
    modu.modulate(bytes, &mut out).unwrap();
    out
}

/// Demodulate `samples` and report whether the frame parses back to `expect`.
fn decodes_to(m: &Mode, samples: &[Complex], expect: &[u8]) -> bool {
    let mut d = (m.make_demod)();
    let mut out = Vec::new();
    let _ = d.demodulate(samples, &mut out);
    matches!(Frame::from_bytes(&out), Ok(f) if f.payload == expect)
}

/// Always-run hard regression: every "working" mode must round-trip a frame
/// exactly on a clean channel. A short payload keeps this fast.
#[test]
fn working_modes_clean_roundtrip() {
    let payload = b"OHM E2E 42";
    for m in working_modes() {
        let frame = Frame::new(frame_types::DATA, 7, payload.to_vec(), frame_flags::NONE);
        let samples = through_wav(&modulate(&m, &frame.to_bytes()));
        assert!(
            decodes_to(&m, &samples, payload),
            "{} clean end-to-end round-trip regressed",
            m.name
        );
    }
}

/// Opt-in measurement (`cargo test -- --ignored --nocapture`): prints the
/// frame-error-rate-vs-SNR curve for each mode. bpsk/fsk are robust enough to
/// assert on; afsk/psk4/ofdm64 are printed for tracking (psk4 in particular is
/// fragile under even mild noise — a phase-ambiguity instability still to fix).
#[test]
#[ignore = "slow noise sweep; run on demand for FER curves"]
fn working_modes_fer_vs_snr() {
    let payload = b"OPENHAM E2E NOISE 0123456789";
    let snrs = [25.0, 15.0, 5.0];
    let trials = 8;

    println!("Frame error rate vs SNR(dB) {snrs:?}:");
    for m in working_modes() {
        let frame = Frame::new(frame_types::DATA, 1, payload.to_vec(), frame_flags::NONE);
        let clean = through_wav(&modulate(&m, &frame.to_bytes()));
        let mut rng = StdRng::seed_from_u64(2024);

        let mut fers = Vec::new();
        for &snr in &snrs {
            let mut fails = 0;
            for _ in 0..trials {
                let mut noisy = clean.clone();
                add_awgn_real_snr(&mut noisy, snr, &mut rng);
                if !decodes_to(&m, &noisy, payload) {
                    fails += 1;
                }
            }
            fers.push(fails as f64 / trials as f64);
        }
        println!("  {:<8} {:?}", m.name, fers);

        // The robust modes must stay error-free across the whole range.
        if matches!(m.name, "bpsk" | "fsk") {
            assert!(
                fers.iter().all(|&f| f == 0.0),
                "{} regressed under noise: {fers:?}",
                m.name
            );
        }
    }
}

type MakeBitDemod = Box<dyn Fn() -> Box<dyn BitDemodulator>>;

/// Modes wired into the frame-acquisition layer (those implementing
/// `BitDemodulator`). Extend as more modes gain reliable bit recovery.
fn acquisition_modes() -> Vec<(&'static str, MakeMod, MakeBitDemod)> {
    vec![
        (
            "bpsk",
            Box::new(|| Box::new(BpskModulator::new(cfg()).unwrap()) as Box<dyn Modulator>),
            Box::new(|| Box::new(BpskDemodulator::new(cfg()).unwrap()) as Box<dyn BitDemodulator>),
        ),
        (
            "fsk",
            Box::new(|| Box::new(FskModulator::new(cfg()).unwrap()) as Box<dyn Modulator>),
            Box::new(|| Box::new(FskDemodulator::new(cfg()).unwrap()) as Box<dyn BitDemodulator>),
        ),
    ]
}

/// Live-operation acceptance test: a transmission begins at an arbitrary instant
/// in a continuous, noisy feed (silence before and after). The receiver must
/// still locate and decode the frame via preamble + sync-word acquisition — it
/// does NOT get a buffer that starts exactly at the frame.
#[test]
fn frame_acquisition_at_arbitrary_offset() {
    let payload = b"OHM LIVE 99";
    let sps = 384usize; // 48000 / 125 baud
    for (name, make_mod, make_bitdemod) in acquisition_modes() {
        let frame = Frame::new(frame_types::DATA, 3, payload.to_vec(), frame_flags::NONE);

        // Transmit: preamble + sync word + frame, then modulate to audio.
        let framed = add_preamble_sync(&frame.to_bytes());
        let mut modu = make_mod();
        let mut signal = Vec::new();
        modu.modulate(&framed, &mut signal).unwrap();

        // Build a live-like capture: a deliberately non-symbol-aligned lead-in of
        // dead air, the burst, then trailing dead air, all under AWGN.
        let lead_in = sps + sps / 3 + 7;
        let mut stream = vec![Complex::new(0.0, 0.0); lead_in];
        stream.extend_from_slice(&signal);
        stream.extend(std::iter::repeat(Complex::new(0.0, 0.0)).take(sps * 2));
        let mut rng = StdRng::seed_from_u64(1);
        add_awgn_real_snr(&mut stream, 25.0, &mut rng);
        let stream = through_wav(&stream);

        // Receive: demodulate to a bit stream, then acquire frames from it.
        let mut bitdemod = make_bitdemod();
        let mut bits = Vec::new();
        bitdemod.demodulate_bits(&stream, &mut bits).unwrap();
        let frames = Acquisition::new().find_frames(&bits);

        assert!(
            frames.iter().any(|f| f.payload == payload),
            "{name} could not acquire a frame at an arbitrary offset in a live stream",
        );
    }
}

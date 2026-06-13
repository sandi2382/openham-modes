//! Integration test: a real BER/FER-vs-Eb/N0 curve for an actual modem, driven
//! through the `openham_core` AWGN harness. This is the end-to-end reliability
//! measurement the project was missing — modulate -> Eb/N0-calibrated AWGN ->
//! demodulate -> count errors — and it prints the measured curve next to the
//! theoretical BPSK reference.

use openham_core::buffer::Complex;
use openham_core::metrics::{bpsk_ber_theory, count_bit_errors, ebn0_sweep, ebn0_sweep_to_csv};
use openham_modem::bpsk::{BpskDemodulator, BpskModulator};
use openham_modem::common::{Demodulator, ModulationConfig, Modulator};

fn make_config() -> ModulationConfig {
    // 48 kHz, 1200 baud, 1500 Hz carrier => 40 samples/symbol.
    ModulationConfig::new(48_000.0, 1_200.0, 1_500.0).unwrap()
}

fn modulate_bpsk(payload: &[u8]) -> Vec<Complex> {
    let mut m = BpskModulator::new(make_config()).unwrap();
    let mut out = Vec::new();
    m.modulate(payload, &mut out).unwrap();
    out
}

fn demodulate_bpsk(samples: &[Complex]) -> Vec<u8> {
    // Fresh demodulator per call so each trial is independent of demod state.
    let mut d = BpskDemodulator::new(make_config()).unwrap();
    let mut out = Vec::new();
    let _ = d.demodulate(samples, &mut out);
    out
}

#[test]
fn bpsk_ber_vs_ebn0_curve() {
    let payload = b"OPENHAM BPSK TEST 0123456789";
    let clean = modulate_bpsk(payload);
    assert!(!clean.is_empty(), "modulator produced no samples");

    // Noise-free floor.
    let rx_clean = demodulate_bpsk(&clean);
    let clean_ber = count_bit_errors(payload, &rx_clean).ber();
    println!(
        "clean-channel BER = {clean_ber:.4} (rx {} bytes vs tx {})",
        rx_clean.len(),
        payload.len()
    );

    let sps = make_config().samples_per_symbol(); // 40
    let ebn0_points = [0.0, 2.0, 4.0, 6.0, 8.0, 10.0];
    let points = ebn0_sweep(payload, &clean, &ebn0_points, 1.0, sps, 60, 2024, demodulate_bpsk);
    println!("BPSK measured vs theory:\n{}", ebn0_sweep_to_csv(&points));

    let low = points.first().unwrap().ber; // 0 dB
    let high = points.last().unwrap().ber; // 10 dB

    // A real waterfall: clearly worse at low Eb/N0 than high.
    assert!(
        low > high,
        "BER should improve with Eb/N0: 0 dB {low}, 10 dB {high}"
    );
    // At 10 dB the link should be close to its noise-free floor.
    assert!(
        high <= clean_ber + 0.02,
        "high-Eb/N0 BER {high} far above noise-free floor {clean_ber}"
    );
    // Low-Eb/N0 BER should be in a sane range (not pinned at 0, not totally broken).
    assert!(
        low > bpsk_ber_theory(0.0) / 4.0,
        "0 dB BER {low} implausibly low — noise may not be reaching the demod"
    );
}

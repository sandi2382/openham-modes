//! Reliability metrics and the BER/FER-vs-SNR sweep harness.
//!
//! The sweep is intentionally decoupled from any modulator/demodulator: the
//! caller supplies the clean modulated signal plus a `decode` closure. This
//! keeps the harness in `core` (which cannot depend on `modem`) while letting
//! integration tests wire in real modems.

use crate::buffer::Complex;
use crate::channel::{add_awgn_ebn0, add_awgn_snr};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// A tally of bit errors over some number of transmitted bits.
#[derive(Debug, Clone, Copy, Default)]
pub struct BitErrors {
    pub errors: usize,
    pub total: usize,
}

impl BitErrors {
    /// Bit error rate in `[0, 1]`. Zero total bits yields 0.0.
    pub fn ber(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.errors as f64 / self.total as f64
        }
    }
}

/// Count differing bits between two byte streams. When the lengths differ, the
/// surplus bytes are charged as fully errored (8 bits each). This keeps the BER
/// meaningful even when a decoder drops or invents data instead of merely
/// flipping bits.
pub fn count_bit_errors(tx: &[u8], rx: &[u8]) -> BitErrors {
    let common = tx.len().min(rx.len());
    let mut errors = 0usize;
    for i in 0..common {
        errors += (tx[i] ^ rx[i]).count_ones() as usize;
    }
    let extra = tx.len().max(rx.len()) - common;
    errors += extra * 8;
    let total = tx.len().max(rx.len()) * 8;
    BitErrors { errors, total }
}

/// Whether two byte streams differ at all (a frame/packet error).
pub fn is_frame_error(tx: &[u8], rx: &[u8]) -> bool {
    tx != rx
}

/// One point on a BER/FER-vs-SNR curve.
#[derive(Debug, Clone, Copy)]
pub struct SweepPoint {
    pub snr_db: f64,
    pub ber: f64,
    pub fer: f64,
    pub trials: usize,
}

/// Run a BER/FER sweep over a set of SNR points.
///
/// * `tx_bytes` — the original payload that was modulated.
/// * `clean_samples` — the noise-free modulated signal for `tx_bytes`.
/// * `snr_db_points` — SNR values (dB) to evaluate.
/// * `trials` — independent noisy realizations per SNR point.
/// * `seed` — seeds the noise RNG for reproducibility.
/// * `decode` — demodulates a (noisy) signal back to bytes.
pub fn snr_sweep(
    tx_bytes: &[u8],
    clean_samples: &[Complex],
    snr_db_points: &[f64],
    trials: usize,
    seed: u64,
    mut decode: impl FnMut(&[Complex]) -> Vec<u8>,
) -> Vec<SweepPoint> {
    let mut points = Vec::with_capacity(snr_db_points.len());
    let mut rng = StdRng::seed_from_u64(seed);
    for &snr_db in snr_db_points {
        let mut bit_errs = BitErrors::default();
        let mut frame_errs = 0usize;
        for _ in 0..trials {
            let mut noisy = clean_samples.to_vec();
            add_awgn_snr(&mut noisy, snr_db, &mut rng);
            let rx = decode(&noisy);
            let be = count_bit_errors(tx_bytes, &rx);
            bit_errs.errors += be.errors;
            bit_errs.total += be.total;
            if is_frame_error(tx_bytes, &rx) {
                frame_errs += 1;
            }
        }
        points.push(SweepPoint {
            snr_db,
            ber: bit_errs.ber(),
            fer: frame_errs as f64 / trials.max(1) as f64,
            trials,
        });
    }
    points
}

/// One point on a BER/FER-vs-Eb/N0 curve.
#[derive(Debug, Clone, Copy)]
pub struct Ebn0Point {
    pub ebn0_db: f64,
    pub ber: f64,
    pub fer: f64,
    pub trials: usize,
}

/// Run a BER/FER sweep over Eb/N0 (dB). `bits_per_symbol` and
/// `samples_per_symbol` describe the modulation so the noise is calibrated
/// independently of oversampling, making results comparable to theory.
#[allow(clippy::too_many_arguments)]
pub fn ebn0_sweep(
    tx_bytes: &[u8],
    clean_samples: &[Complex],
    ebn0_db_points: &[f64],
    bits_per_symbol: f64,
    samples_per_symbol: f64,
    trials: usize,
    seed: u64,
    mut decode: impl FnMut(&[Complex]) -> Vec<u8>,
) -> Vec<Ebn0Point> {
    let mut points = Vec::with_capacity(ebn0_db_points.len());
    let mut rng = StdRng::seed_from_u64(seed);
    for &ebn0_db in ebn0_db_points {
        let mut bit_errs = BitErrors::default();
        let mut frame_errs = 0usize;
        for _ in 0..trials {
            let mut noisy = clean_samples.to_vec();
            add_awgn_ebn0(
                &mut noisy,
                ebn0_db,
                bits_per_symbol,
                samples_per_symbol,
                &mut rng,
            );
            let rx = decode(&noisy);
            let be = count_bit_errors(tx_bytes, &rx);
            bit_errs.errors += be.errors;
            bit_errs.total += be.total;
            if is_frame_error(tx_bytes, &rx) {
                frame_errs += 1;
            }
        }
        points.push(Ebn0Point {
            ebn0_db,
            ber: bit_errs.ber(),
            fer: frame_errs as f64 / trials.max(1) as f64,
            trials,
        });
    }
    points
}

/// Theoretical BPSK bit error rate over an AWGN channel:
/// `BER = Q(sqrt(2 * Eb/N0)) = 0.5 * erfc(sqrt(Eb/N0))`. Useful as a sanity
/// reference for a measured curve.
pub fn bpsk_ber_theory(ebn0_db: f64) -> f64 {
    let ebn0_lin = 10f64.powf(ebn0_db / 10.0);
    0.5 * erfc(ebn0_lin.sqrt())
}

/// Complementary error function via the Abramowitz & Stegun 7.1.26 rational
/// approximation (max abs error ~1.5e-7), adequate for a theory reference.
fn erfc(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    // y == erf(x); fold the sign back in and return erfc.
    1.0 - sign * y
}

/// Render sweep points as CSV (header included).
pub fn sweep_to_csv(points: &[SweepPoint]) -> String {
    let mut s = String::from("snr_db,ber,fer,trials\n");
    for p in points {
        s.push_str(&format!(
            "{:.2},{:.6e},{:.6},{}\n",
            p.snr_db, p.ber, p.fer, p.trials
        ));
    }
    s
}

/// Render Eb/N0 sweep points as CSV, including the theoretical BPSK BER for
/// reference (header included).
pub fn ebn0_sweep_to_csv(points: &[Ebn0Point]) -> String {
    let mut s = String::from("ebn0_db,ber,fer,bpsk_theory_ber,trials\n");
    for p in points {
        s.push_str(&format!(
            "{:.2},{:.6e},{:.6},{:.6e},{}\n",
            p.ebn0_db,
            p.ber,
            p.fer,
            bpsk_ber_theory(p.ebn0_db),
            p.trials
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_errors_when_identical() {
        let be = count_bit_errors(b"hello", b"hello");
        assert_eq!(be.errors, 0);
        assert_eq!(be.total, 40);
        assert_eq!(be.ber(), 0.0);
    }

    #[test]
    fn single_bit_flip_counts_one() {
        // 0x00 vs 0x01 differs in one bit.
        let be = count_bit_errors(&[0x00], &[0x01]);
        assert_eq!(be.errors, 1);
        assert_eq!(be.total, 8);
    }

    #[test]
    fn length_mismatch_penalized() {
        let be = count_bit_errors(b"abcd", b"ab");
        // 2 surplus bytes => 16 errored bits, total over the longer length.
        assert_eq!(be.errors, 16);
        assert_eq!(be.total, 32);
    }

    #[test]
    fn frame_error_detects_difference() {
        assert!(!is_frame_error(b"x", b"x"));
        assert!(is_frame_error(b"x", b"y"));
    }

    #[test]
    fn bpsk_theory_matches_known_values() {
        // Classic BPSK AWGN reference points.
        assert!((bpsk_ber_theory(0.0) - 0.0786).abs() < 5e-4); // ~7.86e-2
        assert!((bpsk_ber_theory(6.0) - 2.388e-3).abs() < 5e-5);
        assert!((bpsk_ber_theory(10.0) - 3.872e-6).abs() < 5e-7);
        // Monotonic decreasing.
        assert!(bpsk_ber_theory(0.0) > bpsk_ber_theory(4.0));
        assert!(bpsk_ber_theory(4.0) > bpsk_ber_theory(8.0));
    }

    /// A 1-sample/symbol real-BPSK link (k=1, sps=1) driven by the Eb/N0 harness
    /// should track the theoretical BPSK curve.
    #[test]
    fn ebn0_sweep_tracks_bpsk_theory() {
        let tx = b"OpenHam Eb/N0 vs theory check, padded for statistics....";
        let clean: Vec<Complex> = tx
            .iter()
            .flat_map(|byte| (0..8).map(move |i| (byte >> i) & 1))
            .map(|bit| Complex::new(if bit == 1 { 1.0 } else { -1.0 }, 0.0))
            .collect();

        let decode = |samples: &[Complex]| -> Vec<u8> {
            let bits: Vec<u8> = samples
                .iter()
                .map(|s| if s.real > 0.0 { 1u8 } else { 0u8 })
                .collect();
            bits.chunks(8)
                .map(|chunk| {
                    let mut byte = 0u8;
                    for (i, &b) in chunk.iter().enumerate() {
                        byte |= b << i;
                    }
                    byte
                })
                .collect()
        };

        // k=1, sps=1 -> Eb/N0 == per-sample SNR, directly comparable to theory.
        let pts = ebn0_sweep(tx, &clean, &[2.0, 6.0], 1.0, 1.0, 4000, 2024, decode);
        for p in &pts {
            let theory = bpsk_ber_theory(p.ebn0_db);
            // Measured BER should be within a factor of ~2.5 of theory (finite trials).
            assert!(
                p.ber > theory / 2.5 && p.ber < theory * 2.5 + 1e-4,
                "Eb/N0 {} dB: measured {} vs theory {}",
                p.ebn0_db,
                p.ber,
                theory
            );
        }
    }

    /// End-to-end sanity: a trivial 1-sample/bit real-BPSK link through the
    /// AWGN harness must show BER falling as SNR rises, reaching ~0 at high SNR.
    #[test]
    fn sweep_ber_decreases_with_snr() {
        let tx = b"OpenHam BER harness test payload!";
        // Modulate: each bit -> +1 / -1 on the real axis.
        let clean: Vec<Complex> = tx
            .iter()
            .flat_map(|byte| (0..8).map(move |i| (byte >> i) & 1))
            .map(|bit| Complex::new(if bit == 1 { 1.0 } else { -1.0 }, 0.0))
            .collect();

        let decode = |samples: &[Complex]| -> Vec<u8> {
            let bits: Vec<u8> = samples
                .iter()
                .map(|s| if s.real > 0.0 { 1u8 } else { 0u8 })
                .collect();
            bits.chunks(8)
                .map(|chunk| {
                    let mut byte = 0u8;
                    for (i, &b) in chunk.iter().enumerate() {
                        byte |= b << i;
                    }
                    byte
                })
                .collect()
        };

        let points = snr_sweep(tx, &clean, &[-2.0, 2.0, 6.0, 10.0], 200, 2024, decode);
        // Monotonic non-increasing BER with rising SNR.
        for w in points.windows(2) {
            assert!(
                w[1].ber <= w[0].ber + 1e-9,
                "BER should not rise with SNR: {:?}",
                points
            );
        }
        // High SNR essentially error-free.
        assert!(points.last().unwrap().ber < 1e-4, "high-SNR BER too high: {points:?}");
        // Low SNR clearly worse.
        assert!(points[0].ber > points.last().unwrap().ber);

        // CSV renders one header + one row per point.
        let csv = sweep_to_csv(&points);
        assert_eq!(csv.lines().count(), points.len() + 1);
    }
}

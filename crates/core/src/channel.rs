//! Channel impairment simulation.
//!
//! These utilities degrade a clean modulated signal so that mode reliability
//! can be measured (for example a BER-vs-SNR curve). Every source of
//! randomness is seedable so tests are deterministic and reproducible.
//!
//! The primitives are exposed both as free functions (used directly by the
//! measurement harness) and as a small composable [`Channel`] trait so several
//! impairments can be chained: e.g. a phase rotation followed by multipath and
//! then AWGN.

use crate::buffer::Complex;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rand_distr::{Distribution, StandardNormal};
use std::f64::consts::PI;

/// Average power of a complex signal: the mean of `|x|^2`.
pub fn signal_power(samples: &[Complex]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| s.norm_sqr()).sum();
    sum / samples.len() as f64
}

/// Measure the SNR (dB) between a clean reference and its noisy version,
/// where `noisy[n] = clean[n] + noise[n]`. Returns `+inf` if the two are
/// identical. Useful for verifying that a noise generator hit its target.
pub fn measure_snr_db(clean: &[Complex], noisy: &[Complex]) -> f64 {
    assert_eq!(clean.len(), noisy.len(), "clean/noisy length mismatch");
    let sig = signal_power(clean);
    let noise: Vec<Complex> = clean.iter().zip(noisy).map(|(c, n)| *n - *c).collect();
    let npow = signal_power(&noise);
    if npow == 0.0 {
        return f64::INFINITY;
    }
    10.0 * (sig / npow).log10()
}

/// Add complex additive white Gaussian noise to reach a target SNR (dB),
/// measured against the signal's own average power. The total noise power is
/// split evenly between the in-phase and quadrature components.
pub fn add_awgn_snr(samples: &mut [Complex], snr_db: f64, rng: &mut StdRng) {
    let sig = signal_power(samples);
    if sig == 0.0 {
        return;
    }
    let snr_lin = 10f64.powf(snr_db / 10.0);
    let noise_power = sig / snr_lin;
    // Half the noise power goes to each quadrature, so the per-component
    // standard deviation is sqrt(N/2).
    let sigma = (noise_power / 2.0).sqrt();
    for s in samples.iter_mut() {
        let zi: f64 = StandardNormal.sample(rng);
        let zq: f64 = StandardNormal.sample(rng);
        s.real += zi * sigma;
        s.imag += zq * sigma;
    }
}

/// Add complex AWGN calibrated to a target Eb/N0 (dB).
///
/// Unlike a raw per-sample SNR, Eb/N0 accounts for the modulation's
/// bits-per-symbol and its oversampling (samples-per-symbol), folding in the
/// matched-filter processing gain. This makes BER curves comparable across
/// modes and directly checkable against theory. The relationship used is
/// `Eb = S * sps / k` and `N0 = Eb / (Eb/N0)`, with the per-sample complex
/// noise power equal to `N0` (sample period normalized to 1).
pub fn add_awgn_ebn0(
    samples: &mut [Complex],
    ebn0_db: f64,
    bits_per_symbol: f64,
    samples_per_symbol: f64,
    rng: &mut StdRng,
) {
    let sig = signal_power(samples);
    if sig == 0.0 {
        return;
    }
    let ebn0_lin = 10f64.powf(ebn0_db / 10.0);
    let noise_power = sig * samples_per_symbol / (bits_per_symbol * ebn0_lin);
    let sigma = (noise_power / 2.0).sqrt();
    for s in samples.iter_mut() {
        let zi: f64 = StandardNormal.sample(rng);
        let zq: f64 = StandardNormal.sample(rng);
        s.real += zi * sigma;
        s.imag += zq * sigma;
    }
}

/// Add real-valued AWGN to reach a target SNR (dB) measured on the real
/// (passband audio) component only, leaving the imaginary part untouched. Use
/// this when the signal will be emitted as real audio (e.g. written to a mono
/// WAV), where complex noise would be half-discarded.
pub fn add_awgn_real_snr(samples: &mut [Complex], snr_db: f64, rng: &mut StdRng) {
    if samples.is_empty() {
        return;
    }
    let sig: f64 =
        samples.iter().map(|s| s.real * s.real).sum::<f64>() / samples.len() as f64;
    if sig == 0.0 {
        return;
    }
    let snr_lin = 10f64.powf(snr_db / 10.0);
    let sigma = (sig / snr_lin).sqrt();
    for s in samples.iter_mut() {
        let z: f64 = StandardNormal.sample(rng);
        s.real += z * sigma;
    }
}

/// Convenience wrapper: return a noisy copy of `clean` at the given SNR,
/// seeded for reproducibility.
pub fn awgn_at_snr(clean: &[Complex], snr_db: f64, seed: u64) -> Vec<Complex> {
    let mut out = clean.to_vec();
    let mut rng = StdRng::seed_from_u64(seed);
    add_awgn_snr(&mut out, snr_db, &mut rng);
    out
}

/// Apply a carrier frequency offset of `freq_hz`, rotating each sample by an
/// advancing phase. Models local-oscillator error between TX and RX.
pub fn apply_freq_offset(samples: &mut [Complex], freq_hz: f64, sample_rate: f64) {
    let w = 2.0 * PI * freq_hz / sample_rate;
    for (n, s) in samples.iter_mut().enumerate() {
        let phase = w * n as f64;
        *s = *s * Complex::new(phase.cos(), phase.sin());
    }
}

/// Apply a fixed phase offset (radians) to every sample.
pub fn apply_phase_offset(samples: &mut [Complex], radians: f64) {
    let rot = Complex::new(radians.cos(), radians.sin());
    for s in samples.iter_mut() {
        *s = *s * rot;
    }
}

/// A multipath tap: a delay (in samples) and a complex gain.
#[derive(Debug, Clone, Copy)]
pub struct MultipathTap {
    pub delay: usize,
    pub gain: Complex,
}

/// Apply a multipath channel: `y[n] = sum_k gain_k * x[n - delay_k]`.
/// The first tap is usually the direct path (delay 0, gain 1+0j). Output
/// length matches the input (echoes that would extend past the end are
/// truncated, as they would be in a fixed capture window).
pub fn apply_multipath(samples: &[Complex], taps: &[MultipathTap]) -> Vec<Complex> {
    let mut out = vec![Complex::new(0.0, 0.0); samples.len()];
    for tap in taps {
        for n in tap.delay..samples.len() {
            out[n] = out[n] + tap.gain * samples[n - tap.delay];
        }
    }
    out
}

/// A channel impairment that transforms a signal in place. Implementors may
/// reallocate the buffer (e.g. multipath), hence `&mut Vec`.
pub trait Channel {
    fn apply(&mut self, samples: &mut Vec<Complex>);
}

/// AWGN impairment at a fixed target SNR with its own seeded RNG.
pub struct Awgn {
    snr_db: f64,
    rng: StdRng,
}

impl Awgn {
    pub fn new(snr_db: f64, seed: u64) -> Self {
        Self {
            snr_db,
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl Channel for Awgn {
    fn apply(&mut self, samples: &mut Vec<Complex>) {
        add_awgn_snr(samples, self.snr_db, &mut self.rng);
    }
}

/// Constant carrier frequency offset impairment.
pub struct FreqOffset {
    pub freq_hz: f64,
    pub sample_rate: f64,
}

impl Channel for FreqOffset {
    fn apply(&mut self, samples: &mut Vec<Complex>) {
        apply_freq_offset(samples, self.freq_hz, self.sample_rate);
    }
}

/// Constant phase rotation impairment.
pub struct PhaseOffset {
    pub radians: f64,
}

impl Channel for PhaseOffset {
    fn apply(&mut self, samples: &mut Vec<Complex>) {
        apply_phase_offset(samples, self.radians);
    }
}

/// Static multipath impairment.
pub struct Multipath {
    pub taps: Vec<MultipathTap>,
}

impl Channel for Multipath {
    fn apply(&mut self, samples: &mut Vec<Complex>) {
        *samples = apply_multipath(samples, &self.taps);
    }
}

/// Apply several impairments in order. Order matters: multipath/phase before
/// AWGN models the receiver seeing distortion plus front-end noise.
#[derive(Default)]
pub struct ChannelChain {
    stages: Vec<Box<dyn Channel>>,
}

impl ChannelChain {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn then(mut self, stage: impl Channel + 'static) -> Self {
        self.stages.push(Box::new(stage));
        self
    }
}

impl Channel for ChannelChain {
    fn apply(&mut self, samples: &mut Vec<Complex>) {
        for stage in self.stages.iter_mut() {
            stage.apply(samples);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reproducible BPSK-ish test signal: random ±1 on the real axis.
    fn bpsk_signal(n: usize, seed: u64) -> Vec<Complex> {
        use rand::Rng;
        let mut rng = StdRng::seed_from_u64(seed);
        (0..n)
            .map(|_| Complex::new(if rng.gen::<bool>() { 1.0 } else { -1.0 }, 0.0))
            .collect()
    }

    #[test]
    fn signal_power_of_unit_samples_is_one() {
        let s = vec![Complex::new(1.0, 0.0); 100];
        assert!((signal_power(&s) - 1.0).abs() < 1e-12);
        let s2 = vec![Complex::new(3.0, 4.0); 50]; // |.|^2 = 25
        assert!((signal_power(&s2) - 25.0).abs() < 1e-12);
    }

    #[test]
    fn awgn_hits_target_snr() {
        // With many samples the measured SNR should be very close to target.
        let clean = bpsk_signal(200_000, 1);
        for target in [0.0, 6.0, 12.0, 20.0] {
            let noisy = awgn_at_snr(&clean, target, 42);
            let measured = measure_snr_db(&clean, &noisy);
            assert!(
                (measured - target).abs() < 0.3,
                "target {target} dB, measured {measured} dB"
            );
        }
    }

    #[test]
    fn awgn_ebn0_yields_expected_per_sample_snr() {
        // For Eb/N0 = ebn0, k bits/symbol, sps samples/symbol, the resulting
        // per-sample SNR is ebn0 * k / sps. Check that the noise we add matches.
        let clean = bpsk_signal(200_000, 5);
        let ebn0_db = 8.0;
        let (k, sps) = (1.0, 40.0);
        let mut noisy = clean.clone();
        let mut rng = StdRng::seed_from_u64(77);
        add_awgn_ebn0(&mut noisy, ebn0_db, k, sps, &mut rng);

        let ebn0_lin = 10f64.powf(ebn0_db / 10.0);
        let expected_snr_db = 10.0 * (ebn0_lin * k / sps).log10();
        let measured = measure_snr_db(&clean, &noisy);
        assert!(
            (measured - expected_snr_db).abs() < 0.3,
            "expected {expected_snr_db} dB per-sample SNR, measured {measured} dB"
        );
    }

    #[test]
    fn real_awgn_hits_target_snr_and_leaves_imag() {
        let clean = bpsk_signal(200_000, 17); // real ±1, imag 0
        let mut noisy = clean.clone();
        let mut rng = StdRng::seed_from_u64(5);
        add_awgn_real_snr(&mut noisy, 10.0, &mut rng);

        let sig: f64 =
            clean.iter().map(|s| s.real * s.real).sum::<f64>() / clean.len() as f64;
        let npow: f64 = clean
            .iter()
            .zip(&noisy)
            .map(|(c, n)| {
                let d = n.real - c.real;
                d * d
            })
            .sum::<f64>()
            / clean.len() as f64;
        let measured = 10.0 * (sig / npow).log10();
        assert!((measured - 10.0).abs() < 0.3, "measured {measured} dB");
        assert!(noisy.iter().all(|s| s.imag == 0.0), "imag must be untouched");
    }

    #[test]
    fn awgn_is_deterministic_with_seed() {
        let clean = bpsk_signal(1000, 7);
        let a = awgn_at_snr(&clean, 5.0, 123);
        let b = awgn_at_snr(&clean, 5.0, 123);
        assert_eq!(a, b);
        let c = awgn_at_snr(&clean, 5.0, 124);
        assert_ne!(a, c);
    }

    #[test]
    fn freq_offset_is_invertible() {
        let clean = bpsk_signal(500, 3);
        let mut s = clean.clone();
        apply_freq_offset(&mut s, 137.0, 48000.0);
        apply_freq_offset(&mut s, -137.0, 48000.0);
        for (a, b) in clean.iter().zip(&s) {
            assert!((a.real - b.real).abs() < 1e-9 && (a.imag - b.imag).abs() < 1e-9);
        }
    }

    #[test]
    fn phase_offset_rotates_and_preserves_power() {
        let mut s = vec![Complex::new(1.0, 0.0)];
        apply_phase_offset(&mut s, PI / 2.0);
        assert!(s[0].real.abs() < 1e-12 && (s[0].imag - 1.0).abs() < 1e-12);

        let clean = bpsk_signal(1000, 9);
        let mut rotated = clean.clone();
        apply_phase_offset(&mut rotated, 0.789);
        assert!((signal_power(&clean) - signal_power(&rotated)).abs() < 1e-9);
    }

    #[test]
    fn multipath_direct_path_is_identity() {
        let clean = bpsk_signal(100, 11);
        let taps = [MultipathTap {
            delay: 0,
            gain: Complex::new(1.0, 0.0),
        }];
        let out = apply_multipath(&clean, &taps);
        assert_eq!(out, clean);
    }

    #[test]
    fn multipath_echo_adds_delayed_copy() {
        let x = vec![
            Complex::new(1.0, 0.0),
            Complex::new(2.0, 0.0),
            Complex::new(3.0, 0.0),
        ];
        let taps = [
            MultipathTap {
                delay: 0,
                gain: Complex::new(1.0, 0.0),
            },
            MultipathTap {
                delay: 1,
                gain: Complex::new(0.5, 0.0),
            },
        ];
        let out = apply_multipath(&x, &taps);
        // y[0]=1, y[1]=2+0.5*1=2.5, y[2]=3+0.5*2=4
        assert!((out[0].real - 1.0).abs() < 1e-12);
        assert!((out[1].real - 2.5).abs() < 1e-12);
        assert!((out[2].real - 4.0).abs() < 1e-12);
    }

    #[test]
    fn channel_chain_applies_in_order() {
        let clean = bpsk_signal(2000, 13);
        // Phase rotation then AWGN.
        let mut chain = ChannelChain::new()
            .then(PhaseOffset { radians: 0.5 })
            .then(Awgn::new(10.0, 99));
        let mut sig = clean.clone();
        chain.apply(&mut sig);
        // Power should be roughly clean power plus noise power (~+0.41 dB at 10 dB SNR).
        assert!(sig.len() == clean.len());
        let snr_vs_rotated = {
            let mut rotated = clean.clone();
            apply_phase_offset(&mut rotated, 0.5);
            measure_snr_db(&rotated, &sig)
        };
        assert!(
            (snr_vs_rotated - 10.0).abs() < 0.5,
            "expected ~10 dB after chain, got {snr_vs_rotated}"
        );
    }
}

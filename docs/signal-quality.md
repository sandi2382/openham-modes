# Reception quality and noise robustness

When you receive a transmission, `openham rx` prints a quality line per decoded
message, for example:

```
Message 1 (bpsk): SNR: 37.4 dB, EVM: 1.4%
  CQ DE S56SPZ TEST 123 K
```

This page explains those numbers, how each mode behaves under noise, and how
much noise is "too much" per mode.

## What the numbers mean

- **SNR (dB)** — an estimate of the *post-detection* signal-to-noise ratio,
  measured from the recovered symbols. It includes the matched-filter
  processing gain (≈ `10·log10(samples_per_symbol)`, ~26 dB at 48 kHz / 125
  baud), so it reads higher than the raw on-air channel SNR. Treat it as a
  relative "how clean is the decode" indicator: higher is better.
- **EVM (%)** — error vector magnitude: how far the received constellation
  points sit from their ideal positions, as a percentage. Lower is better.
  Only meaningful for the phase/amplitude modes (BPSK, …) that have a
  constellation; the frequency modes report EVM as 0.

### Per-mode behaviour

| Mode | SNR estimate | EVM | Notes |
|------|--------------|-----|-------|
| **bpsk** | tracks noise well | tracks noise well | Coherent; EVM is the most direct quality indicator. |
| **fsk**  | tracks noise well | n/a (0) | Non-coherent; SNR is the mark/space energy-discrimination ratio. |
| **afsk** | limited | n/a (0) | Bell-202 tones are not orthogonal, so the discrimination ratio floors around ~16 dB and only drops once noise is severe. Use it as a coarse indicator. |
| **psk4 / ofdm64** | not yet populated | not yet populated | Quality metrics not implemented for these demods. |

## How much noise is too much (per mode)

Measured frame-error-rate vs. on-air channel SNR (real AWGN, ~30-byte payload).
"Reliable" means ~0 frame errors; numbers are approximate and depend on payload
length.

| Mode | Reliable down to (channel SNR) | Robustness |
|------|-------------------------------|------------|
| **bpsk**  | ~5 dB | Excellent — near theoretical BPSK; the most robust mode. |
| **fsk**   | ~5 dB | Excellent. |
| **afsk**  | ~25 dB | Works only on a clean band; degrades fast below ~20 dB. |
| **psk4**  | not robust | Decodes clean but fails under even mild noise. |
| **ofdm64**| not robust | Decodes clean but loses ~⅓ of frames at 25 dB. |

There is **no forward error correction yet**, so these are raw-channel figures.
FEC (planned) will substantially lower the required SNR.

**Recommendation for on-air use:** prefer **bpsk** or **fsk**. They are the only
modes that are both robust to noise and able to acquire a transmission at an
arbitrary point in a live audio stream.

## Measuring it yourself

- Add calibrated noise to a transmission and watch it decode:
  ```
  openham tx -o sig.wav -t "CQ DE S56SPZ K" -c S56SPZ -m bpsk --snr-db 6
  openham rx -i sig.wav -m bpsk
  ```
- Generate the full BER/FER-vs-SNR curves from the test harness:
  ```
  cargo test -p openham-modem --test ber_sweep -- --nocapture          # BPSK vs theory
  cargo test -p openham-tools --test end_to_end working_modes_fer_vs_snr -- --ignored --nocapture
  ```

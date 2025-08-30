# Architecture overview

This document describes the modular architecture and the responsibilities of each component.

High-level layers
- Hardware / SDR layer
  - Drivers for RTL-SDR, HackRF, SDRplay, sound card I/O.
  - Abstraction layer exposes: sample rate, center frequency, IQ or audio PCM streams.

- Modem / DSP layer (crates/core, crates/modem)
  - Resampling, filtering, AGC, synchronization primitives.
  - Modulation primitives (FSK/PSK/OFDM building blocks).
  - Framing and FEC (crates/frame): Reed-Solomon, convolutional, LDPC (optional), interleaving.

- Codecs layer (crates/codecs)
  - Text codec(s): serialized text -> bitstream -> framed payload.
  - Voice codec(s): integrate open voice codecs (e.g., Opus in narrowband configuration) or custom low-bitrate vocoder (ensure not encrypted).
  - Registry to map mode identifiers to codec implementations.

- Transport & container
  - On-air framing and container (.odm for recordings).
  - Metadata exchange (sample_rate, mode_id, version, operator_id, spec_url).

- UI & Tools
  - CLI: tx/rx/analyze/synth.
  - Desktop UI: Tauri + React for control and visualization.
  - Web UI: WASM integration for browser-based demos (receive-only recommended for real transmissions).

Plugin model
- Primary plugin mechanism: WASM modules implementing a defined API contract (register(), process(), metadata()).
- Secondary: dynamic libraries with a clear FFI layer (used when WASM is not viable).
- Plugins declare capabilities with semantic versions; host can load compatible plugin versions.

Testing strategy
- Unit tests for DSP primitives (filters, resampler).
- Property-based tests for framing (quickcheck).
- Integration smoke tests: encode a payload -> modulate -> demodulate -> decode -> assert round-trip.
- Hardware-in-the-loop: optional tests that interact with SDR hardware (marked with CI tag and not run by default).

Security & safety
- Avoid unsafe code unless necessary; mark unsafe sections with reviews and explicit tests.
- Use WASM sandboxing for untrusted plugins.

Data formats
- Mode spec (YAML) includes:
  - id: e.g., ohm.text.v1
  - description
  - on-air framing parameters (symbol rate, modulation, FEC)
  - preamble recommendations (CW, spoken)
  - sample waveform examples

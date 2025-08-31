# OpenHamModes

Repository slug: `openham-modes`

Short description
- OpenHamModes is an open, modular, public project to design, document, and implement non-encrypted experimental digital communications modes for amateur radio (HF/VHF/UHF and audio-over-SDR). The goal is a future-proof, cross-platform codebase for text and digital voice modes (initial), extensible to files, images, and video, while ensuring transmissions remain clearly non-encrypted (morse + spoken identification preamble is recommended).

Legal / policy note

- The project publishes all specifications, source code, and tools openly.
- Do not use encryption for on-air payloads (project policy).
- Transmit a clear identification preamble before uncommon / experimental modes transmissions (e.g., CW identification and a short spoken phrase identifying operator and spec location) so traffic cannot be mistaken for encrypted communications. Example preamble: "This is S56SPZ conducting experimental transmissions with an open digital mode, specification published at github.com/sandi2382/openham-modes or s56spz.com/ohm".

Disclaimer

- **THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND.** The authors and contributors are not responsible for any issues, problems, interference, or damage that may result from the use of this software.
- Users are solely responsible for ensuring compliance with local amateur radio regulations, licensing requirements, and band plans.
- This is experimental software intended for research and educational purposes. Use at your own risk.
- No guarantee is provided regarding signal quality, compatibility, or fitness for any particular purpose.

Goals

- Implement two initial modes: text (low-bitrate, robust) and digital voice.
- Make the core DSP, framing, and codecs portable, safe, and high-performance.
- Provide CLI, desktop, and web UIs; provide bindings for scripting and experimentation.
- Make it easy to add new modes, codecs, modulation parameters, and transport layers.
- Provide tools to capture, replay, analyze, and synthesize transmissions.
- Emphasize modularity, testability, and reproducible builds.

High-level technology choices (rationale)
- Core signal processing & mode implementations: Rust
  - Reasons: memory-safety, high-performance, excellent tooling (cargo), cross-compilation, native WebAssembly (WASM) target for browser usage, good FFI support for Python/Node.
- CLI tools & instrumentations: Rust (binaries) + small Python wrappers for rapid experimentation.
- Desktop UI: Tauri (Rust backend + TypeScript/React frontend) — lightweight and cross-platform.
- Web UI: React + TypeScript, compiled to static pages and WASM where appropriate (for real-time DSP in the browser).
- Scripting & rapid prototyping: Python bindings (pyo3) to core Rust crates.
- Packaging & distribution: Cargo for Rust crates, npm for frontend, GitHub Actions for CI to build cross-platform artifacts.
- Compression (optional, not encryption): Zstandard (zstd) and LZ4 options (implementable as pluggable filters).
- Binary plugin support: WASM (preferred) and dynamic libraries (dlopen) for sandboxing and portability.

Repository layout (top-level)
- /README.md (this file)
- /LICENSE (MIT recommended)
- /CODE_OF_CONDUCT.md
- /CONTRIBUTING.md
- /.github/workflows/ci.yml
- /docs/ (specs, architecture, on-air rules, modulators, sample waveforms)
- /specs/ (formal mode specs in machine- and human-readable formats)
- /crates/ (Rust workspace)
  - /crates/core/ (DSP primitives, sample buffers, resampling, FIR/IIR, FFT wrappers)
  - /crates/frame/ (framing, FEC, interleavers)
  - /crates/codecs/ (text codec, voice codec wrappers, codec registry)
  - /crates/modem/ (modulators/demodulators: audio, PSK, FSK, OFDM building blocks)
  - /crates/tools/ (CLI tools: tx, rx, analyze, synth)
  - /crates/bindings-python/ (PyO3 bindings)
  - /crates/build-helpers/ (cbindgen, packaging helpers)
- /apps/
  - /apps/tauri-ui/ (desktop UI)
  - /apps/web-ui/ (web-based UI)
  - /apps/examples/ (short examples and notebooks)
- /specimen/ (sample waveforms and recorded captures)
- /tests/ (integration tests, hardware-in-loop test harness)
- /ci-scripts/ (release and cross-compilation helpers)

Initial file formats and identifiers
- Config format: TOML for local configs (human readable, strong ecosystem), JSON for APIs.
- Mode spec format: YAML + JSON Schema (for automation).
- Recorded sample container: .odm (Open Digital Mode) — simple container:
  - ASCII header (JSON metadata: mode, sample_rate, timestamp, tx_id, spec_url)
  - binary payload containing raw IQ or PCM samples (optionally compressed).
- Versioning: Semantic Versioning (MAJOR.MINOR.PATCH).
- Mode identifiers: namespace style (e.g., ohm.text.v1, ohm.voice.v1).

Design principles / software practices
- Core library design:
  - Small focused crates (single responsibility) in a Cargo workspace.
  - Clear public API surfaces; internal implementation hidden behind crate-private items.
  - Stable ABI only for explicitly exported C bindings; prefer linking via Rust crates or WASM.
  - Feature flags to enable/disable optional dependencies (e.g., FFT backend).
- Extensibility:
  - Plugin architecture using WASM modules and a plugin registry (sandboxed).
  - Codec & mode registration via capability descriptors and semantic versioning.
  - Backward/forward compatibility rules in specs (minor version compatibility allowed; breaking changes require new major).
- Safety and reliability:
  - Unit tests, integration tests, fuzzing targets (cargo-fuzz) for core DSP and frame handling.
  - Continuous integration: build+test on Linux, macOS, Windows, and run linters.
  - Static analysis: clippy, cargo-deny, wasm-opt for WASM artifacts.
- Observability:
  - Structured logging (tracing crate in Rust), multi-level verbosity.
  - Telemetry only opt-in and local by default — do not transmit telemetry over the air.
- Documentation:
  - Human-readable specs (docs/specs/*.md).
  - Machine-readable specs (specs/*.yaml + JSON Schema).
  - Examples and cookbooks in docs/cookbook.md.
- Licensing:
  - MIT or BSD-3 (permissive) to encourage reuse.
  - Contributor License Agreement not required (unless project maintainers later decide).
- Community & governance:
  - CONTRIBUTING.md, CODE_OF_CONDUCT.md, issue and PR templates.
  - Use Issues + Projects for feature planning; label schemes for triage.

Initial CI (GitHub Actions)
- Build Rust workspace on stable and latest nightly (optional features).
- Run unit and integration tests.
- Build Python wheel for bindings and run a minimal smoke test (if Python runtime available).
- Build web UI (npm) and run lint + basic tests.
- Produce artifacts: Rust release binaries, WASM artifacts, Python wheel, frontend build.

On-air rules & recommended preamble
- Before sending with uncommon modes, send:
  1) CW identification (operator callsign) — e.g., "... --- ..." style is not necessary; standard amateur CW ID suffices.
  2) Spoken identification (short) including operator callsign and URL to spec/repo: "This is S56SPZ conducting experimental transmissions with an open digital mode, specification published at github.com/sandi2382/openham-modes or s56spz.com/ohm"
- Transmissions must obey local regulations (band/timing/power).
- The project will publish recommended preamble templates and example waveforms in docs/preamble.md.

Contact / maintainer
- Maintainer: sandi2382 (replace with primary maintainer as needed)

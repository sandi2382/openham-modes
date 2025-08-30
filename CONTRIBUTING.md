# Contributing to OpenHamModes

Thank you for your interest! This document describes the recommended workflow and expectations for contributions.

1. Filing issues
- Provide a clear title and description.
- Reproduce steps (include commands, sample waveforms, or hardware used).
- If proposing a new mode, include:
  - Name/slug
  - Purpose and target use cases
  - Modulation/bitrate/expected spectral footprint
  - Example over-the-air preamble

2. Branching & PRs
- Fork the repo and create branches named `feature/<short-desc>` or `fix/<short-desc>`.
- Run tests locally before opening a PR.
- PRs should link to issues or describe the motivation.
- Small, reviewable PRs preferred.

3. Code style & tests
- Rust: run `cargo fmt` and `cargo clippy` before PRs.
- Provide unit tests for new functionality where possible.
- Add integration tests for end-to-end mode validation.

4. Documentation
- Update specs in `specs/` and human docs in `docs/` for any mode, API, or behavioral change.
- Provide example waveforms in `specimen/` for new modes.

5. Legal & on-air
- Contributions must not add encrypted over-the-air payloads. Compression-only schemes are allowed.
- Follow local amateur radio rules when generating embodied artifacts for testing on real hardware.

6. Code of conduct
- Please see CODE_OF_CONDUCT.md.

# Architecture Overview

This document describes the modular architecture and the responsibilities of each component in the OpenHam digital modes library.

## High-Level Layers

### Audio I/O Layer
- **WAV file I/O**: Read and write 48kHz 16-bit PCM audio files
- **Professional audio processing**: High-quality signal generation and analysis
- **No hardware dependencies**: Pure software implementation for portability

### Modem / DSP Layer (crates/core, crates/modem)
- **Core DSP primitives**: FFT operations, filtering, buffering, resampling
- **Stable modulation schemes**:
  - BPSK: Binary Phase Shift Keying with robust sync detection
  - FSK: Frequency Shift Keying with noncoherent energy detection  
  - AFSK: Audio FSK with Bell 202/103/VHF/HF profile support
  - OFDM: Orthogonal Frequency Division Multiplexing with pilot equalization
- **Experimental schemes** (partial implementation):
  - PSK variants (QPSK, 8PSK, 16PSK)
  - QAM modes (16/64/256/1024-QAM)

### Codecs Layer (crates/codecs)
- **Text codecs**:
  - Huffman codec with ham radio token support (Q-codes, abbreviations)
  - ASCII codec for uncompressed text
  - UTF-8 Unicode support with exact reconstruction
- **Audio utilities**:
  - CW/Morse code generation with configurable WPM
  - Voice announcement integration (WAV file playback)
  - Pink noise generation for squelch triggering

### Frame Handling (crates/frame)
- **Sync detection**: HDLC-like pattern [55 55 55 55 AA AA 7E 7E]
- **Bit alignment tolerance**: 8-position scan with MSB/LSB fallback
- **Inversion detection**: Automatic polarity correction
- **Simple framing**: Basic container for payload data

### Tools & CLI (crates/tools)
- **Unified CLI tool** (`openham`): TX/RX/generate/info modes
- **Auto-detection**: Multi-demodulator signal identification
- **Enhanced features**: CW preambles, pink noise, voice ID, power control
- **Configuration management**: Flexible parameter handling

## Current Implementation Status

### ✅ Stable Components (100% Test Pass Rate)
- BPSK, FSK, AFSK, OFDM modulation schemes
- Huffman and ASCII text codecs
- CW generation and audio utilities
- Frame sync detection and handling
- Unified CLI tool with full feature set
- Comprehensive test suite (98 test cases)

### ⚠️ Experimental Components (Partial Implementation)
- PSK variants (TX working, RX issues)
- QAM modes (TX working, sync failures)
- Advanced FEC (basic framework only)

### ❌ Not Implemented
- Hardware/SDR integration
- Voice codecs
- Plugin architecture with WASM
- Network protocols
- GUI applications

## Design Principles

### Safety and Reliability
- **Memory safety**: Zero unsafe code, full Rust safety guarantees
- **Error handling**: Comprehensive Result types throughout
- **Testing**: Zero-tolerance round-trip validation policy
- **Quality assurance**: 100% pass rate for stable components

### Modularity
- **Trait-based design**: Clean Modulator/Demodulator interfaces
- **Single responsibility**: Focused crates with clear boundaries
- **Configuration-driven**: Flexible parameter management
- **Extensible**: Easy to add new modulation schemes

### Performance
- **Optimized DSP**: Efficient FFT operations and signal processing
- **Memory efficient**: Zero-copy operations where possible
- **Thread-safe**: Send/Sync implementations for concurrency

## Testing Strategy

### Comprehensive Validation
- **Grid testing**: Modulation × codec × options matrix
- **Round-trip testing**: Exact content matching enforced
- **Integration tests**: End-to-end TX/RX cycles
- **Error handling**: Proper validation for edge cases

### Quality Metrics
- **62 stable tests passing** out of 98 total test cases
- **Zero-tolerance policy**: No approximations in validation
- **Performance tracking**: Signal quality and processing metrics

# OpenHam Text Mode v1 Specification

## Overview

OpenHam Text Mode v1 (ohm.text.v1) is a simple, robust text communication mode for amateur radio. It provides reliable transmission of text messages using proven modulation schemes and efficient text encoding.

## Design Goals

- **Robust modulation**: Multiple stable schemes (BPSK, FSK, AFSK, OFDM)
- **Efficient text encoding**: Ham radio optimized Huffman compression
- **Amateur radio compliance**: No encryption, clear station identification
- **Simple implementation**: Straightforward to implement and decode
- **Proven reliability**: 100% test pass rate for stable components

## Technical Parameters

### Modulation Schemes (All Stable)

#### BPSK (Binary Phase Shift Keying)
- **Symbol rate**: 125 baud (configurable)
- **Carrier frequency**: 1500 Hz (configurable)
- **Phase shift**: 0°/180°
- **Bandwidth**: ~250 Hz

#### FSK (Frequency Shift Keying)
- **Symbol rate**: 125 baud (configurable)
- **Mark frequency**: 1615 Hz (configurable)
- **Space frequency**: 1385 Hz (configurable)
- **Frequency shift**: 230 Hz

#### AFSK (Audio FSK)
- **Bell 202**: 1200 baud, 1200/2200 Hz
- **Bell 103**: 300 baud, 1070/1270 Hz  
- **VHF**: 1200 baud, 1200/2200 Hz
- **HF**: 300 baud, 1600/1800 Hz

#### OFDM (Orthogonal Frequency Division Multiplexing)
- **Subcarriers**: 64 with pilot equalization
- **Cyclic prefix**: For multipath resilience
- **Symbol rate**: 125 baud effective
- **Bandwidth**: ~2000 Hz

### Frame Structure

#### Sync Pattern
```text
0x55 0x55 0x55 0x55 0xAA 0xAA 0x7E 0x7E
```
Standard HDLC-like synchronization sequence providing:
- **Bit synchronization**: Alternating 0x55 patterns
- **Polarity detection**: 0xAA for inversion detection  
- **Frame start**: 0x7E markers for frame boundary

#### Frame Format
```text
| Sync (8 bytes) | Payload (variable) |
```

Simple framing with:
- **Robust sync detection**: 8-position bit alignment scan
- **Inversion tolerance**: Automatic polarity correction
- **Variable payload**: Text-dependent length

## Text Encoding

### Huffman Compression

Canonical Huffman encoding optimized for amateur radio text with:

#### Ham Radio Token Support
**Q-codes** (mapped to Unicode Private Use Area):
- QRZ, QRM, QRO, QRP, QRS, QRT, QRB, QSB
- QSL, QSO, QSY, QTH, and others

**Common abbreviations**:
- CQ, DE, BK, KN, K, AR, SK, YL, OM, 73
- Standard amateur radio shorthand

#### Character Encoding
- **Primary**: UTF-8 Unicode support
- **Compression**: Ham radio optimized Huffman table
- **Fallback**: ASCII for compatibility
- **Reconstruction**: Exact round-trip guarantee

### ASCII Codec

Uncompressed text transmission:
- **Direct encoding**: One-to-one character mapping
- **Full UTF-8 support**: Complete Unicode compatibility
- **No compression**: Maximum compatibility
- **Exact preservation**: Perfect reconstruction

## Protocol Operation

### Transmission Sequence

1. **Optional preamble** (user configurable):
   - CW identification
   - Voice announcement
   - Pink noise trigger

2. **Digital transmission**:
   - Sync pattern transmission
   - Text encoding (Huffman or ASCII)
   - Modulation and transmission

3. **Frame detection**:
   - Sync pattern correlation
   - Bit alignment search (8 positions)
   - Polarity correction if needed

### Error Handling

- **Sync detection**: Robust correlation with tolerance
- **Bit alignment**: Multiple position scanning
- **Polarity correction**: Automatic inversion detection
- **Frame validation**: Basic integrity checking

## Implementation Guidelines

### Audio Parameters

- **Sample Rate**: 48 kHz (configurable)
- **Bit Depth**: 16-bit PCM
- **Channels**: Mono
- **Format**: WAV (uncompressed)

### Performance Requirements

- **Frequency accuracy**: ±10 Hz recommended
- **Timing accuracy**: ±1% symbol timing
- **Signal-to-noise ratio**: Variable by modulation scheme
- **Processing**: Real-time capable on modern hardware

### Software Implementation

```rust
// Example implementation structure
pub struct OhmTextV1 {
    modulator: Box<dyn Modulator>,      // BPSK/FSK/AFSK/OFDM
    demodulator: Box<dyn Demodulator>,
    text_codec: Box<dyn TextCodec>,     // Huffman or ASCII
    sync_detector: SyncDetector,
}
```

## Testing and Validation

### Test Coverage

Current implementation provides **100% test pass rate** for:
- All stable modulation schemes (BPSK, FSK, AFSK, OFDM)
- Text codecs (Huffman and ASCII)
- Round-trip transmission and reception
- Enhanced features (CW, pink noise, voice ID)

### Test Cases

Available in test suite:
- **Grid testing**: 48 combinations of modulation × codec × options
- **Round-trip validation**: Exact content matching
- **Auto-detection**: Multi-demodulator testing
- **Edge cases**: Error handling and recovery

## On-Air Protocol

### Station Identification

Recommended preamble sequence:
1. **CW identification**: Transmitting station callsign
2. **Voice announcement**: "Experimental digital transmission using OpenHam Text Mode"
3. **Digital transmission**: Text payload with frame sync

### Compliance

- **No encryption**: All data transmitted in clear
- **Proper identification**: Station callsign in preamble
- **Band plan compliance**: Use appropriate digital frequencies
- **Power limits**: Respect license class restrictions

## Current Status

### ✅ Stable Implementation
- **BPSK, FSK, AFSK, OFDM**: 100% test pass rate
- **Text codecs**: Huffman and ASCII fully working
- **CLI tool**: Complete implementation
- **Round-trip testing**: Exact match validation

### ⚠️ Experimental Features
- **PSK variants**: QPSK/8PSK/16PSK (TX working, RX issues)
- **QAM modes**: 16/64/256/1024-QAM (TX working, sync failures)

## Usage Examples

### Basic Transmission
```bash
# BPSK with Huffman compression
./openham tx -o signal.wav -t "Hello from S56SPZ" -c S56SPZ -m bpsk

# OFDM with ASCII encoding
./openham tx -o signal.wav -t "Test message" -c S56SPZ -m ofdm --text-codec ascii
```

### Reception
```bash
# Auto-detection
./openham rx -i signal.wav --auto-detect

# Specific modulation
./openham rx -i signal.wav -m bpsk -o decoded.txt
```

## References

- [OpenHam Modes Repository](https://github.com/sandi2382/openham-modes)
- [USAGE.md](../USAGE.md) - Detailed usage instructions
- [ARCHITECTURE.md](../docs/ARCHITECTURE.md) - Technical architecture
- Amateur Radio Digital Communication Standards

---

**Status**: Stable implementation - ready for use  
**Version**: 1.0  
**Test Status**: 100% pass rate for stable features  
**Contact**: See repository for current maintainers
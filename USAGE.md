# OpenHam Digital Modes - Usage Guide

## Overview

OpenHam is a digital modes library for amateur radio, implementing stable modulation schemes (BPSK, FSK, AFSK, OFDM), text codecs, CW generation, and audio processing capabilities.

## Disclaimer

**THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND.** The authors and contributors are not responsible for any issues, problems, interference, or damage that may result from the use of this software.

- Users are solely responsible for ensuring compliance with local amateur radio regulations, licensing requirements, and band plans.
- This is experimental software intended for research and educational purposes. Use at your own risk.
- No guarantee is provided regarding signal quality, compatibility, or fitness for any particular purpose.
- Always test thoroughly before using on-air and ensure proper amateur radio identification procedures.

## Quick Start

### Build the Project

```bash
cargo build --release
```

### Basic Usage

The main tool is accessed via the `openham` binary with the following modes:

- `tx` - Transmit data
- `rx` - Receive data  
- `generate` - Generate test signals
- `info` - Display system information

## Transmission Examples

### Basic Text Transmission

```bash
# BPSK transmission
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m bpsk

# FSK transmission
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m fsk

# AFSK transmission
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m afsk

# OFDM transmission
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m ofdm
```

### File Input Transmission

```bash
# Transmit from text file
./target/release/openham tx -o output.wav -f message.txt -c S56SPZ -m bpsk
```

### Enhanced Features

```bash
# With CW preamble
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m bpsk --cw-preamble

# With pink noise squelch trigger
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m bpsk --pink-noise

# High power transmission
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m bpsk --power 0.9

# Combined features
./target/release/openham tx -o output.wav -t "Hello World" -c S56SPZ -m fsk --cw-preamble --pink-noise
```

### Text Codec Selection

```bash
# Use Huffman (default, UTF-8 safe canonical Huffman with ham radio tokens)
./target/release/openham tx -o output.wav -t "Hello" -c S56SPZ -m bpsk --text-codec huffman

# Use ASCII (no compression)
./target/release/openham tx -o output.wav -t "Hello" -c S56SPZ -m bpsk --text-codec ascii
```

### Voice Identification Preamble

```bash
# Prepend a pre-recorded WAV voice ID before data
./target/release/openham tx -o output.wav -t "Hello" -c S56SPZ -m bpsk --voice-id voice_id.wav
```

## Reception Examples

### Basic Reception

```bash
# BPSK reception
./target/release/openham rx -i signal.wav -o decoded.txt -m bpsk

# FSK reception
./target/release/openham rx -i signal.wav -o decoded.txt -m fsk

# AFSK reception
./target/release/openham rx -i signal.wav -o decoded.txt -m afsk

# OFDM reception
./target/release/openham rx -i signal.wav -o decoded.txt -m ofdm
```

### Auto-Detection Mode

```bash
# Automatically detect modulation type
./target/release/openham rx -i signal.wav -o decoded.txt --auto-detect

# Hint the text codec for decoding
./target/release/openham rx -i signal.wav -o decoded.txt -m bpsk --text-codec huffman
```

### Console Output

```bash
# Output to console instead of file
./target/release/openham rx -i signal.wav -m bpsk
```

Note: When `-o` is used and exactly one message is decoded, the output file contains the raw decoded text without any prefixes or trailing whitespace, enabling exact round-trip comparisons.

## Signal Generation

### Test Signals

```bash
# Sine wave at 1000 Hz for 3 seconds
./target/release/openham generate -o test_sine.wav -s sine -f 1000 -d 3

# White noise for 2 seconds
./target/release/openham generate -o test_noise.wav -s noise -d 2

# Frequency sweep from 500-2000 Hz for 5 seconds
./target/release/openham generate -o test_sweep.wav -s sweep -f 500 -d 5

# Morse code test signal
./target/release/openham generate -o test_morse.wav -s morse -f 600 -d 3
```

## Stable Features

### Modulation Schemes (100% Test Pass Rate)

#### BPSK (Binary Phase Shift Keying)

- Simple, robust modulation
- Good for weak signal conditions
- Binary data encoding
- Complete TX/RX implementation

#### FSK (Frequency Shift Keying)  

- Frequency-based modulation
- Good noise immunity
- Mark/space frequency shifting
- Noncoherent energy detection

#### AFSK (Audio FSK)

- Audio frequency shift keying
- Bell 202/103/VHF/HF profiles
- Full compatibility with existing systems
- Excellent noise immunity

#### OFDM (Orthogonal Frequency Division Multiplexing)

- Multi-carrier modulation
- High spectral efficiency
- Resilient to multipath fading
- 64-point FFT with pilot equalization
- Cyclic prefix correlation

### Text Processing

#### Huffman Codec

- Canonical Huffman encoding
- Ham radio token support (Q-codes, callsigns, abbreviations)
- PUA sentinel markers for Unicode safety
- Optimal compression for amateur radio text

#### ASCII Codec

- Passthrough for uncompressed text
- Full UTF-8 Unicode support
- Exact reconstruction guarantee

### Audio Features

- **CW/Morse Code Generation** - Complete International Morse Code support
- **Voice announcements** - WAV file integration for station ID
- **Pink noise generation** - Squelch triggering capability
- **WAV file I/O** - Professional 48kHz 16-bit audio

### Frame Handling

- **Robust Sync Detection** - HDLC-like pattern [55 55 55 55 AA AA 7E 7E]
- **Bit Alignment Tolerance** - 8-position scan with MSB/LSB fallback
- **Inversion Detection** - Automatic polarity correction
- **Exact Content Validation** - Zero-tolerance round-trip testing

## Command Line Options

### Global Options

- `--help` - Show help information
- `--version` - Show version information

### Transmission Options

- `-o, --output <FILE>` - Output WAV file (required)
- `-t, --text <TEXT>` - Text to transmit
- `-f, --file <FILE>` - File to transmit
- `-c, --callsign <CALL>` - Station callsign
- `-m, --modulation <MOD>` - Modulation type (bpsk, fsk, afsk, ofdm)
- `--cw-preamble` - Add CW preamble
- `--pink-noise` - Add pink noise trigger
- `--power <LEVEL>` - Transmission power level (0.0-1.0)
- `--voice-id <FILE>` - Voice announcement audio file (WAV)
- `--text-codec <CODEC>` - Text codec (`huffman`, `ascii`)

### Reception Options

- `-i, --input <FILE>` - Input WAV file (required)
- `-o, --output <FILE>` - Output text file (optional, defaults to console)
- `-m, --modulation <MOD>` - Expected modulation type
- `--auto-detect` - Automatically detect modulation
- `--text-codec <CODEC>` - Hint text codec to attempt first when decoding

### Signal Generation Options

- `-o, --output <FILE>` - Output WAV file (required)
- `-s, --signal <TYPE>` - Signal type (sine, noise, sweep, morse)
- `-f, --frequency <HZ>` - Frequency in Hz
- `-d, --duration <SEC>` - Duration in seconds

## Testing

### Comprehensive Test Suite

Run the complete test suite:

```bash
# Linux/macOS/WSL
./test_openham.sh

# Windows
test_openham.bat
```

The test suite validates:

- All 4 stable modulation schemes (BPSK, FSK, AFSK, OFDM)
- Text codecs (Huffman and ASCII)
- Round-trip testing with exact content matching
- Enhanced features (CW preambles, pink noise, voice ID)
- Auto-detection capability
- Grid testing across modulation × codec × options matrix

### Manual Testing

```bash
# Test TX/RX cycle
./target/release/openham tx -o test.wav -t "Test message" -c S56SPZ -m bpsk
./target/release/openham rx -i test.wav -m bpsk

# Test auto-detection
./target/release/openham tx -o test_auto.wav -t "Auto test" -c S56SPZ -m fsk
./target/release/openham rx -i test_auto.wav --auto-detect
```

## Technical Specifications

### Audio Parameters

- **Sample Rate**: 48 kHz (configurable)
- **Bit Depth**: 16-bit PCM
- **Channels**: Mono
- **Format**: WAV (uncompressed)

### Modulation Parameters

#### BPSK

- Symbol rate: 125 baud (configurable)
- Carrier frequency: 1500 Hz (configurable)
- Phase shift: 0°/180°

#### FSK

- Symbol rate: 125 baud (configurable)
- Mark frequency: 1615 Hz (configurable)
- Space frequency: 1385 Hz (configurable)
- Frequency shift: 230 Hz

#### AFSK

- Bell 202: 1200 baud, 1200/2200 Hz
- Bell 103: 300 baud, 1070/1270 Hz  
- VHF: 1200 baud, 1200/2200 Hz
- HF: 300 baud, 1600/1800 Hz

#### OFDM

- Subcarriers: 64 (with pilot equalization)
- Cyclic prefix: For multipath resilience
- Pilot tones: Channel equalization
- Guard interval: Inter-symbol interference protection

### Performance Characteristics

- **Test Results**: 100% pass rate for all stable modes
- **Sensitivity**: Optimized for weak signal detection
- **Bandwidth**: Efficient spectral usage
- **Throughput**: Variable based on modulation and conditions

## Troubleshooting

### Common Issues

1. **Build errors**: Ensure Rust toolchain is up to date
2. **Audio issues**: Check file paths and audio format
3. **Modulation detection**: Try manual mode selection
4. **File permissions**: Ensure read/write access to directories

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/openham tx -o output.wav -t "Debug test" -c S56SPZ -m bpsk
```

### Verbose Output

```bash
# Verbose operation information  
./target/release/openham -v tx -o output.wav -t "Verbose test" -c S56SPZ -m bpsk
```

### Stable vs Experimental Features

### ✅ Stable (100% Test Pass Rate - Ready for Use)

- **BPSK, FSK, AFSK, OFDM** - Complete TX/RX implementation with robust sync detection
- **Huffman and ASCII codecs** - Full round-trip validation with exact content matching
- **CW generation** - Complete International Morse Code support
- **Audio processing** - Professional WAV I/O (48kHz 16-bit)
- **CLI interface** - Complete command system with all stable features
- **Enhanced features** - CW preambles, pink noise triggers, voice ID, power control

### ⚠️ Experimental (Partial Implementation - Use with Caution)

- **PSK variants** - Available as `--modulation psk` with `--psk-type` options:
  - `bpsk` - Basic PSK (use `--modulation bpsk` instead for stable version)
  - `qpsk` - QPSK (TX working, RX has punctuation handling issues)
  - `8psk` - 8-PSK (TX working, RX has minor decoding issues)
  - `16psk` - 16-PSK (TX working, RX missing some characters)

- **QAM modes** - Available as `--modulation qam` with `--qam-type` options:
  - `16`, `64`, `256`, `1024` - All orders (TX working, RX sync detection failures)

- **Experimental schemes** - Available as `--modulation experimental`:
  - Research implementations (not recommended for practical use)

### Usage Recommendations

**For practical on-air use**, stick to the stable modes:

```bash
# Recommended stable modes
./target/release/openham tx -o signal.wav -t "Message" -c CALL -m bpsk
./target/release/openham tx -o signal.wav -t "Message" -c CALL -m fsk  
./target/release/openham tx -o signal.wav -t "Message" -c CALL -m afsk
./target/release/openham tx -o signal.wav -t "Message" -c CALL -m ofdm
```

**For experimental work** (test mode only):

```bash
# PSK variants (experimental)
./target/release/openham tx -o signal.wav -t "Test" -c CALL -m psk --psk-type qpsk

# QAM variants (experimental) 
./target/release/openham tx -o signal.wav -t "Test" -c CALL -m qam --qam-type 16
```

## Contributing

See `CONTRIBUTING.md` for development guidelines and contribution procedures.

## License

See `LICENSE` file for licensing information.

## Architecture

See `docs/ARCHITECTURE.md` for detailed technical architecture information.

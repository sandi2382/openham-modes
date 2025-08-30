# OpenHam Text Mode v1 Specification

## Overview

OpenHam Text Mode v1 (ohm.text.v1) is designed for reliable, low-bitrate text communication over amateur radio. It prioritizes robustness over speed, making it suitable for weak signal conditions and emergency communications.

## Design Goals

- **Robust error correction**: Reliable communication under poor conditions
- **Efficient encoding**: Compact representation of common text
- **Amateur radio compliance**: No encryption, clear identification
- **Simple implementation**: Straightforward to decode and implement
- **Extensible**: Foundation for future text mode variants

## Technical Parameters

### Symbol Rate and Modulation

- **Modulation**: BPSK (Binary Phase Shift Keying)
- **Symbol rate**: 31.25 baud (configurable: 15.625, 31.25, 62.5 baud)
- **Bandwidth**: ~62.5 Hz at 31.25 baud (Nyquist + rolloff)
- **Rolloff factor**: 0.35 (root raised cosine)

### Frame Structure

- **Frame length**: 256 bits total
- **Sync pattern**: 32 bits (Barker sequence + unique word)
- **Header**: 32 bits (frame type, sequence, flags, checksum)
- **Payload**: 128 bits (16 bytes user data)
- **FEC parity**: 64 bits (Reed-Solomon RS(32,16))

### Error Correction

- **Outer code**: Reed-Solomon RS(32,16) - can correct 8 byte errors
- **Inner code**: None (future versions may add convolutional coding)
- **Interleaving**: 8x4 block interleaver to combat burst errors

### Encoding Scheme

1. **Text compression**: Huffman coding optimized for English text
2. **Character set**: UTF-8 with fallback to ASCII
3. **Special characters**: Ham radio abbreviations and Q-codes
4. **Escapes**: Control sequences for mode switching

## Frame Format

```text
| Sync (32b) | Header (32b) | Payload (128b) | Parity (64b) |
```

### Sync Pattern

```text
0xACAFE539  // 32-bit sync word (good autocorrelation properties)
```

### Header Format

```text
Bits 0-3:   Frame Type (4 bits)
Bits 4-15:  Sequence Number (12 bits)
Bits 16-23: Flags (8 bits)
Bits 24-31: Header Checksum (8 bits)
```

### Frame Types

- `0x0`: Data frame (text content)
- `0x1`: Control frame (mode commands)
- `0x2`: Beacon frame (station identification)
- `0x3`: ARQ frame (acknowledgment/retransmission)

### Flags

- Bit 0: More fragments follow
- Bit 1: Fragmented message
- Bit 2: Priority message
- Bit 3: Broadcast (no ACK expected)
- Bit 4-7: Reserved (must be 0)

## Text Encoding

### Character Mapping

Common characters are encoded with shorter bit patterns:

```yaml
# High frequency characters (3-4 bits)
' ': '000'      # Space
'E': '001'      # Most common letter
'T': '010'
'A': '011'
'O': '100'
'I': '101'
'N': '110'
'S': '111'

# Medium frequency (5-6 bits)
# ... (full Huffman table in implementation)

# Escape sequences
'\x00': '11110000'  # UTF-8 follows
'\x01': '11110001'  # Q-code follows
'\x02': '11110010'  # Abbreviation follows
'\x03': '11110011'  # Control command
```

### Q-Code Support

Common Q-codes are encoded efficiently:

- `QRT`: End of transmission
- `QRZ`: Who is calling?
- `QTH`: Location
- `QSL`: Acknowledgment
- `QRM`: Interference
- `QRN`: Static noise

### Abbreviation Support

Ham radio abbreviations:

- `73`: Best wishes
- `88`: Love and kisses
- `CQ`: General call
- `DE`: From (this is)
- `SK`: End of contact

## Protocol Operation

### Connection Establishment

1. Transmitting station sends beacon with callsign
2. Receiving station may respond with ACK beacon
3. Data transmission begins with sequence number 0

### Data Transmission

1. Text is compressed and fragmented into 16-byte payloads
2. Each fragment is Reed-Solomon encoded
3. Frames are transmitted with progressive sequence numbers
4. Optional ARQ mode for acknowledged delivery

### Error Handling

1. Receiver attempts RS decoding on each frame
2. If uncorrectable, frame is marked as lost
3. In ARQ mode, NACK is sent for retransmission
4. In broadcast mode, lost frames are skipped

## Implementation Guidelines

### Timing Requirements

- **Symbol timing**: ±1% accuracy required
- **Frequency accuracy**: ±10 Hz at HF
- **Phase noise**: <-40 dBc at 100 Hz offset

### Receiver Requirements

- **Sync detection**: Correlate against known sync pattern
- **Frame alignment**: Bit-level synchronization
- **Carrier tracking**: PLL with ~10 Hz bandwidth
- **AGC**: Fast attack, slow decay for fading

### Software Implementation

```rust
// Example Rust implementation structure
pub struct OhmTextV1 {
    modulator: BpskModulator,
    demodulator: BpskDemodulator,  
    rs_codec: ReedSolomon,
    text_encoder: HuffmanEncoder,
    interleaver: BlockInterleaver,
}
```

## Testing and Validation

### Test Vectors

Reference test cases are provided in `/specimen/ohm-text-v1/`:

- `test-vectors.json`: Encoding/decoding test cases
- `reference-waveform.wav`: 1000 Hz carrier, 48 kHz sample rate
- `weak-signal.wav`: -10 dB SNR test case

### Conformance Tests

1. **Encoding compliance**: Generate known test patterns
2. **Decoding accuracy**: Process reference waveforms
3. **Error correction**: Verify RS performance with errors
4. **Timing tolerance**: Test with frequency/timing offsets

## On-Air Protocol

### Station Identification

Every transmission session must begin with:

1. CW identification of transmitting station
2. Spoken announcement: "Experimental digital transmission using OpenHam Text Mode"
3. First data frame contains station callsign

### Frequency Coordination

- Use designated digital sub-bands
- Coordinate with local amateurs for testing
- Monitor frequency before transmitting
- Use minimum necessary power

## Future Enhancements

### Version 1.1 (Planned)

- Convolutional inner coding for additional protection
- Variable data rates (adaptive to conditions)
- Improved text compression for non-English languages
- Binary file transfer capability

### Version 2.0 (Roadmap)

- OFDM modulation for multipath resilience
- Forward error correction with turbo codes
- Dynamic adaptive protocols
- Integration with digital voice modes

## References

- ITU-R M.1677: Error correction techniques for HF data transmission
- ARRL Digital Communication Protocol Specification
- Reed-Solomon Tutorial: <https://en.wikiversity.org/reed_solomon>
- Amateur Radio Emergency Data Network (AREDN) specifications

## Appendices

### A. Huffman Code Table

[Full character encoding table - see implementation files]

### B. Reed-Solomon Generator Polynomial

```text
g(x) = (x - α^0)(x - α^1)...(x - α^15)
where α is a primitive element of GF(256)
```

### C. Test Vector Examples

```json
{
  "test_case_1": {
    "input_text": "CQ CQ DE W1AW",
    "compressed_bits": "001110010110...",
    "rs_encoded": "DEADBEEF...",
    "modulated_samples": [0.1, -0.3, 0.8, ...]
  }
}
```

---

**Status**: Draft specification - subject to change  
**Next Review**: 2025-09-29  
**Contact**: openham-modes@example.com
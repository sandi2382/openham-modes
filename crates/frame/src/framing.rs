//! Mode-agnostic frame acquisition for continuous streams.
//!
//! On a live radio feed the receiver gets a continuous bit stream and a
//! transmission begins at an arbitrary instant, preceded by silence, noise, or
//! other signals. The receiver therefore cannot assume a frame starts at the
//! first bit — it must *acquire* the frame by locating a known marker.
//!
//! This layer is deliberately independent of any modulation:
//! - **Transmit:** wrap the serialized [`Frame`] bytes with a preamble (for
//!   timing/AGC) and a sync word, then hand the bytes to a modulator.
//! - **Receive:** take the demodulated **bit** stream, correlate the sync word
//!   at every bit offset (tolerating a few bit errors), byte-align from there,
//!   and slice out each frame using its header length.
//!
//! Working at bit granularity is what makes acquisition robust: after timing
//! recovery the demodulator yields correct symbols, but the frame can begin at
//! any bit position in that stream.

use crate::frame::Frame;

/// Preamble: alternating bits (`0x55 = 01010101`) give the demodulator a clean
/// run to settle symbol timing and gain before the sync word arrives.
pub const PREAMBLE: [u8; 4] = [0x55, 0x55, 0x55, 0x55];

/// Frame sync marker: the 32-bit CCSDS Attached Sync Marker, a standard frame
/// sync pattern chosen for its strong autocorrelation in noise.
pub const SYNC_WORD: [u8; 4] = [0x1A, 0xCF, 0xFC, 0x1D];

/// Default tolerance: a detected sync may differ from [`SYNC_WORD`] in at most
/// this many of its 32 bits. Small enough to avoid false locks, large enough to
/// ride through a noisy channel.
pub const DEFAULT_MAX_SYNC_ERRORS: u32 = 4;

/// Wrap serialized frame bytes with the preamble and sync word for transmission.
pub fn add_preamble_sync(frame_bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(PREAMBLE.len() + SYNC_WORD.len() + frame_bytes.len());
    out.extend_from_slice(&PREAMBLE);
    out.extend_from_slice(&SYNC_WORD);
    out.extend_from_slice(frame_bytes);
    out
}

/// Expand bytes to a bit vector, MSB first.
pub fn bytes_to_bits(bytes: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &b in bytes {
        for i in (0..8).rev() {
            bits.push((b >> i) & 1);
        }
    }
    bits
}

/// Pack a bit vector (MSB first) into bytes. A trailing partial byte is dropped.
pub fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(bits.len() / 8);
    for chunk in bits.chunks_exact(8) {
        let mut byte = 0u8;
        for (i, &bit) in chunk.iter().enumerate() {
            byte |= (bit & 1) << (7 - i);
        }
        bytes.push(byte);
    }
    bytes
}

/// Locates frames in a demodulated bit stream by correlating the sync word.
#[derive(Debug, Clone)]
pub struct Acquisition {
    max_sync_errors: u32,
}

impl Default for Acquisition {
    fn default() -> Self {
        Self {
            max_sync_errors: DEFAULT_MAX_SYNC_ERRORS,
        }
    }
}

impl Acquisition {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set how many bit errors a sync match may contain (out of 32).
    pub fn with_max_sync_errors(mut self, max: u32) -> Self {
        self.max_sync_errors = max;
        self
    }

    /// Scan a demodulated bit stream and return every valid frame found.
    ///
    /// At each bit offset the sync word is correlated; on a match (within the
    /// error budget) the following bits are byte-aligned and parsed as a
    /// [`Frame`]. A frame is accepted only if its header checksum validates, so
    /// false sync locks are rejected. Scanning then resumes after the accepted
    /// frame.
    pub fn find_frames(&self, bits: &[u8]) -> Vec<Frame> {
        let sync_bits = bytes_to_bits(&SYNC_WORD);
        let sync_len = sync_bits.len();
        // Also match the bitwise-inverse sync. A coherent demodulator (e.g. BPSK)
        // can recover the bit stream with a 180° phase ambiguity that inverts
        // every bit; detecting the inverted sync lets us recover those frames by
        // inverting the payload bits before parsing.
        let inv_sync: Vec<u8> = sync_bits.iter().map(|b| b ^ 1).collect();
        let mut frames = Vec::new();

        let mut i = 0usize;
        while i + sync_len <= bits.len() {
            let window = &bits[i..i + sync_len];
            let mismatches = sync_bits.iter().zip(window).filter(|(a, b)| a != b).count() as u32;
            let inv_mismatches =
                inv_sync.iter().zip(window).filter(|(a, b)| a != b).count() as u32;

            let inverted = if mismatches <= self.max_sync_errors {
                Some(false)
            } else if inv_mismatches <= self.max_sync_errors {
                Some(true)
            } else {
                None
            };

            if let Some(invert) = inverted {
                let start = i + sync_len;
                let frame_bytes = if invert {
                    let flipped: Vec<u8> = bits[start..].iter().map(|b| b ^ 1).collect();
                    bits_to_bytes(&flipped)
                } else {
                    bits_to_bytes(&bits[start..])
                };
                if let Ok(frame) = Frame::from_bytes(&frame_bytes) {
                    // Advance past the bits this frame consumed and keep scanning.
                    i = start + frame.total_size() * 8;
                    frames.push(frame);
                    continue;
                }
            }
            i += 1;
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{frame_flags, frame_types};

    fn make_frame(payload: &[u8]) -> Frame {
        Frame::new(frame_types::DATA, 9, payload.to_vec(), frame_flags::NONE)
    }

    #[test]
    fn bits_round_trip() {
        let bytes = vec![0x1A, 0xCF, 0xFC, 0x1D, 0x00, 0xFF];
        assert_eq!(bits_to_bytes(&bytes_to_bits(&bytes)), bytes);
    }

    #[test]
    fn add_preamble_sync_prepends_marker() {
        let framed = add_preamble_sync(&[0xDE, 0xAD]);
        assert_eq!(&framed[..4], &PREAMBLE);
        assert_eq!(&framed[4..8], &SYNC_WORD);
        assert_eq!(&framed[8..], &[0xDE, 0xAD]);
    }

    #[test]
    fn acquires_frame_at_arbitrary_bit_offset() {
        let payload = b"OPENHAM ACQ TEST";
        let frame = make_frame(payload);
        let framed = add_preamble_sync(&frame.to_bytes());
        let mut bits = Vec::new();

        // Arbitrary lead-in garbage that is NOT a whole number of bytes, so the
        // frame starts at a non-byte-aligned bit position.
        bits.extend_from_slice(&[1, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0]); // 11 junk bits
        bits.extend(bytes_to_bits(&framed));
        bits.extend_from_slice(&[0, 1, 1, 0, 1]); // trailing junk

        let found = Acquisition::new().find_frames(&bits);
        assert_eq!(found.len(), 1, "should find exactly one frame");
        assert_eq!(found[0].payload, payload);
    }

    #[test]
    fn tolerates_bit_errors_in_sync() {
        let frame = make_frame(b"errors ok");
        let framed = add_preamble_sync(&frame.to_bytes());
        let mut bits = bytes_to_bits(&framed);
        // Flip 3 bits inside the sync word (preamble is 32 bits, sync next 32).
        for idx in [33, 40, 55] {
            bits[idx] ^= 1;
        }
        let found = Acquisition::new().find_frames(&bits);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].payload, b"errors ok");
    }

    #[test]
    fn rejects_when_sync_too_corrupted() {
        let frame = make_frame(b"nope");
        let framed = add_preamble_sync(&frame.to_bytes());
        let mut bits = bytes_to_bits(&framed);
        // Corrupt many sync bits -> beyond tolerance -> no lock.
        for idx in 32..48 {
            bits[idx] ^= 1;
        }
        let found = Acquisition::new().with_max_sync_errors(4).find_frames(&bits);
        assert!(found.is_empty());
    }

    #[test]
    fn acquires_inverted_frame() {
        // A 180° phase ambiguity inverts every bit; acquisition must still work.
        let frame = make_frame(b"inverted payload");
        let framed = add_preamble_sync(&frame.to_bytes());
        let bits: Vec<u8> = bytes_to_bits(&framed).iter().map(|b| b ^ 1).collect();
        let found = Acquisition::new().find_frames(&bits);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].payload, b"inverted payload");
    }

    #[test]
    fn finds_multiple_frames_in_one_stream() {
        let f1 = make_frame(b"first");
        let f2 = make_frame(b"second frame");
        let mut bits = Vec::new();
        bits.extend_from_slice(&[1, 1, 0]); // junk
        bits.extend(bytes_to_bits(&add_preamble_sync(&f1.to_bytes())));
        bits.extend_from_slice(&[0, 1]); // inter-frame junk
        bits.extend(bytes_to_bits(&add_preamble_sync(&f2.to_bytes())));

        let found = Acquisition::new().find_frames(&bits);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].payload, b"first");
        assert_eq!(found[1].payload, b"second frame");
    }
}

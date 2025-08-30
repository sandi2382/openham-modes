//! Frame structure and management

use crate::{FrameError, Result};
use serde::{Deserialize, Serialize};

/// Frame header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameHeader {
    /// Frame type identifier
    pub frame_type: u8,
    /// Sequence number
    pub sequence: u16,
    /// Payload length in bytes
    pub payload_length: u16,
    /// Frame flags
    pub flags: u8,
    /// Header checksum
    pub checksum: u16,
}

impl FrameHeader {
    /// Size of the frame header in bytes
    pub const SIZE: usize = 8;
    
    /// Create a new frame header
    pub fn new(frame_type: u8, sequence: u16, payload_length: u16, flags: u8) -> Self {
        let mut header = Self {
            frame_type,
            sequence,
            payload_length,
            flags,
            checksum: 0,
        };
        header.checksum = header.calculate_checksum();
        header
    }
    
    /// Calculate header checksum
    fn calculate_checksum(&self) -> u16 {
        // Simple checksum calculation (CRC16 would be better)
        let mut sum = 0u16;
        sum = sum.wrapping_add(self.frame_type as u16);
        sum = sum.wrapping_add(self.sequence);
        sum = sum.wrapping_add(self.payload_length);
        sum = sum.wrapping_add(self.flags as u16);
        !sum // One's complement
    }
    
    /// Validate header checksum
    pub fn validate_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }
    
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.push(self.frame_type);
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.payload_length.to_be_bytes());
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.checksum.to_be_bytes());
        bytes
    }
    
    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(FrameError::InvalidFormat {
                msg: format!("Header too short: {} bytes", bytes.len()),
            });
        }
        
        let frame_type = bytes[0];
        let sequence = u16::from_be_bytes([bytes[1], bytes[2]]);
        let payload_length = u16::from_be_bytes([bytes[3], bytes[4]]);
        let flags = bytes[5];
        let checksum = u16::from_be_bytes([bytes[6], bytes[7]]);
        
        let header = Self {
            frame_type,
            sequence,
            payload_length,
            flags,
            checksum,
        };
        
        if !header.validate_checksum() {
            return Err(FrameError::InvalidFormat {
                msg: "Header checksum mismatch".to_string(),
            });
        }
        
        Ok(header)
    }
}

/// Complete frame with header and payload
#[derive(Debug, Clone)]
pub struct Frame {
    pub header: FrameHeader,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Create a new frame
    pub fn new(frame_type: u8, sequence: u16, payload: Vec<u8>, flags: u8) -> Self {
        let header = FrameHeader::new(frame_type, sequence, payload.len() as u16, flags);
        Self { header, payload }
    }
    
    /// Get total frame size in bytes
    pub fn total_size(&self) -> usize {
        FrameHeader::SIZE + self.payload.len()
    }
    
    /// Serialize frame to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        bytes
    }
    
    /// Deserialize frame from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < FrameHeader::SIZE {
            return Err(FrameError::InvalidFormat {
                msg: "Frame too short for header".to_string(),
            });
        }
        
        let header = FrameHeader::from_bytes(&bytes[..FrameHeader::SIZE])?;
        
        let expected_total_size = FrameHeader::SIZE + header.payload_length as usize;
        if bytes.len() < expected_total_size {
            return Err(FrameError::SizeMismatch {
                expected: expected_total_size,
                actual: bytes.len(),
            });
        }
        
        let payload = bytes[FrameHeader::SIZE..expected_total_size].to_vec();
        
        Ok(Self { header, payload })
    }
}

/// Frame builder for constructing frames with various options
pub struct FrameBuilder {
    frame_type: u8,
    sequence: u16,
    flags: u8,
}

impl FrameBuilder {
    /// Create a new frame builder
    pub fn new(frame_type: u8) -> Self {
        Self {
            frame_type,
            sequence: 0,
            flags: 0,
        }
    }
    
    /// Set sequence number
    pub fn sequence(mut self, sequence: u16) -> Self {
        self.sequence = sequence;
        self
    }
    
    /// Set frame flags
    pub fn flags(mut self, flags: u8) -> Self {
        self.flags = flags;
        self
    }
    
    /// Build frame with payload
    pub fn build(self, payload: Vec<u8>) -> Frame {
        Frame::new(self.frame_type, self.sequence, payload, self.flags)
    }
}

/// Frame type constants
pub mod frame_types {
    pub const DATA: u8 = 0x01;
    pub const CONTROL: u8 = 0x02;
    pub const KEEPALIVE: u8 = 0x03;
    pub const ACK: u8 = 0x04;
    pub const NACK: u8 = 0x05;
}

/// Frame flag constants
pub mod frame_flags {
    pub const NONE: u8 = 0x00;
    pub const MORE_FRAGMENTS: u8 = 0x01;
    pub const PRIORITY: u8 = 0x02;
    pub const ENCRYPTED: u8 = 0x04; // Should not be used in amateur radio
    pub const COMPRESSED: u8 = 0x08;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_creation_and_validation() {
        let header = FrameHeader::new(frame_types::DATA, 123, 456, frame_flags::NONE);
        assert_eq!(header.frame_type, frame_types::DATA);
        assert_eq!(header.sequence, 123);
        assert_eq!(header.payload_length, 456);
        assert_eq!(header.flags, frame_flags::NONE);
        assert!(header.validate_checksum());
    }

    #[test]
    fn test_header_serialization() {
        let header = FrameHeader::new(frame_types::DATA, 123, 456, frame_flags::NONE);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), FrameHeader::SIZE);
        
        let recovered = FrameHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header.frame_type, recovered.frame_type);
        assert_eq!(header.sequence, recovered.sequence);
        assert_eq!(header.payload_length, recovered.payload_length);
        assert_eq!(header.flags, recovered.flags);
        assert_eq!(header.checksum, recovered.checksum);
    }

    #[test]
    fn test_frame_creation_and_serialization() {
        let payload = b"Hello, World!".to_vec();
        let frame = Frame::new(frame_types::DATA, 42, payload.clone(), frame_flags::NONE);
        
        assert_eq!(frame.header.frame_type, frame_types::DATA);
        assert_eq!(frame.header.sequence, 42);
        assert_eq!(frame.header.payload_length, payload.len() as u16);
        assert_eq!(frame.payload, payload);
        
        let bytes = frame.to_bytes();
        let recovered = Frame::from_bytes(&bytes).unwrap();
        
        assert_eq!(frame.header.frame_type, recovered.header.frame_type);
        assert_eq!(frame.header.sequence, recovered.header.sequence);
        assert_eq!(frame.payload, recovered.payload);
    }

    #[test]
    fn test_frame_builder() {
        let payload = b"Test payload".to_vec();
        let frame = FrameBuilder::new(frame_types::CONTROL)
            .sequence(100)
            .flags(frame_flags::PRIORITY)
            .build(payload.clone());
        
        assert_eq!(frame.header.frame_type, frame_types::CONTROL);
        assert_eq!(frame.header.sequence, 100);
        assert_eq!(frame.header.flags, frame_flags::PRIORITY);
        assert_eq!(frame.payload, payload);
    }
}
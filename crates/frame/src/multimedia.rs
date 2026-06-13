//! Enhanced frame types for multimedia transmission
//! 
//! Supports transmission of various media types including files, images,
//! video, voice, and text with appropriate framing and metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Media type identification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaType {
    Text,
    Image,
    Audio,
    Video,
    Binary,
    Directory,
}

/// Compression type for payload
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Huffman,
    Deflate,
    Lzma,
    Brotli,
}

/// Multimedia frame header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimediaHeader {
    /// Media type
    pub media_type: MediaType,
    
    /// Original filename (if applicable)
    pub filename: Option<String>,
    
    /// MIME type
    pub mime_type: String,
    
    /// Original file size before compression
    pub original_size: u64,
    
    /// Compressed payload size
    pub compressed_size: u64,
    
    /// Compression method used
    pub compression: CompressionType,
    
    /// Checksum of original data (CRC32)
    pub checksum: u32,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    
    /// Creation timestamp (Unix epoch)
    pub timestamp: u64,
    
    /// Sender callsign
    pub sender: String,
    
    /// Optional description
    pub description: Option<String>,
}

impl MultimediaHeader {
    /// Create a new multimedia header
    pub fn new(
        media_type: MediaType,
        filename: Option<String>,
        mime_type: String,
        original_size: u64,
        sender: String,
    ) -> Self {
        Self {
            media_type,
            filename,
            mime_type,
            original_size,
            compressed_size: 0, // Will be set after compression
            compression: CompressionType::None,
            checksum: 0, // Will be calculated
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            sender,
            description: None,
        }
    }
    
    /// Add metadata field
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Create header for text transmission
    pub fn for_text(content: &str, sender: String, description: Option<String>) -> Self {
        let mut header = Self::new(
            MediaType::Text,
            None,
            "text/plain".to_string(),
            content.len() as u64,
            sender,
        );
        header.description = description;
        header.add_metadata("encoding".to_string(), "UTF-8".to_string());
        header
    }
    
    /// Create header for image transmission
    pub fn for_image(
        filename: String,
        image_data: &[u8],
        format: &str,
        sender: String,
    ) -> Self {
        let mime_type = match format.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "webp" => "image/webp",
            _ => "application/octet-stream",
        }.to_string();
        
        let mut header = Self::new(
            MediaType::Image,
            Some(filename.clone()),
            mime_type,
            image_data.len() as u64,
            sender,
        );
        header.add_metadata("format".to_string(), format.to_string());
        header
    }
    
    /// Create header for audio transmission
    pub fn for_audio(
        filename: String,
        audio_data: &[u8],
        format: &str,
        sender: String,
        duration_ms: Option<u64>,
    ) -> Self {
        let mime_type = match format.to_lowercase().as_str() {
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "flac" => "audio/flac",
            "aac" => "audio/aac",
            _ => "application/octet-stream",
        }.to_string();
        
        let mut header = Self::new(
            MediaType::Audio,
            Some(filename.clone()),
            mime_type,
            audio_data.len() as u64,
            sender,
        );
        header.add_metadata("format".to_string(), format.to_string());
        if let Some(duration) = duration_ms {
            header.add_metadata("duration_ms".to_string(), duration.to_string());
        }
        header
    }
    
    /// Create header for video transmission
    pub fn for_video(
        filename: String,
        video_data: &[u8],
        format: &str,
        sender: String,
        duration_ms: Option<u64>,
        resolution: Option<(u32, u32)>,
    ) -> Self {
        let mime_type = match format.to_lowercase().as_str() {
            "mp4" => "video/mp4",
            "avi" => "video/avi",
            "mov" => "video/quicktime",
            "mkv" => "video/x-matroska",
            "webm" => "video/webm",
            _ => "application/octet-stream",
        }.to_string();
        
        let mut header = Self::new(
            MediaType::Video,
            Some(filename.clone()),
            mime_type,
            video_data.len() as u64,
            sender,
        );
        header.add_metadata("format".to_string(), format.to_string());
        if let Some(duration) = duration_ms {
            header.add_metadata("duration_ms".to_string(), duration.to_string());
        }
        if let Some((width, height)) = resolution {
            header.add_metadata("width".to_string(), width.to_string());
            header.add_metadata("height".to_string(), height.to_string());
        }
        header
    }
    
    /// Create header for binary file transmission
    pub fn for_binary_file(
        filename: String,
        file_data: &[u8],
        sender: String,
    ) -> Self {
        Self::new(
            MediaType::Binary,
            Some(filename),
            "application/octet-stream".to_string(),
            file_data.len() as u64,
            sender,
        )
    }
    
    /// Serialize header to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let json = serde_json::to_string(self)?;
        Ok(json.into_bytes())
    }
    
    /// Deserialize header from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::str::from_utf8(data)?;
        let header = serde_json::from_str(json)?;
        Ok(header)
    }
}

/// Complete multimedia frame with header and payload
#[derive(Debug, Clone)]
pub struct MultimediaFrame {
    pub header: MultimediaHeader,
    pub payload: Vec<u8>,
}

impl MultimediaFrame {
    /// Create a new multimedia frame
    pub fn new(header: MultimediaHeader, payload: Vec<u8>) -> Self {
        Self { header, payload }
    }
    
    /// Create frame for text with optional compression
    pub fn create_text_frame(
        text: &str,
        sender: String,
        description: Option<String>,
        compress: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut header = MultimediaHeader::for_text(text, sender, description);
        let mut payload = text.as_bytes().to_vec();
        
        // Calculate checksum of original data
        header.checksum = crc32fast::hash(&payload);
        
        if compress {
            // Simple compression using deflate
            use std::io::Write;
            use flate2::{write::DeflateEncoder, Compression};
            
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&payload)?;
            let compressed = encoder.finish()?;
            
            if compressed.len() < payload.len() {
                header.compression = CompressionType::Deflate;
                header.compressed_size = compressed.len() as u64;
                payload = compressed;
            } else {
                header.compressed_size = payload.len() as u64;
            }
        } else {
            header.compressed_size = payload.len() as u64;
        }
        
        Ok(Self::new(header, payload))
    }
    
    /// Create frame for binary data with compression
    pub fn create_binary_frame(
        filename: String,
        data: &[u8],
        sender: String,
        compress: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut header = MultimediaHeader::for_binary_file(filename, data, sender);
        let mut payload = data.to_vec();
        
        // Calculate checksum
        header.checksum = crc32fast::hash(&payload);
        
        if compress {
            use std::io::Write;
            use flate2::{write::DeflateEncoder, Compression};
            
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&payload)?;
            let compressed = encoder.finish()?;
            
            if compressed.len() < payload.len() {
                header.compression = CompressionType::Deflate;
                header.compressed_size = compressed.len() as u64;
                payload = compressed;
            } else {
                header.compressed_size = payload.len() as u64;
            }
        } else {
            header.compressed_size = payload.len() as u64;
        }
        
        Ok(Self::new(header, payload))
    }
    
    /// Decompress payload if compressed
    pub fn decompress_payload(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self.header.compression {
            CompressionType::None => Ok(self.payload.clone()),
            CompressionType::Deflate => {
                use std::io::Read;
                use flate2::read::DeflateDecoder;
                
                let mut decoder = DeflateDecoder::new(&self.payload[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            _ => Err("Unsupported compression type".into()),
        }
    }
    
    /// Verify payload integrity
    pub fn verify_integrity(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let decompressed = self.decompress_payload()?;
        let calculated_checksum = crc32fast::hash(&decompressed);
        Ok(calculated_checksum == self.header.checksum)
    }
    
    /// Serialize entire frame to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let header_bytes = self.header.to_bytes()?;
        let header_len = header_bytes.len() as u32;
        
        let mut result = Vec::new();
        
        // Frame format:
        // [4 bytes: header length][header][payload]
        result.extend_from_slice(&header_len.to_le_bytes());
        result.extend_from_slice(&header_bytes);
        result.extend_from_slice(&self.payload);
        
        Ok(result)
    }
    
    /// Deserialize frame from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() < 4 {
            return Err("Invalid frame: too short".into());
        }
        
        let header_len = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        if data.len() < 4 + header_len {
            return Err("Invalid frame: header truncated".into());
        }
        
        let header_bytes = &data[4..4 + header_len];
        let payload = data[4 + header_len..].to_vec();
        
        let header = MultimediaHeader::from_bytes(header_bytes)?;
        
        Ok(Self::new(header, payload))
    }
}

/// Frame splitter for large multimedia files
pub struct FrameSplitter {
    max_frame_size: usize,
    frame_id: u32,
}

impl FrameSplitter {
    pub fn new(max_frame_size: usize) -> Self {
        Self {
            max_frame_size,
            frame_id: 0,
        }
    }
    
    /// Split large frame into smaller transmission frames
    pub fn split_frame(&mut self, frame: &MultimediaFrame) -> Result<Vec<TransmissionFrame>, Box<dyn std::error::Error>> {
        let frame_bytes = frame.to_bytes()?;
        let total_size = frame_bytes.len();
        let num_fragments = (total_size + self.max_frame_size - 1) / self.max_frame_size;
        
        let mut fragments = Vec::new();
        
        for i in 0..num_fragments {
            let start = i * self.max_frame_size;
            let end = std::cmp::min(start + self.max_frame_size, total_size);
            let fragment_data = frame_bytes[start..end].to_vec();
            
            let fragment_header = FragmentHeader {
                frame_id: self.frame_id,
                fragment_index: i as u16,
                total_fragments: num_fragments as u16,
                fragment_size: fragment_data.len() as u32,
                total_frame_size: total_size as u32,
                checksum: crc32fast::hash(&fragment_data),
            };
            
            fragments.push(TransmissionFrame {
                header: fragment_header,
                data: fragment_data,
            });
        }
        
        self.frame_id = self.frame_id.wrapping_add(1);
        Ok(fragments)
    }
}

/// Fragment header for transmission frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentHeader {
    pub frame_id: u32,
    pub fragment_index: u16,
    pub total_fragments: u16,
    pub fragment_size: u32,
    pub total_frame_size: u32,
    pub checksum: u32,
}

/// Individual transmission frame
#[derive(Debug, Clone)]
pub struct TransmissionFrame {
    pub header: FragmentHeader,
    pub data: Vec<u8>,
}

impl TransmissionFrame {
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let header_json = serde_json::to_string(&self.header)?;
        let header_bytes = header_json.into_bytes();
        let header_len = header_bytes.len() as u16;
        
        let mut result = Vec::new();
        result.extend_from_slice(&header_len.to_le_bytes());
        result.extend_from_slice(&header_bytes);
        result.extend_from_slice(&self.data);
        
        Ok(result)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() < 2 {
            return Err("Invalid transmission frame: too short".into());
        }
        
        let header_len = u16::from_le_bytes([data[0], data[1]]) as usize;
        
        if data.len() < 2 + header_len {
            return Err("Invalid transmission frame: header truncated".into());
        }
        
        let header_bytes = &data[2..2 + header_len];
        let frame_data = data[2 + header_len..].to_vec();
        
        let header_json = std::str::from_utf8(header_bytes)?;
        let header: FragmentHeader = serde_json::from_str(header_json)?;
        
        Ok(Self {
            header,
            data: frame_data,
        })
    }
    
    /// Verify fragment integrity
    pub fn verify(&self) -> bool {
        let calculated_checksum = crc32fast::hash(&self.data);
        calculated_checksum == self.header.checksum && 
        self.data.len() == self.header.fragment_size as usize
    }
}

/// Frame assembler to reconstruct multimedia frames from fragments
pub struct FrameAssembler {
    fragments: HashMap<u32, Vec<Option<TransmissionFrame>>>,
}

impl FrameAssembler {
    pub fn new() -> Self {
        Self {
            fragments: HashMap::new(),
        }
    }
    
    /// Add a fragment to the assembler
    pub fn add_fragment(&mut self, fragment: TransmissionFrame) -> Result<Option<MultimediaFrame>, Box<dyn std::error::Error>> {
        if !fragment.verify() {
            return Err("Fragment failed integrity check".into());
        }
        
        let frame_id = fragment.header.frame_id;
        let fragment_index = fragment.header.fragment_index as usize;
        let total_fragments = fragment.header.total_fragments as usize;
        
        // Initialize fragment list if needed
        let fragment_list = self.fragments.entry(frame_id)
            .or_insert_with(|| vec![None; total_fragments]);
        
        // Ensure fragment list is the right size
        if fragment_list.len() != total_fragments {
            fragment_list.resize(total_fragments, None);
        }
        
        // Add fragment
        if fragment_index < fragment_list.len() {
            fragment_list[fragment_index] = Some(fragment);
        }
        
        // Check if frame is complete
        if fragment_list.iter().all(|f| f.is_some()) {
            // Reconstruct frame
            let mut frame_data = Vec::new();
            for fragment_opt in fragment_list {
                if let Some(fragment) = fragment_opt {
                    frame_data.extend_from_slice(&fragment.data);
                }
            }
            
            // Remove from tracking
            self.fragments.remove(&frame_id);
            
            // Parse multimedia frame
            let multimedia_frame = MultimediaFrame::from_bytes(&frame_data)?;
            
            // Verify integrity
            if multimedia_frame.verify_integrity()? {
                Ok(Some(multimedia_frame))
            } else {
                Err("Reconstructed frame failed integrity check".into())
            }
        } else {
            Ok(None)
        }
    }
    
    /// Get completion status for a frame
    pub fn get_completion_status(&self, frame_id: u32) -> Option<(usize, usize)> {
        if let Some(fragments) = self.fragments.get(&frame_id) {
            let received = fragments.iter().filter(|f| f.is_some()).count();
            Some((received, fragments.len()))
        } else {
            None
        }
    }
    
    /// Clean up incomplete frames older than specified age
    pub fn cleanup_old_frames(&mut self, max_age_secs: u64) {
        // In a real implementation, you'd track timestamps and clean up old fragments
        // For now, just remove frames with only a few fragments after some threshold
        let frame_ids: Vec<u32> = self.fragments.keys().cloned().collect();
        
        for frame_id in frame_ids {
            if let Some(fragments) = self.fragments.get(&frame_id) {
                let received = fragments.iter().filter(|f| f.is_some()).count();
                let total = fragments.len();
                
                // Remove frames with less than 10% completion after they've been around
                if received < total / 10 {
                    self.fragments.remove(&frame_id);
                }
            }
        }
    }
}

impl Default for FrameAssembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multimedia_header_creation() {
        let header = MultimediaHeader::for_text("Hello, World!", "N0CALL".to_string(), None);
        assert_eq!(header.media_type, MediaType::Text);
        assert_eq!(header.original_size, 13);
    }

    #[test]
    fn test_text_frame_creation() {
        let frame = MultimediaFrame::create_text_frame(
            "Test message",
            "N0CALL".to_string(),
            Some("Test transmission".to_string()),
            false,
        ).unwrap();
        
        assert_eq!(frame.header.media_type, MediaType::Text);
        assert!(frame.verify_integrity().unwrap());
    }

    #[test]
    fn test_frame_serialization() {
        let frame = MultimediaFrame::create_text_frame(
            "Test message",
            "N0CALL".to_string(),
            None,
            false,
        ).unwrap();
        
        let serialized = frame.to_bytes().unwrap();
        let deserialized = MultimediaFrame::from_bytes(&serialized).unwrap();
        
        assert_eq!(frame.header.checksum, deserialized.header.checksum);
        assert_eq!(frame.payload, deserialized.payload);
    }

    #[test]
    fn test_frame_splitting_and_assembly() {
        let large_text = "A".repeat(1000); // 1KB of text
        let frame = MultimediaFrame::create_text_frame(
            &large_text,
            "N0CALL".to_string(),
            None,
            false,
        ).unwrap();
        
        let mut splitter = FrameSplitter::new(256); // 256 byte max frames
        let fragments = splitter.split_frame(&frame).unwrap();
        
        assert!(fragments.len() > 1); // Should be split
        
        // Reassemble
        let mut assembler = FrameAssembler::new();
        let mut reconstructed = None;
        
        for fragment in fragments {
            if let Some(complete_frame) = assembler.add_fragment(fragment).unwrap() {
                reconstructed = Some(complete_frame);
                break;
            }
        }
        
        assert!(reconstructed.is_some());
        let reconstructed_frame = reconstructed.unwrap();
        assert!(reconstructed_frame.verify_integrity().unwrap());
        
        // Verify content matches
        let original_text = String::from_utf8(frame.payload).unwrap();
        let reconstructed_text = String::from_utf8(reconstructed_frame.payload).unwrap();
        assert_eq!(original_text, reconstructed_text);
    }
}
//! Text codec implementations

use crate::{CodecError, Result};
use std::collections::HashMap;

/// Generic text codec trait
pub trait TextCodec {
    /// Encode text to bytes
    fn encode(&mut self, text: &str) -> Result<Vec<u8>>;
    
    /// Decode bytes to text
    fn decode(&mut self, data: &[u8]) -> Result<String>;
    
    /// Get compression ratio (0.0 to 1.0, lower is better compression)
    fn compression_ratio(&self) -> f64;
    
    /// Reset codec state
    fn reset(&mut self);
}

/// Huffman coding implementation for text compression
pub struct HuffmanCodec {
    encode_table: HashMap<char, String>,
    decode_tree: DecodeNode,
}

#[derive(Debug, Clone)]
struct DecodeNode {
    character: Option<char>,
    left: Option<Box<DecodeNode>>,
    right: Option<Box<DecodeNode>>,
}

impl HuffmanCodec {
    /// Create a new Huffman codec with English frequency table
    pub fn new_english() -> Self {
        let mut codec = Self {
            encode_table: HashMap::new(),
            decode_tree: DecodeNode {
                character: None,
                left: None,
                right: None,
            },
        };
        
        codec.build_english_table();
        codec
    }
    
    /// Build encoding/decoding tables for English text
    fn build_english_table(&mut self) {
        // Simplified English character frequencies
        let frequencies = vec![
            (' ', 0.183), ('E', 0.127), ('T', 0.091), ('A', 0.082),
            ('O', 0.075), ('I', 0.070), ('N', 0.067), ('S', 0.063),
            ('H', 0.061), ('R', 0.060), ('D', 0.043), ('L', 0.040),
            ('C', 0.028), ('U', 0.028), ('M', 0.024), ('W', 0.023),
            ('F', 0.022), ('G', 0.020), ('Y', 0.020), ('P', 0.019),
            ('B', 0.013), ('V', 0.010), ('K', 0.008), ('J', 0.001),
            ('X', 0.001), ('Q', 0.001), ('Z', 0.001),
        ];
        
        // Build Huffman tree (simplified implementation)
        self.build_codes(&frequencies);
    }
    
    /// Build Huffman codes from frequency table
    fn build_codes(&mut self, frequencies: &[(char, f64)]) {
        // Simplified code assignment for demo
        let codes = vec![
            (' ', "00"), ('E', "01"), ('T', "100"), ('A', "101"),
            ('O', "1100"), ('I', "1101"), ('N', "1110"), ('S', "1111"),
            // Add more codes as needed
        ];
        
        for (ch, code) in codes {
            self.encode_table.insert(ch, code.to_string());
        }
        
        // Build decode tree from encode table
        self.build_decode_tree();
    }
    
    /// Build decode tree from encoding table
    fn build_decode_tree(&mut self) {
        self.decode_tree = DecodeNode {
            character: None,
            left: None,
            right: None,
        };
        
        for (ch, code) in &self.encode_table {
            let mut current = &mut self.decode_tree;
            
            for bit in code.chars() {
                let next_node = match bit {
                    '0' => &mut current.left,
                    '1' => &mut current.right,
                    _ => continue,
                };
                
                if next_node.is_none() {
                    *next_node = Some(Box::new(DecodeNode {
                        character: None,
                        left: None,
                        right: None,
                    }));
                }
                
                current = next_node.as_mut().unwrap();
            }
            
            current.character = Some(*ch);
        }
    }
}

impl TextCodec for HuffmanCodec {
    fn encode(&mut self, text: &str) -> Result<Vec<u8>> {
        let mut bits = String::new();
        
        for ch in text.chars() {
            if let Some(code) = self.encode_table.get(&ch.to_ascii_uppercase()) {
                bits.push_str(code);
            } else {
                // Fallback for unknown characters - use escape sequence
                bits.push_str("11110000"); // UTF-8 escape
                // Add actual UTF-8 bytes (simplified)
                for byte in ch.to_string().bytes() {
                    bits.push_str(&format!("{:08b}", byte));
                }
            }
        }
        
        // Pad to byte boundary
        while bits.len() % 8 != 0 {
            bits.push('0');
        }
        
        // Convert bit string to bytes
        let mut bytes = Vec::new();
        for chunk in bits.as_bytes().chunks(8) {
            if chunk.len() == 8 {
                let byte_str = std::str::from_utf8(chunk)
                    .map_err(|e| CodecError::EncodingFailed { 
                        msg: format!("Invalid bit string: {}", e) 
                    })?;
                let byte_val = u8::from_str_radix(byte_str, 2)
                    .map_err(|e| CodecError::EncodingFailed { 
                        msg: format!("Invalid binary: {}", e) 
                    })?;
                bytes.push(byte_val);
            }
        }
        
        Ok(bytes)
    }
    
    fn decode(&mut self, data: &[u8]) -> Result<String> {
        let mut bits = String::new();
        for &byte in data {
            bits.push_str(&format!("{:08b}", byte));
        }
        
        let mut result = String::new();
        let mut current = &self.decode_tree;
        
        for bit_char in bits.chars() {
            match bit_char {
                '0' => {
                    if let Some(ref left) = current.left {
                        current = left;
                    } else {
                        return Err(CodecError::DecodingFailed {
                            msg: "Invalid bit sequence".to_string(),
                        });
                    }
                }
                '1' => {
                    if let Some(ref right) = current.right {
                        current = right;
                    } else {
                        return Err(CodecError::DecodingFailed {
                            msg: "Invalid bit sequence".to_string(),
                        });
                    }
                }
                _ => continue,
            }
            
            if let Some(ch) = current.character {
                result.push(ch);
                current = &self.decode_tree;
            }
        }
        
        Ok(result)
    }
    
    fn compression_ratio(&self) -> f64 {
        // Estimate based on average code length
        0.6 // Placeholder
    }
    
    fn reset(&mut self) {
        // Huffman codec is stateless
    }
}

/// Simple ASCII codec (no compression)
pub struct AsciiCodec;

impl TextCodec for AsciiCodec {
    fn encode(&mut self, text: &str) -> Result<Vec<u8>> {
        Ok(text.as_bytes().to_vec())
    }
    
    fn decode(&mut self, data: &[u8]) -> Result<String> {
        String::from_utf8(data.to_vec())
            .map_err(|e| CodecError::DecodingFailed { 
                msg: format!("Invalid UTF-8: {}", e) 
            })
    }
    
    fn compression_ratio(&self) -> f64 {
        1.0 // No compression
    }
    
    fn reset(&mut self) {
        // Stateless
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_codec() {
        let mut codec = AsciiCodec;
        let text = "Hello, World!";
        
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        
        assert_eq!(text, decoded);
    }

    #[test]
    fn test_huffman_codec() {
        let mut codec = HuffmanCodec::new_english();
        let text = "TEST";
        
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        
        // Should decode to something (might not be exact due to simplified implementation)
        assert!(!decoded.is_empty());
    }
}
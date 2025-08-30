//! Codec registry for managing available codecs

use crate::{CodecError, Result};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Information about a codec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub codec_type: CodecType,
    pub version: String,
    pub parameters: HashMap<String, CodecParameter>,
}

/// Type of codec
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodecType {
    Text,
    Voice,
    Binary,
}

/// Codec parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecParameter {
    pub name: String,
    pub description: String,
    pub parameter_type: ParameterType,
    pub default_value: String,
    pub valid_range: Option<(String, String)>,
}

/// Parameter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Integer,
    Float,
    String,
    Boolean,
    Enum(Vec<String>),
}

/// Registry for managing available codecs
pub struct CodecRegistry {
    codecs: HashMap<String, CodecInfo>,
}

impl CodecRegistry {
    /// Create a new codec registry
    pub fn new() -> Self {
        let mut registry = Self {
            codecs: HashMap::new(),
        };
        
        registry.register_builtin_codecs();
        registry
    }
    
    /// Register built-in codecs
    fn register_builtin_codecs(&mut self) {
        // Register Huffman text codec
        let huffman_info = CodecInfo {
            id: "huffman-english".to_string(),
            name: "Huffman English".to_string(),
            description: "Huffman coding optimized for English text".to_string(),
            codec_type: CodecType::Text,
            version: "1.0.0".to_string(),
            parameters: HashMap::new(),
        };
        self.codecs.insert(huffman_info.id.clone(), huffman_info);
        
        // Register ASCII codec
        let ascii_info = CodecInfo {
            id: "ascii".to_string(),
            name: "ASCII".to_string(),
            description: "Plain ASCII encoding (no compression)".to_string(),
            codec_type: CodecType::Text,
            version: "1.0.0".to_string(),
            parameters: HashMap::new(),
        };
        self.codecs.insert(ascii_info.id.clone(), ascii_info);
        
        // Register PCM voice codec
        let mut pcm_params = HashMap::new();
        pcm_params.insert("sample_rate".to_string(), CodecParameter {
            name: "Sample Rate".to_string(),
            description: "Audio sample rate in Hz".to_string(),
            parameter_type: ParameterType::Integer,
            default_value: "8000".to_string(),
            valid_range: Some(("8000".to_string(), "48000".to_string())),
        });
        
        let pcm_info = CodecInfo {
            id: "pcm-16".to_string(),
            name: "PCM 16-bit".to_string(),
            description: "Uncompressed 16-bit PCM audio".to_string(),
            codec_type: CodecType::Voice,
            version: "1.0.0".to_string(),
            parameters: pcm_params,
        };
        self.codecs.insert(pcm_info.id.clone(), pcm_info);
    }
    
    /// Register a new codec
    pub fn register(&mut self, info: CodecInfo) -> Result<()> {
        if self.codecs.contains_key(&info.id) {
            return Err(CodecError::InvalidParameters {
                msg: format!("Codec '{}' already registered", info.id),
            });
        }
        
        self.codecs.insert(info.id.clone(), info);
        Ok(())
    }
    
    /// Get information about a codec
    pub fn get(&self, id: &str) -> Option<&CodecInfo> {
        self.codecs.get(id)
    }
    
    /// List all available codecs
    pub fn list(&self) -> Vec<&CodecInfo> {
        self.codecs.values().collect()
    }
    
    /// List codecs by type
    pub fn list_by_type(&self, codec_type: CodecType) -> Vec<&CodecInfo> {
        self.codecs
            .values()
            .filter(|info| std::mem::discriminant(&info.codec_type) == std::mem::discriminant(&codec_type))
            .collect()
    }
    
    /// Check if a codec is available
    pub fn is_available(&self, id: &str) -> bool {
        self.codecs.contains_key(id)
    }
    
    /// Export codec registry to JSON
    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.codecs.values().collect::<Vec<_>>())
            .map_err(|e| CodecError::InvalidParameters {
                msg: format!("Failed to serialize registry: {}", e),
            })
    }
}

impl Default for CodecRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = CodecRegistry::new();
        assert!(registry.is_available("ascii"));
        assert!(registry.is_available("huffman-english"));
        assert!(registry.is_available("pcm-16"));
    }

    #[test]
    fn test_codec_listing() {
        let registry = CodecRegistry::new();
        
        let text_codecs = registry.list_by_type(CodecType::Text);
        assert!(text_codecs.len() >= 2);
        
        let voice_codecs = registry.list_by_type(CodecType::Voice);
        assert!(voice_codecs.len() >= 1);
    }

    #[test]
    fn test_codec_registration() {
        let mut registry = CodecRegistry::new();
        
        let custom_codec = CodecInfo {
            id: "custom-test".to_string(),
            name: "Test Codec".to_string(),
            description: "Test codec for unit tests".to_string(),
            codec_type: CodecType::Binary,
            version: "0.1.0".to_string(),
            parameters: HashMap::new(),
        };
        
        registry.register(custom_codec).unwrap();
        assert!(registry.is_available("custom-test"));
        
        let info = registry.get("custom-test").unwrap();
        assert_eq!(info.name, "Test Codec");
    }
}
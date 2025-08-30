//! Forward Error Correction (FEC) implementations

use crate::{FrameError, Result};

/// Generic FEC encoder trait
pub trait FecEncoder {
    /// Encode data with error correction
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Get the code rate (k/n where k is data bits, n is total bits)
    fn code_rate(&self) -> f64;
    
    /// Get overhead bytes for given input length
    fn overhead_bytes(&self, input_len: usize) -> usize;
}

/// Generic FEC decoder trait
pub trait FecDecoder {
    /// Decode data and correct errors
    fn decode(&mut self, encoded_data: &[u8]) -> Result<Vec<u8>>;
    
    /// Check if data can be corrected
    fn can_correct(&self, encoded_data: &[u8]) -> bool;
    
    /// Get error statistics from last decode
    fn error_stats(&self) -> ErrorStats;
}

/// Error correction statistics
#[derive(Debug, Clone, Default)]
pub struct ErrorStats {
    pub corrected_errors: usize,
    pub detected_errors: usize,
    pub uncorrectable_errors: usize,
}

/// Reed-Solomon encoder/decoder
pub struct ReedSolomon {
    n: usize, // Total symbols
    k: usize, // Data symbols
    t: usize, // Error correction capability
}

impl ReedSolomon {
    /// Create a new Reed-Solomon codec
    pub fn new(n: usize, k: usize) -> Result<Self> {
        if n <= k {
            return Err(FrameError::InvalidFecParameters {
                msg: format!("Invalid RS parameters: n={}, k={}", n, k),
            });
        }
        
        if n > 255 {
            return Err(FrameError::InvalidFecParameters {
                msg: format!("RS block size too large: {}", n),
            });
        }
        
        let t = (n - k) / 2;
        
        Ok(Self { n, k, t })
    }
    
    /// Create RS(255,223) - commonly used configuration
    pub fn rs_255_223() -> Result<Self> {
        Self::new(255, 223)
    }
    
    /// Create RS(255,239) - higher rate configuration
    pub fn rs_255_239() -> Result<Self> {
        Self::new(255, 239)
    }
}

impl FecEncoder for ReedSolomon {
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() > self.k {
            return Err(FrameError::InvalidFecParameters {
                msg: format!("Data too long for RS({},{}): {} bytes", self.n, self.k, data.len()),
            });
        }
        
        // TODO: Implement actual Reed-Solomon encoding
        // For now, return a placeholder that appends parity bytes
        let mut encoded = data.to_vec();
        encoded.resize(self.n, 0); // Pad with zeros as placeholder parity
        
        Ok(encoded)
    }
    
    fn code_rate(&self) -> f64 {
        self.k as f64 / self.n as f64
    }
    
    fn overhead_bytes(&self, input_len: usize) -> usize {
        let blocks = (input_len + self.k - 1) / self.k; // Ceiling division
        blocks * (self.n - self.k)
    }
}

impl FecDecoder for ReedSolomon {
    fn decode(&mut self, encoded_data: &[u8]) -> Result<Vec<u8>> {
        if encoded_data.len() != self.n {
            return Err(FrameError::FecDecodingFailed {
                msg: format!("Invalid RS block size: expected {}, got {}", self.n, encoded_data.len()),
            });
        }
        
        // TODO: Implement actual Reed-Solomon decoding
        // For now, return the first k bytes
        Ok(encoded_data[..self.k].to_vec())
    }
    
    fn can_correct(&self, encoded_data: &[u8]) -> bool {
        // TODO: Implement syndrome calculation
        encoded_data.len() == self.n
    }
    
    fn error_stats(&self) -> ErrorStats {
        // TODO: Return actual error statistics
        ErrorStats::default()
    }
}

/// Convolutional encoder/decoder
pub struct Convolutional {
    constraint_length: usize,
    code_rate: (usize, usize), // (k, n) where k input bits produce n output bits
    polynomials: Vec<u32>,
    state: u32,
}

impl Convolutional {
    /// Create a new convolutional codec
    pub fn new(constraint_length: usize, polynomials: Vec<u32>) -> Result<Self> {
        if constraint_length < 3 || constraint_length > 15 {
            return Err(FrameError::InvalidFecParameters {
                msg: format!("Invalid constraint length: {}", constraint_length),
            });
        }
        
        if polynomials.is_empty() {
            return Err(FrameError::InvalidFecParameters {
                msg: "At least one generator polynomial required".to_string(),
            });
        }
        
        // Rate 1/n where n is number of polynomials
        let code_rate = (1, polynomials.len());
        
        Ok(Self {
            constraint_length,
            code_rate,
            polynomials,
            state: 0,
        })
    }
    
    /// Create rate 1/2, K=7 convolutional code (NASA standard)
    pub fn nasa_standard() -> Result<Self> {
        Self::new(7, vec![0o171, 0o133])
    }
    
    /// Create rate 1/3, K=7 convolutional code
    pub fn rate_1_3_k7() -> Result<Self> {
        Self::new(7, vec![0o171, 0o133, 0o165])
    }
}

impl FecEncoder for Convolutional {
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoded = Vec::new();
        
        for &byte in data {
            for bit_pos in (0..8).rev() {
                let input_bit = (byte >> bit_pos) & 1;
                
                // Shift input bit into encoder state
                self.state = (self.state >> 1) | ((input_bit as u32) << (self.constraint_length - 1));
                
                // Generate output bits for each polynomial
                let mut output_byte = 0u8;
                for (i, &poly) in self.polynomials.iter().enumerate() {
                    let output_bit = (self.state & poly).count_ones() & 1;
                    output_byte |= (output_bit as u8) << (7 - i);
                }
                
                encoded.push(output_byte);
            }
        }
        
        // Add tail bits to flush encoder
        for _ in 0..(self.constraint_length - 1) {
            self.state >>= 1;
            
            let mut output_byte = 0u8;
            for (i, &poly) in self.polynomials.iter().enumerate() {
                let output_bit = (self.state & poly).count_ones() & 1;
                output_byte |= (output_bit as u8) << (7 - i);
            }
            
            encoded.push(output_byte);
        }
        
        Ok(encoded)
    }
    
    fn code_rate(&self) -> f64 {
        self.code_rate.0 as f64 / self.code_rate.1 as f64
    }
    
    fn overhead_bytes(&self, input_len: usize) -> usize {
        let input_bits = input_len * 8;
        let output_bits = input_bits * self.code_rate.1 / self.code_rate.0;
        let tail_bits = (self.constraint_length - 1) * self.code_rate.1;
        (output_bits + tail_bits + 7) / 8 // Convert to bytes
    }
}

impl FecDecoder for Convolutional {
    fn decode(&mut self, encoded_data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Implement Viterbi decoding algorithm
        // For now, return a placeholder that takes every nth bit
        let rate_ratio = self.code_rate.1 / self.code_rate.0;
        let decoded_len = encoded_data.len() / rate_ratio;
        let decoded = vec![0u8; decoded_len];
        
        Ok(decoded)
    }
    
    fn can_correct(&self, _encoded_data: &[u8]) -> bool {
        // Convolutional codes can always attempt correction
        true
    }
    
    fn error_stats(&self) -> ErrorStats {
        // TODO: Return actual Viterbi decoder statistics
        ErrorStats::default()
    }
}

/// Simple parity check encoder/decoder
pub struct ParityCheck {
    even_parity: bool,
}

impl ParityCheck {
    /// Create a new parity check codec
    pub fn new(even_parity: bool) -> Self {
        Self { even_parity }
    }
    
    /// Calculate parity bit for data
    fn calculate_parity(&self, data: &[u8]) -> u8 {
        let mut parity = 0u8;
        for &byte in data {
            parity ^= byte.count_ones() as u8;
        }
        
        if self.even_parity {
            parity & 1
        } else {
            (parity & 1) ^ 1
        }
    }
}

impl FecEncoder for ParityCheck {
    fn encode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoded = data.to_vec();
        let parity = self.calculate_parity(data);
        encoded.push(parity);
        Ok(encoded)
    }
    
    fn code_rate(&self) -> f64 {
        // Parity check adds 1 byte overhead
        // For n bytes input, we get n+1 bytes output
        // Code rate = n/(n+1), but we'll use a typical block size
        255.0 / 256.0 // Typical RS(255,254) rate
    }
    
    fn overhead_bytes(&self, _input_len: usize) -> usize {
        1 // Always one parity byte
    }
}

impl FecDecoder for ParityCheck {
    fn decode(&mut self, encoded_data: &[u8]) -> Result<Vec<u8>> {
        if encoded_data.is_empty() {
            return Err(FrameError::FecDecodingFailed {
                msg: "Empty encoded data".to_string(),
            });
        }
        
        let data = &encoded_data[..encoded_data.len() - 1];
        let received_parity = encoded_data[encoded_data.len() - 1];
        let calculated_parity = self.calculate_parity(data);
        
        if received_parity != calculated_parity {
            return Err(FrameError::FecDecodingFailed {
                msg: "Parity check failed".to_string(),
            });
        }
        
        Ok(data.to_vec())
    }
    
    fn can_correct(&self, encoded_data: &[u8]) -> bool {
        if encoded_data.is_empty() {
            return false;
        }
        
        let data = &encoded_data[..encoded_data.len() - 1];
        let received_parity = encoded_data[encoded_data.len() - 1];
        let calculated_parity = self.calculate_parity(data);
        
        received_parity == calculated_parity
    }
    
    fn error_stats(&self) -> ErrorStats {
        ErrorStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reed_solomon_creation() {
        let rs = ReedSolomon::new(255, 223).unwrap();
        assert_eq!(rs.n, 255);
        assert_eq!(rs.k, 223);
        assert_eq!(rs.t, 16);
    }

    #[test]
    fn test_convolutional_creation() {
        let conv = Convolutional::nasa_standard().unwrap();
        assert_eq!(conv.constraint_length, 7);
        assert_eq!(conv.code_rate, (1, 2));
    }

    #[test]
    fn test_parity_check() {
        let mut parity = ParityCheck::new(true);
        let data = b"Hello";
        
        let encoded = parity.encode(data).unwrap();
        assert_eq!(encoded.len(), data.len() + 1);
        
        let decoded = parity.decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_parity_check_error_detection() {
        let mut parity = ParityCheck::new(true);
        let data = b"Hello";
        
        let mut encoded = parity.encode(data).unwrap();
        encoded[0] ^= 0x01; // Introduce error
        
        let result = parity.decode(&encoded);
        assert!(result.is_err());
    }
}
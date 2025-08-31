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
    encode_table: HashMap<char, Vec<u8>>,
    decode_tree: DecodeNode,
    token_map: HashMap<&'static str, char>,        // token => sentinel char
    reverse_token_map: HashMap<char, &'static str>, // sentinel char => token
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
            token_map: HashMap::new(),
            reverse_token_map: HashMap::new(),
        };
        
        codec.init_tokens();
        codec.build_english_table();
        codec
    }
    
    /// Build encoding/decoding tables for English text using canonical Huffman
    /// The frequency model includes space, letters (both cases), digits and common punctuation.
    fn build_english_table(&mut self) {
        // Frequency model (approximate English frequencies scaled to integers)
        // Higher numbers => shorter codes.
        let mut freqs: Vec<(char, u32)> = Vec::new();

        // Space is most common
        freqs.push((' ', 700));

        // Letters (uppercase and lowercase share the same relative weight)
        let letter_freqs: [(char, u32); 26] = [
            ('E', 120), ('T', 90), ('A', 81), ('O', 75), ('I', 70), ('N', 67),
            ('S', 63), ('H', 61), ('R', 60), ('D', 43), ('L', 40), ('C', 28),
            ('U', 28), ('M', 24), ('W', 24), ('F', 22), ('G', 20), ('Y', 20),
            ('P', 19), ('B', 15), ('V', 10), ('K', 8), ('J', 2), ('X', 2),
            ('Q', 1), ('Z', 1),
        ];
        for (ch, w) in letter_freqs.iter() {
            freqs.push((*ch, *w));
            freqs.push((ch.to_ascii_lowercase(), *w));
        }

        // Digits: moderate frequency
        for d in '0'..='9' {
            freqs.push((d, 8));
        }

        // Common punctuation
        for &(ch, w) in [
            ('.', 12), (',', 12), ('!', 6), ('?', 6), (':', 4), (';', 4),
            ('-', 7), ('\'', 5), ('"', 3), ('(', 2), (')', 2), ('/', 2)
        ].iter() {
            freqs.push((ch, w));
        }

        // Add multi-character ham tokens as sentinel characters with high weights
        // so they get short codes.
        for (&sentinel, _tok) in self.reverse_token_map.iter() {
            freqs.push((sentinel, 180));
        }

        self.build_codes_from_frequencies(freqs);
    }

    /// Initialize common ham/QRN/Q-code tokens mapped to private-use sentinels
    fn init_tokens(&mut self) {
        // Private Use Area start
        let mut next = 0xE000u32;
        let tokens: [&'static str; 22] = [
            // Longer tokens first improves greedy matching later
            "QRZ", "QRM", "QRO", "QRP", "QRS", "QRT", "QRB", "QSB", "QSL", "QSO", "QSY", "QTH",
            "CQ", "DE", "BK", "KN", "K", "AR", "SK", "YL", "OM", "73",
        ];
        for t in tokens.iter() {
            let ch = char::from_u32(next).unwrap();
            self.token_map.insert(*t, ch);
            self.reverse_token_map.insert(ch, *t);
            next += 1;
        }
    }

    /// Build canonical Huffman codes from a (char, weight) list
    fn build_codes_from_frequencies(&mut self, freqs: Vec<(char, u32)>) {
        use std::cmp::Ordering;
        use std::collections::BinaryHeap;

        #[derive(Clone)]
        struct Node {
            weight: u32,
            symbol: Option<char>,
            left: Option<Box<Node>>,
            right: Option<Box<Node>>,
        }

        struct HeapItem(u32, usize, Box<Node>); // (weight, tie-breaker, node)
        
        impl PartialEq for HeapItem {
            fn eq(&self, other: &Self) -> bool { self.0 == other.0 && self.1 == other.1 }
        }
        impl Eq for HeapItem {}
        impl Ord for HeapItem {
            fn cmp(&self, other: &Self) -> Ordering {
                // Reverse for min-heap behavior via BinaryHeap (which is max-heap)
                other.0.cmp(&self.0).then_with(|| other.1.cmp(&self.1))
            }
        }
        impl PartialOrd for HeapItem {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
        }

        // Initialize heap with leaves
        let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
        let mut counter: usize = 0; // stable tie-breaker
        for (ch, w) in freqs.into_iter().filter(|&(_, w)| w > 0) {
            let node = Box::new(Node { weight: w, symbol: Some(ch), left: None, right: None });
            heap.push(HeapItem(w, { counter += 1; counter }, node));
        }

        if heap.is_empty() {
            return; // nothing to build
        }

        // Build Huffman tree
        while heap.len() > 1 {
            let HeapItem(w1, _, n1) = heap.pop().unwrap();
            let HeapItem(w2, _, n2) = heap.pop().unwrap();
            let merged = Box::new(Node { weight: w1 + w2, symbol: None, left: Some(n1), right: Some(n2) });
            heap.push(HeapItem(w1 + w2, { counter += 1; counter }, merged));
        }
        let root = heap.pop().unwrap().2;

        // Collect code lengths by DFS
        let mut lengths: Vec<(char, usize)> = Vec::new();
        fn dfs(node: &Node, depth: usize, out: &mut Vec<(char, usize)>) {
            if let Some(ch) = node.symbol {
                // Edge case: single-symbol alphabet -> assign length 1
                out.push((ch, if depth == 0 { 1 } else { depth }));
                return;
            }
            if let Some(ref l) = node.left { dfs(l, depth + 1, out); }
            if let Some(ref r) = node.right { dfs(r, depth + 1, out); }
        }
        dfs(&root, 0, &mut lengths);

        // Canonical assignment: sort by (len, symbol)
        lengths.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

        // Assign codes incrementally
        let mut codes: Vec<(char, Vec<u8>)> = Vec::with_capacity(lengths.len());
        let mut code_val: u32 = 0;
        let mut prev_len: usize = lengths.first().map(|x| x.1).unwrap_or(1);
        for (ch, len) in lengths {
            if len > prev_len { code_val <<= (len - prev_len) as u32; }
            // Convert code_val with 'len' bits into MSB-first bit vector
            let mut bits = Vec::with_capacity(len);
            for i in (0..len).rev() {
                let bit = (code_val >> i) & 1;
                bits.push(bit as u8);
            }
            codes.push((ch, bits));
            code_val += 1;
            prev_len = len;
        }

        // Update encode table
        self.encode_table.clear();
        for (ch, bits) in codes {
            self.encode_table.insert(ch, bits);
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
        
        for (ch, bits) in &self.encode_table {
            let mut current = &mut self.decode_tree;
            
            for &bit in bits {
                let next_node = match bit {
                    0 => &mut current.left,
                    1 => &mut current.right,
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
        let mut bits = Vec::new();

        // Prepare tokens sorted by length desc for greedy matching
        let mut token_list: Vec<&'static str> = self.token_map.keys().copied().collect();
        token_list.sort_by(|a, b| b.len().cmp(&a.len()));

        let s = text;
        let bytes = s.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            // Try token match at this byte position (ASCII tokens)
            let mut matched = false;
            for tok in &token_list {
                let tlen = tok.len();
                if i + tlen <= bytes.len() && bytes[i..i + tlen].eq_ignore_ascii_case(tok.as_bytes()) {
                    // Enforce word boundaries: tokens must be delimited by non-alphanumeric chars (or edges)
                    let prev_is_alnum = if i == 0 { false } else { bytes[i - 1].is_ascii_alphanumeric() };
                    let next_is_alnum = if i + tlen >= bytes.len() { false } else { bytes[i + tlen].is_ascii_alphanumeric() };
                    if prev_is_alnum || next_is_alnum {
                        // Not a whole-word token match; skip this token here
                        continue;
                    }
                    // Emit sentinel for token
                    if let Some(&sent) = self.token_map.get(*tok) {
                        if let Some(code) = self.encode_table.get(&sent) {
                            bits.extend_from_slice(code);
                            i += tlen;
                            matched = true;
                            break;
                        }
                    }
                }
            }
            if matched { continue; }

            // No token matched, encode single Unicode character
            let ch = s[i..].chars().next().unwrap();
            if let Some(code) = self.encode_table.get(&ch) {
                bits.extend_from_slice(code);
            } else {
                // Escape and encode this character as UTF-8 bytes
                for _ in 0..20 { bits.push(1); }
                let mut buf = [0u8; 4];
                let utf8 = ch.encode_utf8(&mut buf);
                let n = utf8.as_bytes().len();
                let len_field = (n as u8) - 1; // 0..3
                bits.push(((len_field >> 1) & 1) as u8);
                bits.push((len_field & 1) as u8);
                for &b in utf8.as_bytes() {
                    for k in (0..8).rev() { bits.push(((b >> k) & 1) as u8); }
                }
            }
            i += ch.len_utf8();
        }

        // Prepend a 16-bit bit-length header so decoder can ignore padding
        let valid_bits = bits.len() as u16;
        let mut full_bits = Vec::with_capacity(16 + bits.len());
        for i in (0..16).rev() {
            full_bits.push(((valid_bits >> i) & 1) as u8);
        }
        full_bits.extend_from_slice(&bits);

        // Pad to byte boundary
        while full_bits.len() % 8 != 0 {
            full_bits.push(0);
        }

        // Convert bit array to bytes
        let mut bytes = Vec::new();
        for chunk in full_bits.chunks(8) {
            let mut byte_val = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit == 1 {
                    byte_val |= 1 << (7 - i);
                }
            }
            bytes.push(byte_val);
        }

        Ok(bytes)
    }
    
    fn decode(&mut self, data: &[u8]) -> Result<String> {
        if data.len() < 2 {
            return Err(CodecError::DecodingFailed { msg: "Data too short".to_string() });
        }

        // Unpack all bits from bytes
        let mut all_bits = Vec::with_capacity(data.len() * 8);
        for &byte in data {
            for i in (0..8).rev() {
                all_bits.push(((byte >> i) & 1) as u8);
            }
        }

        // Read first 16 bits as valid bit length
        if all_bits.len() < 16 {
            return Err(CodecError::DecodingFailed { msg: "Insufficient header bits".to_string() });
        }
        let mut bit_len: u16 = 0;
        for i in 0..16 {
            bit_len = (bit_len << 1) | (all_bits[i] as u16);
        }

        // Slice exactly the number of valid bits
        let mut bits = Vec::new();
        let available = all_bits.len() - 16;
        let take = std::cmp::min(bit_len as usize, available);
        bits.extend_from_slice(&all_bits[16..16 + take]);

    let mut result = String::new();
        let mut current = &self.decode_tree;
        let mut i = 0;
        
        while i < bits.len() {
            // Check for escape sequence (20 consecutive ones)
            if i + 20 < bits.len() {
                let mut is_escape = true;
                for j in 0..20 {
                    if bits[i + j] != 1 {
                        is_escape = false;
                        break;
                    }
                }
                if is_escape {
                    // Need at least 20 ones + 2 bits length
                    if i + 22 > bits.len() { break; }
                    let len_hi = bits[i + 20];
                    let len_lo = bits[i + 21];
                    let n = ((len_hi << 1) | len_lo) + 1; // 1..4
                    let n_bytes = n as usize;
                    let needed = 22 + 8 * n_bytes;
                    if i + needed > bits.len() { break; }
                    let mut bytes = Vec::with_capacity(n_bytes);
                    let mut idx = i + 22;
                    for _ in 0..n_bytes {
                        let mut b = 0u8;
                        for k in 0..8 {
                            if bits[idx + k] == 1 { b |= 1 << (7 - k); }
                        }
                        bytes.push(b);
                        idx += 8;
                    }
                    match std::str::from_utf8(&bytes) {
                        Ok(s) => result.push_str(s),
                        Err(_) => {
                            return Err(CodecError::DecodingFailed { msg: "Invalid UTF-8 in escape".to_string() });
                        }
                    }
                    i += needed;
                    current = &self.decode_tree;
                    continue;
                }
            }
            
            // Regular Huffman decoding
            match bits[i] {
                0 => {
                    if let Some(ref left) = current.left {
                        current = left;
                    } else {
                        return Err(CodecError::DecodingFailed {
                            msg: "Invalid bit sequence - no left branch".to_string(),
                        });
                    }
                }
                1 => {
                    if let Some(ref right) = current.right {
                        current = right;
                    } else {
                        return Err(CodecError::DecodingFailed {
                            msg: "Invalid bit sequence - no right branch".to_string(),
                        });
                    }
                }
                _ => {
                    return Err(CodecError::DecodingFailed {
                        msg: "Invalid bit value".to_string(),
                    });
                }
            }
            
            if let Some(ch) = current.character {
                if let Some(tok) = self.reverse_token_map.get(&ch) {
                    result.push_str(tok);
                } else {
                    result.push(ch);
                }
                current = &self.decode_tree;
            }
            
            i += 1;
        }
        
        Ok(result)
    }
    
    fn compression_ratio(&self) -> f64 {
        // Estimate based on average code length vs 8 bits per char
        0.6 // Typically 40% compression for English text
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
        let text = "HELLO";
        
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        
        assert_eq!(text, decoded);
    }

    #[test]
    fn test_huffman_no_trailing_space() {
        let mut codec = HuffmanCodec::new_english();
        let text = "HELLO";
        let encoded = codec.encode(text).unwrap();
        // Ensure decode equals exactly, no padding artifacts
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_huffman_no_false_token_inside_words() {
        let mut codec = HuffmanCodec::new_english();
        let text = "from smoky cqsly ombudsman qrzv"; // contains substrings matching tokens but not word-bounded
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_huffman_unicode_fallback() {
        let mut codec = HuffmanCodec::new_english();
        let text = "HELLO ŠČĆŽ";
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_huffman_tokens_roundtrip_1() {
        let mut codec = HuffmanCodec::new_english();
        let text = "CQ CQ DE S56SPZ K";
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_huffman_tokens_roundtrip_2() {
        let mut codec = HuffmanCodec::new_english();
        let text = "QRZ? QRM QTH JN76";
        let encoded = codec.encode(text).unwrap();
        let decoded = codec.decode(&encoded).unwrap();
        assert_eq!(decoded, text);
    }
}
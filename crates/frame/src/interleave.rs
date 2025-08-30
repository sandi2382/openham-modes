//! Interleaving for burst error mitigation

use crate::{FrameError, Result};

/// Generic interleaver trait
pub trait Interleaver {
    /// Interleave data to spread errors
    fn interleave(&mut self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Deinterleave data to concentrate errors
    fn deinterleave(&mut self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Reset interleaver state
    fn reset(&mut self);
}

/// Block interleaver implementation
pub struct BlockInterleaver {
    rows: usize,
    cols: usize,
}

impl BlockInterleaver {
    /// Create a new block interleaver
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        if rows == 0 || cols == 0 {
            return Err(FrameError::InterleavingError {
                msg: "Interleaver dimensions must be greater than 0".to_string(),
            });
        }
        
        Ok(Self { rows, cols })
    }
    
    /// Get the block size (total elements)
    pub fn block_size(&self) -> usize {
        self.rows * self.cols
    }
}

impl Interleaver for BlockInterleaver {
    fn interleave(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let block_size = self.block_size();
        
        if data.len() % block_size != 0 {
            return Err(FrameError::InterleavingError {
                msg: format!("Data length {} not multiple of block size {}", data.len(), block_size),
            });
        }
        
        let mut result = Vec::with_capacity(data.len());
        
        // Process data in blocks
        for block_start in (0..data.len()).step_by(block_size) {
            let block_end = block_start + block_size;
            let block = &data[block_start..block_end];
            
            // Write data row by row, read column by column
            for col in 0..self.cols {
                for row in 0..self.rows {
                    let index = row * self.cols + col;
                    result.push(block[index]);
                }
            }
        }
        
        Ok(result)
    }
    
    fn deinterleave(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let block_size = self.block_size();
        
        if data.len() % block_size != 0 {
            return Err(FrameError::InterleavingError {
                msg: format!("Data length {} not multiple of block size {}", data.len(), block_size),
            });
        }
        
        let mut result = Vec::with_capacity(data.len());
        
        // Process data in blocks
        for block_start in (0..data.len()).step_by(block_size) {
            let block_end = block_start + block_size;
            let block = &data[block_start..block_end];
            
            // Create temporary matrix
            let mut matrix = vec![vec![0u8; self.cols]; self.rows];
            
            // Fill matrix column by column
            let mut index = 0;
            for col in 0..self.cols {
                for row in 0..self.rows {
                    matrix[row][col] = block[index];
                    index += 1;
                }
            }
            
            // Read matrix row by row
            for row in 0..self.rows {
                for col in 0..self.cols {
                    result.push(matrix[row][col]);
                }
            }
        }
        
        Ok(result)
    }
    
    fn reset(&mut self) {
        // Block interleaver is stateless
    }
}

/// Convolutional interleaver implementation
pub struct ConvolutionalInterleaver {
    branches: usize,
    depth: usize,
    delays: Vec<Vec<u8>>,
    input_index: usize,
    output_index: usize,
}

impl ConvolutionalInterleaver {
    /// Create a new convolutional interleaver
    pub fn new(branches: usize, depth: usize) -> Result<Self> {
        if branches == 0 {
            return Err(FrameError::InterleavingError {
                msg: "Number of branches must be greater than 0".to_string(),
            });
        }
        
        // Create delay lines for each branch
        let mut delays = Vec::with_capacity(branches);
        for i in 0..branches {
            let delay_length = i * depth;
            delays.push(vec![0u8; delay_length]);
        }
        
        Ok(Self {
            branches,
            depth,
            delays,
            input_index: 0,
            output_index: 0,
        })
    }
    
    /// Get total memory requirement
    pub fn memory_size(&self) -> usize {
        self.delays.iter().map(|d| d.len()).sum()
    }
}

impl Interleaver for ConvolutionalInterleaver {
    fn interleave(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(data.len());
        
        for &byte in data {
            // Get current branch
            let branch = self.input_index % self.branches;
            
            // Process through delay line
            let output = if self.delays[branch].is_empty() {
                // No delay for this branch
                byte
            } else {
                // Shift through delay line
                let delayed = self.delays[branch][0];
                for i in 0..self.delays[branch].len() - 1 {
                    self.delays[branch][i] = self.delays[branch][i + 1];
                }
                let delay_len = self.delays[branch].len();
                self.delays[branch][delay_len - 1] = byte;
                delayed
            };
            
            result.push(output);
            self.input_index += 1;
        }
        
        Ok(result)
    }
    
    fn deinterleave(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(data.len());
        
        for &byte in data {
            // Get current branch
            let branch = self.output_index % self.branches;
            
            // Process through delay line (same as interleave for convolutional)
            let output = if self.delays[branch].is_empty() {
                byte
            } else {
                let delayed = self.delays[branch][0];
                for i in 0..self.delays[branch].len() - 1 {
                    self.delays[branch][i] = self.delays[branch][i + 1];
                }
                let delay_len = self.delays[branch].len();
                self.delays[branch][delay_len - 1] = byte;
                delayed
            };
            
            result.push(output);
            self.output_index += 1;
        }
        
        Ok(result)
    }
    
    fn reset(&mut self) {
        for delay in &mut self.delays {
            delay.fill(0);
        }
        self.input_index = 0;
        self.output_index = 0;
    }
}

/// Helical interleaver (variant of convolutional)
pub struct HelicalInterleaver {
    matrix: Vec<Vec<u8>>,
    rows: usize,
    cols: usize,
    input_pos: (usize, usize),
    output_pos: (usize, usize),
}

impl HelicalInterleaver {
    /// Create a new helical interleaver
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        if rows == 0 || cols == 0 {
            return Err(FrameError::InterleavingError {
                msg: "Interleaver dimensions must be greater than 0".to_string(),
            });
        }
        
        let matrix = vec![vec![0u8; cols]; rows];
        
        Ok(Self {
            matrix,
            rows,
            cols,
            input_pos: (0, 0),
            output_pos: (0, 0),
        })
    }
    
    /// Advance position with helical pattern
    fn advance_position(&self, pos: (usize, usize)) -> (usize, usize) {
        let (row, col) = pos;
        let new_col = (col + 1) % self.cols;
        let new_row = if new_col == 0 {
            (row + 1) % self.rows
        } else {
            row
        };
        (new_row, new_col)
    }
}

impl Interleaver for HelicalInterleaver {
    fn interleave(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(data.len());
        
        for &byte in data {
            // Store input at current input position
            self.matrix[self.input_pos.0][self.input_pos.1] = byte;
            
            // Read output from current output position
            let output = self.matrix[self.output_pos.0][self.output_pos.1];
            result.push(output);
            
            // Advance positions
            self.input_pos = self.advance_position(self.input_pos);
            self.output_pos = self.advance_position(self.output_pos);
        }
        
        Ok(result)
    }
    
    fn deinterleave(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        // For helical interleaver, deinterleaving is the same as interleaving
        // with different starting positions
        self.interleave(data)
    }
    
    fn reset(&mut self) {
        for row in &mut self.matrix {
            row.fill(0);
        }
        self.input_pos = (0, 0);
        self.output_pos = (0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_interleaver_creation() {
        let interleaver = BlockInterleaver::new(4, 8).unwrap();
        assert_eq!(interleaver.rows, 4);
        assert_eq!(interleaver.cols, 8);
        assert_eq!(interleaver.block_size(), 32);
    }

    #[test]
    fn test_block_interleaver_roundtrip() {
        let mut interleaver = BlockInterleaver::new(2, 4).unwrap();
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7];
        
        let interleaved = interleaver.interleave(&data).unwrap();
        let deinterleaved = interleaver.deinterleave(&interleaved).unwrap();
        
        assert_eq!(data, deinterleaved);
    }

    #[test]
    fn test_block_interleaver_pattern() {
        let mut interleaver = BlockInterleaver::new(2, 2).unwrap();
        let data = vec![0, 1, 2, 3];
        
        let interleaved = interleaver.interleave(&data).unwrap();
        // Expected: [0, 2, 1, 3] (read column-wise)
        assert_eq!(interleaved, vec![0, 2, 1, 3]);
    }

    #[test]
    fn test_convolutional_interleaver_creation() {
        let interleaver = ConvolutionalInterleaver::new(4, 2).unwrap();
        assert_eq!(interleaver.branches, 4);
        assert_eq!(interleaver.depth, 2);
        assert_eq!(interleaver.memory_size(), 0 + 2 + 4 + 6); // Sum of delays
    }

    #[test]
    fn test_helical_interleaver_creation() {
        let interleaver = HelicalInterleaver::new(3, 4).unwrap();
        assert_eq!(interleaver.rows, 3);
        assert_eq!(interleaver.cols, 4);
    }

    #[test]
    fn test_invalid_dimensions() {
        assert!(BlockInterleaver::new(0, 4).is_err());
        assert!(ConvolutionalInterleaver::new(0, 2).is_err());
        assert!(HelicalInterleaver::new(3, 0).is_err());
    }
}
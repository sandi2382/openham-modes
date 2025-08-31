//! Orthogonal Frequency Division Multiplexing (OFDM) implementation
//! 
//! Supports configurable OFDM with pilot tones, cyclic prefix, and
//! channel estimation for robust multipath communication.

use crate::Result;
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality};
use openham_core::buffer::Complex;
use openham_core::fft::{FftProcessor, FftConfig};
use std::f64::consts::PI;

/// OFDM configuration parameters
#[derive(Debug, Clone)]
pub struct OfdmConfig {
    pub fft_size: usize,           // FFT size (e.g., 64, 128, 256, 512, 1024)
    pub cp_length: usize,          // Cyclic prefix length
    pub data_carriers: Vec<usize>, // Indices of data-carrying subcarriers
    pub pilot_carriers: Vec<usize>, // Indices of pilot subcarriers
    pub pilot_symbols: Vec<Complex>, // Pilot symbol values
    pub null_carriers: Vec<usize>,   // Null subcarriers (including DC)
}

impl OfdmConfig {
    /// Create a basic OFDM configuration for amateur radio
    pub fn amateur_radio_64() -> Self {
        let fft_size = 64;
        let cp_length = 16; // 25% cyclic prefix
        
        // Real-signal OFDM: use only positive-frequency bins (1..N/2-1)
        // and mirror them to negative frequencies when modulating.
        // Avoid DC (0) and Nyquist (N/2 = 32). Reserve a few pilots.
        let mut data_carriers = Vec::new();
        for i in 1..32 { // positive frequencies only
            if i == 5 || i == 15 || i == 25 { continue; } // pilots
            data_carriers.push(i);
        }
        
        // Pilot carriers for channel estimation (positive side only)
        let pilot_carriers = vec![5, 15, 25];
        
        // BPSK pilot symbols
        let pilot_symbols = vec![
            Complex::new(1.0, 0.0),
            Complex::new(-1.0, 0.0),
            Complex::new(1.0, 0.0),
            Complex::new(-1.0, 0.0),
            Complex::new(1.0, 0.0),
        ];
        
        // Null carriers: DC and Nyquist
    let null_carriers = vec![0, 32];
        
        Self {
            fft_size,
            cp_length,
            data_carriers,
            pilot_carriers,
            pilot_symbols,
            null_carriers,
        }
    }
    
    /// Create a robust OFDM configuration with more pilots
    pub fn robust_128() -> Self {
        let fft_size = 128;
        let cp_length = 32; // 25% cyclic prefix
        
        // More conservative data carrier allocation
        let mut data_carriers = Vec::new();
        for i in 1..53 {
            if i % 7 != 0 { // Every 7th carrier is pilot
                data_carriers.push(i);
            }
        }
        for i in 75..127 {
            if i % 7 != 0 {
                data_carriers.push(i);
            }
        }
        
        // Regular pilot spacing
        let mut pilot_carriers = Vec::new();
        let mut pilot_symbols = Vec::new();
        for i in 1..53 {
            if i % 7 == 0 {
                pilot_carriers.push(i);
                pilot_symbols.push(if (i / 7) % 2 == 0 {
                    Complex::new(1.0, 0.0)
                } else {
                    Complex::new(-1.0, 0.0)
                });
            }
        }
        for i in 75..127 {
            if i % 7 == 0 {
                pilot_carriers.push(i);
                pilot_symbols.push(if (i / 7) % 2 == 0 {
                    Complex::new(1.0, 0.0)
                } else {
                    Complex::new(-1.0, 0.0)
                });
            }
        }
        
        // Null carriers
        let mut null_carriers = vec![0]; // DC
        for i in 53..75 { // Guard band
            null_carriers.push(i);
        }
        
        Self {
            fft_size,
            cp_length,
            data_carriers,
            pilot_carriers,
            pilot_symbols,
            null_carriers,
        }
    }
    
    /// Get total symbol length (FFT + CP)
    pub fn symbol_length(&self) -> usize {
        self.fft_size + self.cp_length
    }
    
    /// Get number of data carriers
    pub fn num_data_carriers(&self) -> usize {
        self.data_carriers.len()
    }
}

/// OFDM modulator
pub struct OfdmModulator {
    config: ModulationConfig,
    ofdm_config: OfdmConfig,
    fft_processor: FftProcessor,
    pilot_phase: f64,
}

impl OfdmModulator {
    /// Create a new OFDM modulator
    pub fn new(config: ModulationConfig, ofdm_config: OfdmConfig) -> Result<Self> {
        let fft_config = FftConfig::new(ofdm_config.fft_size, config.sample_rate)?;
        let fft_processor = FftProcessor::new(fft_config)?;
        
        Ok(Self {
            config,
            ofdm_config,
            fft_processor,
            pilot_phase: 0.0,
        })
    }
    
    /// Map bits to subcarrier symbols (using QPSK for now)
    fn map_bits_to_symbols(&self, bits: &[u8]) -> Vec<Complex> {
        let mut symbols = Vec::new();
        let bits_per_symbol = 2; // QPSK
        
        // Convert bytes to bits
        let mut bit_stream = Vec::new();
        for &byte in bits {
            for i in (0..8).rev() {
                bit_stream.push((byte >> i) & 1);
            }
        }
        
        // Group bits into QPSK symbols
        for symbol_bits in bit_stream.chunks(bits_per_symbol) {
            let bits_value = symbol_bits.iter().fold(0u8, |acc, &bit| (acc << 1) | bit);
            
            let symbol = match bits_value {
                0b00 => Complex::new(1.0, 1.0),    // 00 -> +1+1j
                0b01 => Complex::new(1.0, -1.0),   // 01 -> +1-1j
                0b10 => Complex::new(-1.0, 1.0),   // 10 -> -1+1j
                0b11 => Complex::new(-1.0, -1.0),  // 11 -> -1-1j
                _ => Complex::new(0.0, 0.0),
            };
            
            // Normalize for unit power
            symbols.push(symbol * (1.0 / 2.0_f64.sqrt()));
        }
        
        symbols
    }
    
    /// Insert pilot tones with phase rotation
    fn insert_pilots(&mut self, frame: &mut [Complex]) {
        for (i, &carrier_idx) in self.ofdm_config.pilot_carriers.iter().enumerate() {
            if i < self.ofdm_config.pilot_symbols.len() && carrier_idx < frame.len() {
                // Apply pilot phase rotation for channel tracking
                let pilot_with_phase = Complex::new(
                    self.ofdm_config.pilot_symbols[i].real * self.pilot_phase.cos() 
                        - self.ofdm_config.pilot_symbols[i].imag * self.pilot_phase.sin(),
                    self.ofdm_config.pilot_symbols[i].real * self.pilot_phase.sin() 
                        + self.ofdm_config.pilot_symbols[i].imag * self.pilot_phase.cos(),
                );
                frame[carrier_idx] = pilot_with_phase;
                // Mirror to negative frequency bin to enforce Hermitian symmetry
                let n = frame.len();
                let mirror = (n + n - carrier_idx) % n; // effectively n - carrier_idx
                if mirror != carrier_idx && mirror < n {
                    frame[mirror] = Complex::new(pilot_with_phase.real, -pilot_with_phase.imag);
                }
            }
        }
        
        // Update pilot phase for next symbol
        self.pilot_phase += PI / 4.0; // 45 degree rotation per symbol
        if self.pilot_phase >= 2.0 * PI {
            self.pilot_phase -= 2.0 * PI;
        }
    }
    
    /// Add cyclic prefix
    fn add_cyclic_prefix(&self, ofdm_symbol: &[Complex]) -> Vec<Complex> {
        let mut result = Vec::with_capacity(self.ofdm_config.symbol_length());
        
        // Add cyclic prefix (copy last CP samples to beginning)
        let cp_start = ofdm_symbol.len() - self.ofdm_config.cp_length;
        result.extend_from_slice(&ofdm_symbol[cp_start..]);
        
        // Add the complete OFDM symbol
        result.extend_from_slice(ofdm_symbol);
        
        result
    }
}

impl Modulator for OfdmModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        // Map bits to symbols
        let data_symbols = self.map_bits_to_symbols(bits);
        let symbols_per_ofdm = self.ofdm_config.num_data_carriers();
        
        // Process OFDM symbols
        for symbol_chunk in data_symbols.chunks(symbols_per_ofdm) {
            // Create frequency domain frame
            let mut freq_frame = vec![Complex::new(0.0, 0.0); self.ofdm_config.fft_size];
            
            // Insert data symbols
            for (i, &symbol) in symbol_chunk.iter().enumerate() {
                if i < self.ofdm_config.data_carriers.len() {
                    let carrier_idx = self.ofdm_config.data_carriers[i];
                    if carrier_idx < freq_frame.len() {
                        // Place on positive bin
                        freq_frame[carrier_idx] = symbol;
                        // Mirror to negative bin for real IFFT output
                        let n = freq_frame.len();
                        let mirror = (n + n - carrier_idx) % n; // n - carrier_idx
                        if mirror != carrier_idx && mirror < n {
                            freq_frame[mirror] = Complex::new(symbol.real, -symbol.imag);
                        }
                    }
                }
            }
            
            // Insert pilot tones
            self.insert_pilots(&mut freq_frame);
            
            // Ensure DC and Nyquist are zero/real
            freq_frame[0] = Complex::new(0.0, 0.0);
            if self.ofdm_config.fft_size % 2 == 0 {
                let nyq = self.ofdm_config.fft_size / 2;
                freq_frame[nyq] = Complex::new(0.0, 0.0);
            }

            // Convert to time domain using IFFT
            let mut time_frame = vec![Complex::new(0.0, 0.0); self.ofdm_config.fft_size];
            self.fft_processor.ifft(&freq_frame, &mut time_frame)?;
            
            // Add cyclic prefix
            let ofdm_symbol = self.add_cyclic_prefix(&time_frame);
            
            // Add to output
            output.extend_from_slice(&ofdm_symbol);
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        self.ofdm_config.symbol_length()
    }
    
    fn symbol_rate(&self) -> f64 {
        self.config.sample_rate / self.ofdm_config.symbol_length() as f64
    }
    
    fn reset(&mut self) {
        self.pilot_phase = 0.0;
    }
}

/// OFDM demodulator with channel estimation
pub struct OfdmDemodulator {
    config: ModulationConfig,
    ofdm_config: OfdmConfig,
    fft_processor: FftProcessor,
    is_sync: bool,
    signal_quality: SignalQuality,
    channel_estimates: Vec<Complex>,
    pilot_phase: f64,
    symbol_buffer: Vec<Complex>,
}

impl OfdmDemodulator {
    /// Create a new OFDM demodulator
    pub fn new(config: ModulationConfig, ofdm_config: OfdmConfig) -> Result<Self> {
        let fft_config = FftConfig::new(ofdm_config.fft_size, config.sample_rate)?;
        let fft_processor = FftProcessor::new(fft_config)?;
        
        // Initialize channel estimates to ones (flat channel assumption)
        let channel_estimates = vec![Complex::new(1.0, 0.0); ofdm_config.fft_size];
        
        Ok(Self {
            config,
            ofdm_config,
            fft_processor,
            is_sync: false,
            signal_quality: SignalQuality::default(),
            channel_estimates,
            pilot_phase: 0.0,
            symbol_buffer: Vec::new(),
        })
    }
    
    /// Compute normalized CP correlation metric at a given offset
    fn cp_correlation_at(&self, buf: &[Complex], off: usize) -> f64 {
        if off + self.ofdm_config.fft_size + self.ofdm_config.cp_length > buf.len() {
            return 0.0;
        }
        let mut num_r = 0.0;
        let mut num_i = 0.0;
        let mut p1 = 0.0;
        let mut p2 = 0.0;
        for n in 0..self.ofdm_config.cp_length {
            let a = buf[off + n];
            let b = buf[off + self.ofdm_config.fft_size + n];
            // a * conj(b)
            num_r += a.real * b.real + a.imag * b.imag;
            num_i += a.imag * b.real - a.real * b.imag;
            p1 += a.norm_sqr();
            p2 += b.norm_sqr();
        }
        let denom = (p1 * p2).sqrt();
        if denom <= 1e-12 { 0.0 } else { (num_r * num_r + num_i * num_i).sqrt() / denom }
    }
    
    /// Find best OFDM symbol start within buffer using CP correlation
    fn find_symbol_start(&self, buf: &[Complex]) -> Option<(usize, f64)> {
        let need = self.ofdm_config.fft_size + self.ofdm_config.cp_length;
        if buf.len() < need { return None; }
        let mut best_off = 0usize;
        let mut best_val = 0.0;
        let max_off = buf.len() - need;
        for off in 0..=max_off {
            let v = self.cp_correlation_at(buf, off);
            if v > best_val { best_val = v; best_off = off; }
        }
        Some((best_off, best_val))
    }
    
    /// Remove cyclic prefix
    fn remove_cyclic_prefix(&self, received_symbol: &[Complex]) -> Vec<Complex> {
        if received_symbol.len() >= self.ofdm_config.symbol_length() {
            let start_idx = self.ofdm_config.cp_length;
            let end_idx = start_idx + self.ofdm_config.fft_size;
            received_symbol[start_idx..end_idx].to_vec()
        } else {
            vec![Complex::new(0.0, 0.0); self.ofdm_config.fft_size]
        }
    }
    
    /// Estimate channel using pilot tones
    fn estimate_channel(&mut self, freq_frame: &[Complex]) {
        for (i, &carrier_idx) in self.ofdm_config.pilot_carriers.iter().enumerate() {
            if i < self.ofdm_config.pilot_symbols.len() && carrier_idx < freq_frame.len() {
                let received_pilot = freq_frame[carrier_idx];
                
                // Expected pilot with phase rotation
                let expected_pilot = Complex::new(
                    self.ofdm_config.pilot_symbols[i].real * self.pilot_phase.cos() 
                        - self.ofdm_config.pilot_symbols[i].imag * self.pilot_phase.sin(),
                    self.ofdm_config.pilot_symbols[i].real * self.pilot_phase.sin() 
                        + self.ofdm_config.pilot_symbols[i].imag * self.pilot_phase.cos(),
                );
                
                // Channel estimate = received / expected
                if expected_pilot.norm() > 1e-6 {
                    self.channel_estimates[carrier_idx] = Complex::new(
                        (received_pilot.real * expected_pilot.real + received_pilot.imag * expected_pilot.imag) / expected_pilot.norm_sqr(),
                        (received_pilot.imag * expected_pilot.real - received_pilot.real * expected_pilot.imag) / expected_pilot.norm_sqr(),
                    );
                }
            }
        }
        
        // Update pilot phase for next symbol
        self.pilot_phase += PI / 4.0;
        if self.pilot_phase >= 2.0 * PI {
            self.pilot_phase -= 2.0 * PI;
        }
        
        // Interpolate channel estimates for data carriers (simplified)
        // In a real implementation, this would use more sophisticated interpolation
    }
    
    /// Apply channel equalization
    fn equalize(&self, freq_frame: &mut [Complex]) {
        for i in 0..freq_frame.len() {
            if self.channel_estimates[i].norm() > 1e-6 {
                // Zero-forcing equalization: divide by channel estimate
                let h_conj = Complex::new(
                    self.channel_estimates[i].real,
                    -self.channel_estimates[i].imag,
                );
                let h_mag_sqr = self.channel_estimates[i].norm_sqr();
                
                freq_frame[i] = Complex::new(
                    (freq_frame[i].real * h_conj.real - freq_frame[i].imag * h_conj.imag) / h_mag_sqr,
                    (freq_frame[i].real * h_conj.imag + freq_frame[i].imag * h_conj.real) / h_mag_sqr,
                );
            }
        }
    }
    
    /// Demodulate QPSK symbols to bits
    fn demodulate_symbols(&self, symbols: &[Complex]) -> Vec<u8> {
        let mut bits = Vec::new();
        
        for &symbol in symbols {
            // Hard decision QPSK demodulation
            let i_bit = if symbol.real > 0.0 { 0 } else { 1 };
            let q_bit = if symbol.imag > 0.0 { 0 } else { 1 };
            
            bits.push(i_bit);
            bits.push(q_bit);
        }
        
        // Pack bits into bytes
        let mut bytes = Vec::new();
        for byte_bits in bits.chunks(8) {
            let mut byte_val = 0u8;
            for (i, &bit) in byte_bits.iter().enumerate() {
                if bit != 0 {
                    byte_val |= 1 << (7 - i);
                }
            }
            bytes.push(byte_val);
        }
        
        bytes
    }
}

impl Demodulator for OfdmDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();
        
        // Add samples to buffer
        self.symbol_buffer.extend_from_slice(samples);
        
        // Process complete OFDM symbols
        let symbol_length = self.ofdm_config.symbol_length();
        while self.symbol_buffer.len() >= symbol_length {
            // If not synchronized yet, scan for the first symbol start within buffer
            if !self.is_sync {
                if let Some((start, corr)) = self.find_symbol_start(&self.symbol_buffer) {
                    // Require a reasonable correlation to lock; tolerate noise/preamble
                    if corr >= 0.5 {
                        if start > 0 { self.symbol_buffer.drain(..start); }
                        self.is_sync = true;
                    } else {
                        // Not enough evidence of OFDM symbol yet; keep last symbol_length-1 samples
                        if self.symbol_buffer.len() > symbol_length { 
                            let drop = self.symbol_buffer.len() - (symbol_length - 1);
                            self.symbol_buffer.drain(..drop);
                        }
                        break;
                    }
                } else {
                    break;
                }
            }
            // Fine timing within CP window on current symbol-length window
            let mut best_off = 0usize;
            let mut best_val = -1.0f64;
            for off in 0..self.ofdm_config.cp_length.min(self.symbol_buffer.len().saturating_sub(symbol_length)+1) {
                let v = self.cp_correlation_at(&self.symbol_buffer[..symbol_length + off], off);
                if v > best_val { best_val = v; best_off = off; }
            }

            // Apply fine offset if available, then extract exactly one symbol
            if best_off > 0 {
                if self.symbol_buffer.len() < symbol_length + best_off { break; }
                self.symbol_buffer.drain(..best_off);
            }
            if self.symbol_buffer.len() < symbol_length { break; }
            let ofdm_symbol: Vec<Complex> = self.symbol_buffer[..symbol_length].to_vec();
            self.symbol_buffer.drain(..symbol_length);
            
            // Already synchronized by CP correlation search
            
            // Remove cyclic prefix
            let time_frame = self.remove_cyclic_prefix(&ofdm_symbol);
            
            // Convert to frequency domain using FFT
            let mut freq_frame = vec![Complex::new(0.0, 0.0); self.ofdm_config.fft_size];
            self.fft_processor.fft(&time_frame, &mut freq_frame)?;
            
            // Estimate channel using pilots
            self.estimate_channel(&freq_frame);
            
            // Apply channel equalization
            self.equalize(&mut freq_frame);
            
            // Extract data symbols from positive-frequency carriers only
            let mut data_symbols = Vec::new();
            for &carrier_idx in &self.ofdm_config.data_carriers {
                if carrier_idx < freq_frame.len() {
                    data_symbols.push(freq_frame[carrier_idx]);
                }
            }
            
            // Demodulate symbols to bits
            let symbol_bits = self.demodulate_symbols(&data_symbols);
            output.extend(symbol_bits);
            
            // Update signal quality (simplified)
            let avg_power: f64 = data_symbols.iter().map(|s| s.norm_sqr()).sum::<f64>() / data_symbols.len() as f64;
            if avg_power > 0.0 {
                self.signal_quality.snr_db = 10.0 * avg_power.log10();
            }
        }
        
        Ok(())
    }
    
    fn is_synchronized(&self) -> bool {
        self.is_sync
    }
    
    fn signal_quality(&self) -> SignalQuality {
        self.signal_quality.clone()
    }
    
    fn reset(&mut self) {
        self.is_sync = false;
        self.signal_quality = SignalQuality::default();
        self.channel_estimates = vec![Complex::new(1.0, 0.0); self.ofdm_config.fft_size];
        self.pilot_phase = 0.0;
        self.symbol_buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ofdm_config_creation() {
        let config = OfdmConfig::amateur_radio_64();
        assert_eq!(config.fft_size, 64);
        assert_eq!(config.cp_length, 16);
        assert!(!config.data_carriers.is_empty());
        assert!(!config.pilot_carriers.is_empty());
    }

    #[test]
    fn test_ofdm_modulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let ofdm_config = OfdmConfig::amateur_radio_64();
        let _modulator = OfdmModulator::new(mod_config, ofdm_config).unwrap();
    }

    #[test]
    fn test_ofdm_demodulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let ofdm_config = OfdmConfig::robust_128();
        let _demodulator = OfdmDemodulator::new(mod_config, ofdm_config).unwrap();
    }

    #[test]
    fn test_cyclic_prefix() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let ofdm_config = OfdmConfig::amateur_radio_64();
        let modulator = OfdmModulator::new(mod_config, ofdm_config).unwrap();
        
        let test_symbol = vec![Complex::new(1.0, 0.0); 64];
        let with_cp = modulator.add_cyclic_prefix(&test_symbol);
        
        assert_eq!(with_cp.len(), 80); // 64 + 16 CP
        
        // Check that CP contains last 16 samples
        for i in 0..16 {
            assert_eq!(with_cp[i].real, test_symbol[48 + i].real);
        }
    }

    #[test]
    fn test_ofdm_modulation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let ofdm_config = OfdmConfig::amateur_radio_64();
        let mut modulator = OfdmModulator::new(mod_config, ofdm_config).unwrap();
        
        let data = vec![0b11001010, 0b10110011]; // Test data
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        
        // Should generate samples
        assert!(!output.is_empty());
        
        // Should be multiple of symbol length
        assert_eq!(output.len() % modulator.samples_per_symbol(), 0);
    }
}
//! Enhanced Phase Shift Keying (PSK) implementations
//! 
//! Supports BPSK, QPSK, 8PSK and other PSK variants with
//! constellation mapping and differential encoding options.

use crate::Result;
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality, PulseShaper, GardnerTimingRecovery};
use openham_core::buffer::Complex;
use std::f64::consts::PI;

/// PSK constellation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PskType {
    Bpsk,       // 1 bit per symbol
    Qpsk,       // 2 bits per symbol
    Psk8,       // 3 bits per symbol
    Psk16,      // 4 bits per symbol
}

impl PskType {
    /// Get bits per symbol for this PSK type
    pub fn bits_per_symbol(self) -> usize {
        match self {
            PskType::Bpsk => 1,
            PskType::Qpsk => 2,
            PskType::Psk8 => 3,
            PskType::Psk16 => 4,
        }
    }
    
    /// Get number of constellation points
    pub fn constellation_size(self) -> usize {
        1 << self.bits_per_symbol()
    }
    
    /// Map bits to constellation point
    pub fn map_bits_to_symbol(self, bits: u8) -> Complex {
        let points = self.constellation_size();
        let index = bits as usize % points;
        let angle = 2.0 * PI * index as f64 / points as f64;
        
        match self {
            PskType::Bpsk => {
                // BPSK: 0° and 180°
                if index == 0 { Complex::new(1.0, 0.0) } else { Complex::new(-1.0, 0.0) }
            },
            PskType::Qpsk => {
                // QPSK: 45°, 135°, 225°, 315° (offset QPSK)
                let phase = angle + PI / 4.0;
                Complex::new(phase.cos(), phase.sin())
            },
            PskType::Psk8 => {
                // 8PSK: 0°, 45°, 90°, 135°, 180°, 225°, 270°, 315°
                Complex::new(angle.cos(), angle.sin())
            },
            PskType::Psk16 => {
                // 16PSK: evenly spaced around unit circle
                Complex::new(angle.cos(), angle.sin())
            },
        }
    }
    
    /// Map constellation point to bits (hard decision)
    pub fn map_symbol_to_bits(self, symbol: Complex) -> u8 {
        let points = self.constellation_size();
        let phase = symbol.imag.atan2(symbol.real);
        
        // Normalize phase to [0, 2π]
        let normalized_phase = if phase < 0.0 { phase + 2.0 * PI } else { phase };
        
        // Adjust for constellation offset
        let adjusted_phase = match self {
            PskType::Qpsk => normalized_phase - PI / 4.0,
            _ => normalized_phase,
        };
        
        // Map to nearest constellation point
        let angle_per_point = 2.0 * PI / points as f64;
        let index = ((adjusted_phase + angle_per_point / 2.0) / angle_per_point).floor() as usize % points;
        
        index as u8
    }
}

/// PSK modulator configuration
#[derive(Debug, Clone)]
pub struct PskConfig {
    pub psk_type: PskType,
    pub differential: bool,  // Use differential encoding
    pub gray_coding: bool,   // Use Gray coding for bit mapping
}

impl PskConfig {
    /// Create BPSK configuration
    pub fn bpsk() -> Self {
        Self {
            psk_type: PskType::Bpsk,
            differential: false,
            gray_coding: true,
        }
    }
    
    /// Create QPSK configuration
    pub fn qpsk() -> Self {
        Self {
            psk_type: PskType::Qpsk,
            differential: false,
            gray_coding: true,
        }
    }
    
    /// Create 8PSK configuration
    pub fn psk8() -> Self {
        Self {
            psk_type: PskType::Psk8,
            differential: false,
            gray_coding: true,
        }
    }
    
    /// Create differential QPSK configuration
    pub fn dqpsk() -> Self {
        Self {
            psk_type: PskType::Qpsk,
            differential: true,
            gray_coding: true,
        }
    }
}

/// Enhanced PSK modulator
pub struct PskModulator {
    config: ModulationConfig,
    psk_config: PskConfig,
    pulse_shaper: PulseShaper,
    phase: f64,
    sample_counter: f64,
    previous_symbol: Complex,
}

impl PskModulator {
    /// Create a new PSK modulator
    pub fn new(config: ModulationConfig, psk_config: PskConfig) -> Result<Self> {
        let samples_per_symbol = config.samples_per_symbol();
        let pulse_shaper = PulseShaper::root_raised_cosine(
            samples_per_symbol,
            config.rolloff_factor,
            config.filter_length,
        )?;
        
        Ok(Self {
            config,
            psk_config,
            pulse_shaper,
            phase: 0.0,
            sample_counter: 0.0,
            previous_symbol: Complex::new(1.0, 0.0),
        })
    }
    
    /// Apply Gray coding to the given bits based on PSK type
    fn apply_gray_coding(&self, bits: u8) -> u8 {
        if !self.psk_config.gray_coding {
            return bits;
        }
        match self.psk_config.psk_type {
            PskType::Bpsk => bits & 0x01,
            PskType::Qpsk => {
                // Binary -> Gray for 2-bit
                match bits & 0x03 {
                    0b00 => 0b00,
                    0b01 => 0b01,
                    0b10 => 0b11,
                    0b11 => 0b10,
                    _ => bits & 0x03,
                }
            }
            PskType::Psk8 => {
                // Binary -> Gray for 3-bit
                match bits & 0x07 {
                    0b000 => 0b000,
                    0b001 => 0b001,
                    0b010 => 0b011,
                    0b011 => 0b010,
                    0b100 => 0b110,
                    0b101 => 0b111,
                    0b110 => 0b101,
                    0b111 => 0b100,
                    _ => bits & 0x07,
                }
            }
            PskType::Psk16 => {
                // Binary -> Gray for 4-bit
                let b = bits & 0x0F;
                (b >> 1) ^ b
            }
        }
    }
    
    /// Apply differential encoding
    fn apply_differential_encoding(&mut self, symbol: Complex) -> Complex {
        if !self.psk_config.differential {
            return symbol;
        }
        
        // Differential encoding: current_symbol = previous_symbol * symbol
        let result = Complex::new(
            self.previous_symbol.real * symbol.real - self.previous_symbol.imag * symbol.imag,
            self.previous_symbol.real * symbol.imag + self.previous_symbol.imag * symbol.real,
        );
        
        self.previous_symbol = result;
        result
    }
    
    /// Generate carrier wave
    fn generate_carrier(&mut self, symbol: Complex) -> Complex {
        let omega = 2.0 * PI * self.config.carrier_frequency / self.config.sample_rate;
        let phase = omega * self.sample_counter;
        
        self.sample_counter += 1.0;
        
        let carrier = Complex::new(phase.cos(), phase.sin());
        
        // Multiply baseband symbol with carrier
        Complex::new(
            symbol.real * carrier.real - symbol.imag * carrier.imag,
            symbol.real * carrier.imag + symbol.imag * carrier.real,
        )
    }
}

impl Modulator for PskModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let bits_per_symbol = self.psk_config.psk_type.bits_per_symbol();
        let samples_per_symbol = self.config.samples_per_symbol() as usize;
        
        // Convert bytes to bit stream
        let mut bit_stream = Vec::new();
        for &byte in bits {
            for i in (0..8).rev() {
                bit_stream.push((byte >> i) & 1);
            }
        }
        
        // Process symbols
        for symbol_bits in bit_stream.chunks(bits_per_symbol) {
            // Pack bits into symbol value
            let mut symbol_value = 0u8;
            for &bit in symbol_bits {
                symbol_value = (symbol_value << 1) | bit;
            }
            
            // Apply Gray coding
            let gray_coded = self.apply_gray_coding(symbol_value);
            
            // Map to constellation
            let mut constellation_symbol = self.psk_config.psk_type.map_bits_to_symbol(gray_coded);
            
            // Apply differential encoding
            constellation_symbol = self.apply_differential_encoding(constellation_symbol);
            
            // Generate samples for this symbol
            for _ in 0..samples_per_symbol {
                let shaped = self.pulse_shaper.filter(constellation_symbol);
                let modulated = self.generate_carrier(shaped);
                output.push(modulated);
            }
        }
        
        Ok(())
    }
    
    fn samples_per_symbol(&self) -> usize {
        self.config.samples_per_symbol() as usize
    }
    
    fn symbol_rate(&self) -> f64 {
        self.config.symbol_rate
    }
    
    fn reset(&mut self) {
        self.pulse_shaper.reset();
        self.phase = 0.0;
        self.sample_counter = 0.0;
        self.previous_symbol = Complex::new(1.0, 0.0);
    }
}

/// Enhanced PSK demodulator
pub struct PskDemodulator {
    config: ModulationConfig,
    psk_config: PskConfig,
    pulse_shaper: PulseShaper,
    phase: f64,
    sample_counter: f64,
    is_sync: bool,
    signal_quality: SignalQuality,
    previous_symbol: Complex,
    constellation_points: Vec<Complex>,
}

impl PskDemodulator {
    /// Create a new PSK demodulator
    pub fn new(config: ModulationConfig, psk_config: PskConfig) -> Result<Self> {
        let samples_per_symbol = config.samples_per_symbol();
        let pulse_shaper = PulseShaper::root_raised_cosine(
            samples_per_symbol,
            config.rolloff_factor,
            config.filter_length,
        )?;
        
        // Pre-compute constellation points
        let constellation_size = psk_config.psk_type.constellation_size();
        let mut constellation_points = Vec::with_capacity(constellation_size);
        for i in 0..constellation_size {
            constellation_points.push(psk_config.psk_type.map_bits_to_symbol(i as u8));
        }
        
        Ok(Self {
            config,
            psk_config,
            pulse_shaper,
            phase: 0.0,
            sample_counter: 0.0,
            is_sync: false,
            signal_quality: SignalQuality::default(),
            previous_symbol: Complex::new(1.0, 0.0),
            constellation_points,
        })
    }
    
    /// Demodulate to baseband
    fn demodulate_to_baseband(&mut self, sample: Complex) -> Complex {
        let omega = 2.0 * PI * self.config.carrier_frequency / self.config.sample_rate;
        let phase = omega * self.sample_counter;
        
        self.sample_counter += 1.0;
        
        let lo = Complex::new(phase.cos(), -phase.sin());
        
        Complex::new(
            sample.real * lo.real - sample.imag * lo.imag,
            sample.real * lo.imag + sample.imag * lo.real,
        )
    }
    
    /// Find closest constellation point
    fn find_closest_constellation_point(&self, symbol: Complex) -> (u8, f64) {
        let mut min_distance = f64::INFINITY;
        let mut best_symbol = 0u8;
        
        for (i, &point) in self.constellation_points.iter().enumerate() {
            let diff = Complex::new(symbol.real - point.real, symbol.imag - point.imag);
            let distance = diff.norm_sqr();
            
            if distance < min_distance {
                min_distance = distance;
                best_symbol = i as u8;
            }
        }
        
        (best_symbol, min_distance)
    }
    
    /// Remove Gray coding
    fn remove_gray_coding(&self, bits: u8) -> u8 {
        if !self.psk_config.gray_coding {
            return bits;
        }
        
        match self.psk_config.psk_type {
            PskType::Bpsk => bits,
            PskType::Qpsk => {
                // Reverse Gray code mapping for QPSK
                match bits & 0x03 {
                    0b00 => 0b00,
                    0b01 => 0b01,
                    0b11 => 0b10,
                    0b10 => 0b11,
                    _ => bits,
                }
            },
            PskType::Psk8 => {
                // Reverse Gray code mapping for 8PSK
                match bits & 0x07 {
                    0b000 => 0b000,
                    0b001 => 0b001,
                    0b011 => 0b010,
                    0b010 => 0b011,
                    0b110 => 0b100,
                    0b111 => 0b101,
                    0b101 => 0b110,
                    0b100 => 0b111,
                    _ => bits,
                }
            },
            PskType::Psk16 => {
                // Reverse Gray coding for 16PSK
                let mut result = bits;
                result ^= result >> 1;
                result ^= result >> 2;
                result ^= result >> 4;
                result
            },
        }
    }
    
    /// Apply differential decoding
    fn apply_differential_decoding(&mut self, symbol: Complex) -> Complex {
        if !self.psk_config.differential {
            return symbol;
        }
        
        // Differential decoding: current_data = current_symbol * conjugate(previous_symbol)
        let result = Complex::new(
            symbol.real * self.previous_symbol.real + symbol.imag * self.previous_symbol.imag,
            symbol.imag * self.previous_symbol.real - symbol.real * self.previous_symbol.imag,
        );
        
        self.previous_symbol = symbol;
        result
    }
}

impl Demodulator for PskDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();

        let samples_per_symbol = self.config.samples_per_symbol() as usize;
        if samples_per_symbol == 0 || samples.is_empty() { return Ok(()); }
        let bits_per_symbol = self.psk_config.psk_type.bits_per_symbol();

        // Precompute baseband then perform Gardner timing recovery to extract symbols
        self.sample_counter = 0.0;
        let mut bb: Vec<Complex> = Vec::with_capacity(samples.len());
        for &s in samples {
            let b = self.demodulate_to_baseband(s);
            let shaped = self.pulse_shaper.filter(b);
            bb.push(shaped);
        }
        let mut gardner = GardnerTimingRecovery::new(self.config.samples_per_symbol(), 0.01, 0.707);
        let mut syms: Vec<Complex> = Vec::new();
        gardner.process(&bb, &mut syms)?;

        // Try multiple sampling offsets and pick the one with earliest preamble
        let sync: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E];
        let mut candidate_streams: Vec<Vec<u8>> = Vec::new();
        let mut best: Option<(usize, usize)> = None; // (offset, pos)

        self.previous_symbol = Complex::new(1.0, 0.0);
        let m = self.psk_config.psk_type.constellation_size();
        // Use symbol-spaced samples from Gardner; treat a single logical offset domain
        for offset in 0..1 {
            let mut tmp_bits: Vec<u8> = Vec::new();
            let mut tmp_bytes: Vec<u8> = Vec::new();
            // Decide a single global phase rotation for this offset
            let mut best_rotation: usize = 0;
            let mut best_sync_pos: Option<usize> = None;
            let mut best_avg_err = f64::INFINITY;

            // Evaluate each rotation by decoding a small window and searching for early sync
            for r in 0..m {
                let base = 2.0 * std::f64::consts::PI * (r as f64) / (m as f64);
                let rot = Complex::new(base.cos(), base.sin());
                let mut test_bytes: Vec<u8> = Vec::new();
                let mut test_bits: Vec<u8> = Vec::new();
                let mut i = 0;
                let mut err_acc = 0.0;
                let mut sym_count = 0usize;
                while i < syms.len() && sym_count < 128 { // analyze first ~128 symbols
                    let s = self.apply_differential_decoding(syms[i]);
                    let sr = Complex::new(s.real * rot.real - s.imag * rot.imag, s.real * rot.imag + s.imag * rot.real);
                    let (bits_cand, err) = self.find_closest_constellation_point(sr);
                    err_acc += err;
                    sym_count += 1;
                    let final_bits = self.remove_gray_coding(bits_cand);
                    for k in (0..bits_per_symbol).rev() {
                        let bit = (final_bits >> k) & 1;
                        test_bits.push(bit);
                        if test_bits.len() == 8 {
                            let mut byte = 0u8;
                            for (j, &b) in test_bits.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                            test_bytes.push(byte);
                            test_bits.clear();
                        }
                    }
                    i += 1;
                }
                if !test_bits.is_empty() {
                    let mut byte = 0u8;
                    for (j, &b) in test_bits.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                    test_bytes.push(byte);
                }
                // Look for sync pattern
                let sync: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E];
                let mut pos_opt = None;
                if test_bytes.len() >= sync.len() {
                    for pos in 0..=test_bytes.len() - sync.len() {
                        if &test_bytes[pos..pos+sync.len()] == sync { pos_opt = Some(pos); break; }
                    }
                }
                let avg_err = if sym_count > 0 { err_acc / sym_count as f64 } else { f64::INFINITY };
                match (best_sync_pos, pos_opt) {
                    (None, Some(p)) => { best_sync_pos = Some(p); best_rotation = r; best_avg_err = avg_err; }
                    (Some(bp), Some(p)) => { if p < bp { best_sync_pos = Some(p); best_rotation = r; best_avg_err = avg_err; } }
                    (None, None) => { if avg_err < best_avg_err { best_rotation = r; best_avg_err = avg_err; } }
                    _ => {}
                }
            }

            // Decode full stream with chosen rotation and decision-directed phase tracking
            let base = 2.0 * std::f64::consts::PI * (best_rotation as f64) / (m as f64);
            let mut theta = base; // initial phase
            let mut rot = Complex::new(theta.cos(), theta.sin());
            let mu = 0.05f64; // PLL step size
            let mut i = 0;
            while i < syms.len() {
                let s = self.apply_differential_decoding(syms[i]);
                // Apply current rotation
                let sr = Complex::new(s.real * rot.real - s.imag * rot.imag, s.real * rot.imag + s.imag * rot.real);
                let (bits_cand, err) = self.find_closest_constellation_point(sr);
                self.signal_quality.evm_percent = (err.sqrt() * 100.0).min(100.0);
                // Decision-directed phase error: angle between received and decided point
                let decided = self.psk_config.psk_type.map_bits_to_symbol(bits_cand);
                let e_re = sr.real * decided.real + sr.imag * decided.imag;
                let e_im = sr.imag * decided.real - sr.real * decided.imag;
                let error_angle = e_im.atan2(e_re);
                theta -= mu * error_angle; // negative feedback
                rot = Complex::new(theta.cos(), theta.sin());

                let final_bits = self.remove_gray_coding(bits_cand);
                for k in (0..bits_per_symbol).rev() {
                    let bit = (final_bits >> k) & 1;
                    tmp_bits.push(bit);
                    if tmp_bits.len() == 8 {
                        let mut byte = 0u8;
                        for (j, &b) in tmp_bits.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                        tmp_bytes.push(byte);
                        tmp_bits.clear();
                    }
                }
                i += 1;
            }
            if !tmp_bits.is_empty() {
                let mut byte = 0u8;
                for (j, &b) in tmp_bits.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                tmp_bytes.push(byte);
            }

            let mut pos_opt = None;
            if tmp_bytes.len() >= sync.len() {
                for pos in 0..=tmp_bytes.len() - sync.len() { if &tmp_bytes[pos..pos+sync.len()] == sync { pos_opt = Some(pos); break; } }
            }
            if let Some(pos) = pos_opt {
                match best { None => best = Some((offset, pos)), Some((_, bp)) if pos < bp => best = Some((offset, pos)), _ => {} }
            }
            candidate_streams.push(tmp_bytes);
        }

        if let Some((bo, _)) = best { output.extend_from_slice(&candidate_streams[bo]); self.is_sync = true; return Ok(()); }
        if let Some((bo, _)) = candidate_streams.iter().enumerate().max_by_key(|(_, v)| v.len()) {
            output.extend_from_slice(&candidate_streams[bo]); self.is_sync = true; return Ok(());
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
        self.pulse_shaper.reset();
        self.phase = 0.0;
        self.sample_counter = 0.0;
        self.is_sync = false;
        self.signal_quality = SignalQuality::default();
        self.previous_symbol = Complex::new(1.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psk_type_properties() {
        assert_eq!(PskType::Bpsk.bits_per_symbol(), 1);
        assert_eq!(PskType::Qpsk.bits_per_symbol(), 2);
        assert_eq!(PskType::Psk8.bits_per_symbol(), 3);
        assert_eq!(PskType::Psk16.bits_per_symbol(), 4);
        
        assert_eq!(PskType::Bpsk.constellation_size(), 2);
        assert_eq!(PskType::Qpsk.constellation_size(), 4);
        assert_eq!(PskType::Psk8.constellation_size(), 8);
        assert_eq!(PskType::Psk16.constellation_size(), 16);
    }

    #[test]
    fn test_constellation_mapping() {
        // Test BPSK
        let sym0 = PskType::Bpsk.map_bits_to_symbol(0);
        let sym1 = PskType::Bpsk.map_bits_to_symbol(1);
        assert!((sym0.real - 1.0).abs() < 1e-10);
        assert!((sym1.real + 1.0).abs() < 1e-10);
        
        // Test QPSK constellation points are on unit circle
        for i in 0..4 {
            let sym = PskType::Qpsk.map_bits_to_symbol(i);
            assert!((sym.norm() - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_psk_modulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let psk_config = PskConfig::qpsk();
        let _modulator = PskModulator::new(mod_config, psk_config).unwrap();
    }

    #[test]
    fn test_psk_demodulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let psk_config = PskConfig::qpsk();
        let _demodulator = PskDemodulator::new(mod_config, psk_config).unwrap();
    }

    #[test]
    fn test_gray_coding() {
        let config = PskConfig::qpsk();
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let modulator = PskModulator::new(mod_config, config).unwrap();
        
        // Test Gray code mapping for QPSK
        assert_eq!(modulator.apply_gray_coding(0b00), 0b00);
        assert_eq!(modulator.apply_gray_coding(0b01), 0b01);
        assert_eq!(modulator.apply_gray_coding(0b10), 0b11);
        assert_eq!(modulator.apply_gray_coding(0b11), 0b10);
    }

    #[test]
    fn test_psk_modulation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let psk_config = PskConfig::qpsk();
        let mut modulator = PskModulator::new(mod_config, psk_config).unwrap();
        
        let data = vec![0b11001010]; // Test data
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        
        // Should generate samples
        assert!(!output.is_empty());
        
        // Should have samples for 8 bits / 2 bits per symbol = 4 symbols
        let expected_samples = 4 * modulator.samples_per_symbol();
        assert_eq!(output.len(), expected_samples);
    }
}
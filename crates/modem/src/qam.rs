//! Quadrature Amplitude Modulation (QAM) implementation
//! 
//! Supports 16-QAM, 64-QAM, 256-QAM and other QAM variants
//! with constellation shaping and adaptive equalization.

use crate::Result;
use crate::common::{Modulator, Demodulator, ModulationConfig, SignalQuality, PulseShaper, GardnerTimingRecovery};
use openham_core::buffer::Complex;
use std::f64::consts::PI;

/// QAM constellation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QamType {
    Qam16,      // 4 bits per symbol
    Qam64,      // 6 bits per symbol
    Qam256,     // 8 bits per symbol
    Qam1024,    // 10 bits per symbol
}

impl QamType {
    /// Get bits per symbol for this QAM type
    pub fn bits_per_symbol(self) -> usize {
        match self {
            QamType::Qam16 => 4,
            QamType::Qam64 => 6,
            QamType::Qam256 => 8,
            QamType::Qam1024 => 10,
        }
    }
    
    /// Get constellation size
    pub fn constellation_size(self) -> usize {
        1 << self.bits_per_symbol()
    }
    
    /// Get constellation dimensions (I and Q levels)
    pub fn constellation_dimensions(self) -> (usize, usize) {
        match self {
            QamType::Qam16 => (4, 4),     // 4x4 grid
            QamType::Qam64 => (8, 8),     // 8x8 grid
            QamType::Qam256 => (16, 16),  // 16x16 grid
            QamType::Qam1024 => (32, 32), // 32x32 grid
        }
    }
    
    /// Map bits to QAM constellation point
    pub fn map_bits_to_symbol(self, bits: u16) -> Complex {
        let (i_levels, q_levels) = self.constellation_dimensions();
        let bits_per_dimension = (self.bits_per_symbol() / 2) as u16;
        
        // Split bits into I and Q components
        let i_bits = bits & ((1 << bits_per_dimension) - 1);
        let q_bits = (bits >> bits_per_dimension) & ((1 << bits_per_dimension) - 1);
        
        // Map to constellation levels (-max to +max)
        let max_level = (i_levels / 2) as f64 - 0.5;
        let i_level = (i_bits as f64) - max_level;
        let q_level = (q_bits as f64) - max_level;
        
        // Normalize for unit average power
        let normalization = match self {
            QamType::Qam16 => 1.0 / (10.0_f64).sqrt(),    // sqrt(10) normalization
            QamType::Qam64 => 1.0 / (42.0_f64).sqrt(),    // sqrt(42) normalization
            QamType::Qam256 => 1.0 / (170.0_f64).sqrt(),  // sqrt(170) normalization
            QamType::Qam1024 => 1.0 / (682.0_f64).sqrt(), // sqrt(682) normalization
        };
        
        Complex::new(i_level * normalization, q_level * normalization)
    }
    
    /// Map constellation point to bits (hard decision)
    pub fn map_symbol_to_bits(self, symbol: Complex) -> u16 {
        let (i_levels, q_levels) = self.constellation_dimensions();
        let bits_per_dimension = (self.bits_per_symbol() / 2) as u16;
        
        // Denormalize
        let normalization = match self {
            QamType::Qam16 => (10.0_f64).sqrt(),
            QamType::Qam64 => (42.0_f64).sqrt(),
            QamType::Qam256 => (170.0_f64).sqrt(),
            QamType::Qam1024 => (682.0_f64).sqrt(),
        };
        
        let i_val = symbol.real * normalization;
        let q_val = symbol.imag * normalization;
        
        // Map to nearest constellation points
        let max_level = (i_levels / 2) as f64 - 0.5;
        let i_level = (i_val + max_level).round().max(0.0).min((i_levels - 1) as f64) as u16;
        let q_level = (q_val + max_level).round().max(0.0).min((q_levels - 1) as f64) as u16;
        
        // Combine I and Q bits
        (q_level << bits_per_dimension) | i_level
    }
    
    /// Calculate constellation points for plotting/analysis
    pub fn get_constellation_points(self) -> Vec<Complex> {
        let mut points = Vec::new();
        let constellation_size = self.constellation_size();
        
        for i in 0..constellation_size {
            points.push(self.map_bits_to_symbol(i as u16));
        }
        
        points
    }
}

/// QAM modulator configuration
#[derive(Debug, Clone)]
pub struct QamConfig {
    pub qam_type: QamType,
    pub gray_coding: bool,      // Use Gray coding for bit mapping
    pub constellation_shaping: bool, // Apply constellation shaping
}

impl QamConfig {
    /// Create 16-QAM configuration
    pub fn qam16() -> Self {
        Self {
            qam_type: QamType::Qam16,
            gray_coding: true,
            constellation_shaping: false,
        }
    }
    
    /// Create 64-QAM configuration
    pub fn qam64() -> Self {
        Self {
            qam_type: QamType::Qam64,
            gray_coding: true,
            constellation_shaping: false,
        }
    }
    
    /// Create 256-QAM configuration
    pub fn qam256() -> Self {
        Self {
            qam_type: QamType::Qam256,
            gray_coding: true,
            constellation_shaping: true,
        }
    }
    
    /// Create high-order QAM with constellation shaping
    pub fn qam1024_shaped() -> Self {
        Self {
            qam_type: QamType::Qam1024,
            gray_coding: true,
            constellation_shaping: true,
        }
    }
}

/// QAM modulator
pub struct QamModulator {
    config: ModulationConfig,
    qam_config: QamConfig,
    pulse_shaper: PulseShaper,
    phase: f64,
    sample_counter: f64,
}

impl QamModulator {
    /// Create a new QAM modulator
    pub fn new(config: ModulationConfig, qam_config: QamConfig) -> Result<Self> {
        let samples_per_symbol = config.samples_per_symbol();
        let pulse_shaper = PulseShaper::root_raised_cosine(
            samples_per_symbol,
            config.rolloff_factor,
            config.filter_length,
        )?;
        
        Ok(Self {
            config,
            qam_config,
            pulse_shaper,
            phase: 0.0,
            sample_counter: 0.0,
        })
    }
    
    #[inline]
    fn bin_to_gray(x: u16) -> u16 { x ^ (x >> 1) }
    
    #[inline]
    fn gray_to_bin(mut g: u16) -> u16 { let mut b = g; while g > 0 { g >>= 1; b ^= g; } b }
    
    /// Apply constellation shaping
    fn apply_constellation_shaping(&self, symbol: Complex) -> Complex {
        if !self.qam_config.constellation_shaping {
            return symbol;
        }
        
        // Simple probabilistic shaping - reduce power of outer constellation points
        let power = symbol.norm_sqr();
        let max_power = match self.qam_config.qam_type {
            QamType::Qam16 => 2.0,
            QamType::Qam64 => 6.0,
            QamType::Qam256 => 14.0,
            QamType::Qam1024 => 30.0,
        };
        
        if power > max_power * 0.8 {
            // Scale down outer constellation points
            let scale = 0.9;
            Complex::new(symbol.real * scale, symbol.imag * scale)
        } else {
            symbol
        }
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

impl Modulator for QamModulator {
    fn modulate(&mut self, bits: &[u8], output: &mut Vec<Complex>) -> Result<()> {
        output.clear();
        
        let bits_per_symbol = self.qam_config.qam_type.bits_per_symbol();
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
            let mut symbol_value = 0u16;
            for &bit in symbol_bits {
                symbol_value = (symbol_value << 1) | (bit as u16);
            }
            
            // Apply per-dimension Gray coding
            let bits_per_dim = (bits_per_symbol / 2) as u16;
            let mask = (1u16 << bits_per_dim) - 1;
            let i_bits = symbol_value & mask;
            let q_bits = (symbol_value >> bits_per_dim) & mask;
            let i_mapped = if self.qam_config.gray_coding { Self::bin_to_gray(i_bits) } else { i_bits };
            let q_mapped = if self.qam_config.gray_coding { Self::bin_to_gray(q_bits) } else { q_bits };
            let gray_coded = (q_mapped << bits_per_dim) | i_mapped;
            
            // Map to constellation
            let mut constellation_symbol = self.qam_config.qam_type.map_bits_to_symbol(gray_coded);
            
            // Apply constellation shaping
            constellation_symbol = self.apply_constellation_shaping(constellation_symbol);
            
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
    }
}

/// QAM demodulator with adaptive equalization
pub struct QamDemodulator {
    config: ModulationConfig,
    qam_config: QamConfig,
    pulse_shaper: PulseShaper,
    phase: f64,
    sample_counter: f64,
    is_sync: bool,
    signal_quality: SignalQuality,
    equalizer_taps: Vec<Complex>,
    equalizer_enabled: bool,
    // Precomputed (constellation point, binary bits) pairs for consistent slicing
    const_points_bin: Vec<(Complex, u16)>,
}

impl QamDemodulator {
    /// Create a new QAM demodulator
    pub fn new(config: ModulationConfig, qam_config: QamConfig) -> Result<Self> {
        let samples_per_symbol = config.samples_per_symbol();
        let pulse_shaper = PulseShaper::root_raised_cosine(
            samples_per_symbol,
            config.rolloff_factor,
            config.filter_length,
        )?;
        
        // Build binary-mapped constellation: for each binary (i_bin,q_bin), produce point via per-axis Gray mapping if enabled
        let bits_per_dim = (qam_config.qam_type.bits_per_symbol() / 2) as u16;
        let levels = 1u16 << bits_per_dim;
        let mut const_points_bin: Vec<(Complex, u16)> = Vec::with_capacity((levels as usize).pow(2));
        for q_bin in 0..levels {
            for i_bin in 0..levels {
                let i_code = if qam_config.gray_coding { QamModulator::bin_to_gray(i_bin) } else { i_bin };
                let q_code = if qam_config.gray_coding { QamModulator::bin_to_gray(q_bin) } else { q_bin };
                let idx = ((q_code << bits_per_dim) | i_code) as u16;
                let pt = qam_config.qam_type.map_bits_to_symbol(idx);
                let bin_bits = ((q_bin << bits_per_dim) | i_bin) as u16;
                const_points_bin.push((pt, bin_bits));
            }
        }
        
        // Initialize simple 3-tap equalizer
        let equalizer_taps = vec![
            Complex::new(0.0, 0.0),
            Complex::new(1.0, 0.0), // Center tap
            Complex::new(0.0, 0.0),
        ];

        Ok(Self {
            config,
            qam_config,
            pulse_shaper,
            phase: 0.0,
            sample_counter: 0.0,
            is_sync: false,
            signal_quality: SignalQuality::default(),
            equalizer_taps,
            equalizer_enabled: false,
            const_points_bin,
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
    
    /// Slice a symbol via nearest-neighbor search on precomputed constellation; returns binary bits and EVM
    fn slice_symbol(&self, symbol: Complex) -> (u16, f64, Complex) {
        let mut best_bits = 0u16;
        let mut best_err = f64::INFINITY;
        let mut best_pt = Complex::new(0.0, 0.0);
        for (pt, bin_bits) in &self.const_points_bin {
            let dr = symbol.real - pt.real;
            let di = symbol.imag - pt.imag;
            let e = dr*dr + di*di;
            if e < best_err {
                best_err = e;
                best_bits = *bin_bits;
                best_pt = *pt;
            }
        }
        (best_bits, best_err, best_pt)
    }
    
    /// Update adaptive equalizer (simplified LMS algorithm)
    fn update_equalizer(&mut self, received: Complex, decided: Complex) {
        if !self.equalizer_enabled {
            return;
        }
        
        let error = Complex::new(
            received.real - decided.real,
            received.imag - decided.imag,
        );
        
        let mu = 0.01; // Step size
        
        // Update center tap (simplified single-tap update)
        self.equalizer_taps[1].real -= mu * error.real;
        self.equalizer_taps[1].imag -= mu * error.imag;
        
        // Normalize to prevent instability
        let tap_magnitude = self.equalizer_taps[1].norm();
        if tap_magnitude > 2.0 {
            self.equalizer_taps[1].real /= tap_magnitude;
            self.equalizer_taps[1].imag /= tap_magnitude;
        }
    }
}

impl Demodulator for QamDemodulator {
    fn demodulate(&mut self, samples: &[Complex], output: &mut Vec<u8>) -> Result<()> {
        output.clear();

        let samples_per_symbol = self.config.samples_per_symbol() as usize;
        if samples_per_symbol == 0 || samples.is_empty() { return Ok(()); }
        let bits_per_symbol = self.qam_config.qam_type.bits_per_symbol();

        // Precompute baseband
        self.sample_counter = 0.0;
        let mut bb: Vec<Complex> = Vec::with_capacity(samples.len());
        for &s in samples {
            let b = self.demodulate_to_baseband(s);
            let shaped = self.pulse_shaper.filter(b);
            bb.push(shaped);
        }

        // Perform Gardner timing recovery to get symbol-spaced sequence
        let mut gardner = GardnerTimingRecovery::new(self.config.samples_per_symbol(), 0.01, 0.707);
        let mut syms: Vec<Complex> = Vec::new();
        gardner.process(&bb, &mut syms)?;

        let sync: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E];
        let mut candidate_streams: Vec<Vec<u8>> = Vec::new();
        let mut best: Option<(usize, usize)> = None;

        for offset in 0..1 {
            let mut tmp_bits: Vec<u8> = Vec::new();
            let mut tmp_bytes: Vec<u8> = Vec::new();
            // Determine a single global rotation to align constellation bytes for this offset
            let m = 32; // denser rotation grid for QAM
            let mut best_rot: f64 = 0.0;
            let mut best_sync_pos: Option<usize> = None;
            let mut best_evm = f64::INFINITY;
            for r in 0..m {
                let base = 2.0 * std::f64::consts::PI * (r as f64) / (m as f64);
                let rot = Complex::new(base.cos(), base.sin());
                let mut bits_try: Vec<u8> = Vec::new();
                let mut bytes_try: Vec<u8> = Vec::new();
                let mut i = 0;
                let mut evm_acc = 0.0;
                let mut cnt = 0usize;
                while i < syms.len() && cnt < 256 {
                    let shaped = syms[i];
                    let rotated = Complex::new(
                        shaped.real * rot.real - shaped.imag * rot.imag,
                        shaped.real * rot.imag + shaped.imag * rot.real,
                    );
                    let (symbol_bits, err, _decided) = self.slice_symbol(rotated);
                    evm_acc += err;
                    cnt += 1;
                    for k in (0..bits_per_symbol).rev() {
                        let bit = (symbol_bits >> k) & 1;
                        bits_try.push(bit as u8);
                        if bits_try.len() == 8 {
                            let mut byte = 0u8;
                            for (j, &b) in bits_try.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                            bytes_try.push(byte);
                            bits_try.clear();
                        }
                    }
                    i += 1;
                }
                if !bits_try.is_empty() {
                    let mut byte = 0u8;
                    for (j, &b) in bits_try.iter().enumerate() { if b != 0 { byte |= 1 << (7 - j); } }
                    bytes_try.push(byte);
                }
                let mut pos_opt = None;
                if bytes_try.len() >= sync.len() {
                    for pos in 0..=bytes_try.len() - sync.len() { if &bytes_try[pos..pos+sync.len()] == sync { pos_opt = Some(pos); break; } }
                }
                let evm_avg = if cnt > 0 { evm_acc / cnt as f64 } else { f64::INFINITY };
                match (best_sync_pos, pos_opt) {
                    (None, Some(p)) => { best_sync_pos = Some(p); best_rot = base; best_evm = evm_avg; }
                    (Some(bp), Some(p)) => { if p < bp { best_sync_pos = Some(p); best_rot = base; best_evm = evm_avg; } }
                    _ => { if evm_avg < best_evm { best_rot = base; best_evm = evm_avg; } }
                }
            }
            let rot = Complex::new(best_rot.cos(), best_rot.sin());
            let mut theta: f64 = 0.0;
            let mu: f64 = 0.02;
            let mut i = 0;
            while i < syms.len() {
                let shaped = syms[i];
                // Coarse rotation
                let rb = Complex::new(
                    shaped.real * rot.real - shaped.imag * rot.imag,
                    shaped.real * rot.imag + shaped.imag * rot.real,
                );
                // Fine rotation via PLL correction (-theta)
                let ct = theta.cos();
                let st = theta.sin();
                let rotated = Complex::new(
                    rb.real * ct + rb.imag * st,
                    -rb.real * st + rb.imag * ct,
                );
                // Equalize
                let equalized = if self.equalizer_enabled {
                    Complex::new(
                        rotated.real * self.equalizer_taps[1].real - rotated.imag * self.equalizer_taps[1].imag,
                        rotated.real * self.equalizer_taps[1].imag + rotated.imag * self.equalizer_taps[1].real,
                    )
                } else { rotated };
                let (symbol_bits, err, decided_point) = self.slice_symbol(equalized);
                self.signal_quality.evm_percent = (err.sqrt() * 100.0 / 2.0).min(100.0);
                // Update equalizer toward decided point
                self.update_equalizer(equalized, decided_point);
                // Decision-directed phase error: rotated * conj(decided)
                let realp = equalized.real * decided_point.real + equalized.imag * decided_point.imag;
                let imagp = equalized.imag * decided_point.real - equalized.real * decided_point.imag;
                let err_phase = imagp.atan2(realp);
                theta += mu * err_phase;
                for k in (0..bits_per_symbol).rev() {
                    let bit = (symbol_bits >> k) & 1;
                    tmp_bits.push(bit as u8);
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
        
        // Reset equalizer
        self.equalizer_taps = vec![
            Complex::new(0.0, 0.0),
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 0.0),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qam_type_properties() {
        assert_eq!(QamType::Qam16.bits_per_symbol(), 4);
        assert_eq!(QamType::Qam64.bits_per_symbol(), 6);
        assert_eq!(QamType::Qam256.bits_per_symbol(), 8);
        assert_eq!(QamType::Qam1024.bits_per_symbol(), 10);
        
        assert_eq!(QamType::Qam16.constellation_size(), 16);
        assert_eq!(QamType::Qam64.constellation_size(), 64);
        assert_eq!(QamType::Qam256.constellation_size(), 256);
        assert_eq!(QamType::Qam1024.constellation_size(), 1024);
    }

    #[test]
    fn test_qam_constellation_mapping() {
        // Test 16-QAM constellation
        let points = QamType::Qam16.get_constellation_points();
        assert_eq!(points.len(), 16);
        
        // All points should be roughly normalized
        for point in &points {
            assert!(point.norm() > 0.1);
            assert!(point.norm() < 2.0);
        }
    }

    #[test]
    fn test_qam_bit_mapping_roundtrip() {
        // Test that bit mapping is reversible
        for bits in 0..16u16 {
            let symbol = QamType::Qam16.map_bits_to_symbol(bits);
            let recovered = QamType::Qam16.map_symbol_to_bits(symbol);
            assert_eq!(bits, recovered);
        }
    }

    #[test]
    fn test_qam_modulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let qam_config = QamConfig::qam16();
        let _modulator = QamModulator::new(mod_config, qam_config).unwrap();
    }

    #[test]
    fn test_qam_demodulator_creation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let qam_config = QamConfig::qam64();
        let _demodulator = QamDemodulator::new(mod_config, qam_config).unwrap();
    }

    #[test]
    fn test_qam_modulation() {
        let mod_config = ModulationConfig::new(48000.0, 1000.0, 1500.0).unwrap();
        let qam_config = QamConfig::qam16();
        let mut modulator = QamModulator::new(mod_config, qam_config).unwrap();
        
        let data = vec![0b11001010, 0b10110011]; // Test data
        let mut output = Vec::new();
        
        modulator.modulate(&data, &mut output).unwrap();
        
        // Should generate samples
        assert!(!output.is_empty());
        
        // Should have samples for 16 bits / 4 bits per symbol = 4 symbols
        let expected_samples = 4 * modulator.samples_per_symbol();
        assert_eq!(output.len(), expected_samples);
    }
}
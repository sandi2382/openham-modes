//! OpenHam - Unified transmission and reception tool for digital modes
//! 
//! This is the main entry point for OpenHam digital modes operations

use clap::{Parser, Subcommand, ValueEnum};
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

use openham_core::buffer::Complex;
use openham_modem::prelude::*;
use openham_modem::psk::PskType;
use openham_modem::afsk::AfskConfig;
use openham_frame::prelude::*;
use openham_codecs::prelude::*;
use openham_codecs::text::{TextCodec, AsciiCodec};

/// OpenHam unified digital modes tool
#[derive(Parser)]
#[command(name = "openham")]
#[command(about = "OpenHam digital modes transmission and reception")]
#[command(version = "0.1.0")]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Enable debug output
    #[arg(long, global = true)]
    debug: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transmit data
    Tx(TransmitConfig),
    /// Receive and decode
    Rx(ReceiveConfig),
    /// Generate test signals
    Generate(GenerateConfig),
    /// Show capabilities
    Info,
}

/// Transmission configuration
#[derive(Parser, Clone, Serialize, Deserialize)]
pub struct TransmitConfig {
    /// Output audio file
    #[arg(short, long)]
    pub output: PathBuf,
    
    /// Input text to transmit
    #[arg(short, long)]
    pub text: Option<String>,
    
    /// Input file to transmit
    #[arg(short, long)]
    pub file: Option<PathBuf>,
    
    /// Station callsign
    #[arg(short, long, default_value = "NOCALL")]
    pub callsign: String,
    
    /// Modulation scheme
    #[arg(short, long, default_value = "bpsk")]
    pub modulation: ModulationType,

    /// PSK variant (when -m psk)
    #[arg(long, value_name = "TYPE", help = "PSK type: bpsk|qpsk|psk8|8psk|psk16|16psk")]
    pub psk_type: Option<String>,

    /// Enable differential PSK encoding
    #[arg(long, default_value_t = false)]
    pub psk_differential: bool,

    /// Disable Gray coding for PSK
    #[arg(long, default_value_t = false)]
    pub psk_no_gray: bool,

    /// QAM variant (when -m qam)
    #[arg(long, value_name = "ORDER", help = "QAM order: 16|64|256|1024 or names qam16|qam64|qam256|qam1024")]
    pub qam_type: Option<String>,

    /// AFSK profile (when -m afsk)
    #[arg(long, value_enum)]
    pub afsk_profile: Option<AfskProfile>,

    /// Experimental mode selection (when -m experimental)
    #[arg(long, value_enum)]
    pub experimental_mode: Option<ExperimentalMode>,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Center frequency in Hz
    #[arg(long, default_value = "1500")]
    pub center_freq: f64,
    
    /// Symbol rate in Hz
    #[arg(long, default_value = "125")]
    pub symbol_rate: f64,
    
    /// Enable CW preamble
    #[arg(long)]
    pub cw_preamble: bool,
    
    /// CW speed in WPM
    #[arg(long, default_value = "20")]
    pub cw_wpm: u32,
    
    /// CW tone frequency
    #[arg(long, default_value = "600")]
    pub cw_freq: f64,
    
    /// Enable pink noise burst
    #[arg(long)]
    pub pink_noise: bool,
    
    /// Power level (0.0-1.0)
    #[arg(long, default_value = "0.8")]
    pub power: f64,

    /// Text codec to use for payload encoding
    #[arg(long, value_enum, default_value = "huffman")]
    pub text_codec: TextCodecType,

    /// Optional pre-recorded voice ID WAV file to prepend
    #[arg(long, value_name = "WAV_PATH")]
    pub voice_id: Option<PathBuf>,
}

/// Reception configuration
#[derive(Parser, Clone)]
pub struct ReceiveConfig {
    /// Input audio file
    #[arg(short, long)]
    pub input: PathBuf,
    
    /// Output text file
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// Modulation scheme
    #[arg(short, long, default_value = "bpsk")]
    pub modulation: String,

    /// PSK variant (when -m psk)
    #[arg(long, value_name = "TYPE", help = "PSK type: bpsk|qpsk|psk8|8psk|psk16|16psk")]
    pub psk_type: Option<String>,

    /// Assume differential PSK encoding (rx)
    #[arg(long, default_value_t = false)]
    pub psk_differential: bool,

    /// Disable Gray coding for PSK (rx)
    #[arg(long, default_value_t = false)]
    pub psk_no_gray: bool,

    /// QAM variant (when -m qam)
    #[arg(long, value_name = "ORDER", help = "QAM order: 16|64|256|1024 or names qam16|qam64|qam256|qam1024")]
    pub qam_type: Option<String>,

    /// AFSK profile (when -m afsk)
    #[arg(long, value_enum)]
    pub afsk_profile: Option<AfskProfile>,

    /// Experimental mode selection (when -m experimental)
    #[arg(long, value_enum)]
    pub experimental_mode: Option<ExperimentalMode>,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Center frequency in Hz
    #[arg(long, default_value = "1500")]
    pub center_freq: f64,
    
    /// Symbol rate in Hz
    #[arg(long, default_value = "125")]
    pub symbol_rate: f64,
    
    /// Enable auto-detection
    #[arg(long)]
    pub auto_detect: bool,

    /// Optional text codec hint to attempt first
    #[arg(long, value_enum)]
    pub text_codec: Option<TextCodecType>,
}

/// Signal generation configuration
#[derive(Parser, Clone)]
pub struct GenerateConfig {
    /// Output audio file
    #[arg(short, long)]
    pub output: PathBuf,
    
    /// Signal type
    #[arg(short, long, default_value = "sine")]
    pub signal: SignalType,
    
    /// Duration in seconds
    #[arg(short, long, default_value = "5.0")]
    pub duration: f64,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Frequency in Hz
    #[arg(short, long, default_value = "1000")]
    pub frequency: f64,
}

/// Supported modulation types
#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum ModulationType {
    Bpsk,
    Fsk,
    Ofdm,
    Afsk,
    Psk,
    Qam,
    Experimental,
}

// (PSK/QAM use free-form string parsing for broader alias support)

/// AFSK profiles
#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum AfskProfile { Bell202, Bell103, Vhf, Hf }

/// Experimental modes
#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum ExperimentalMode { Fhss, Rotating, Chaos, Waterfall }

// Helper parsers for flexible CLI values
fn parse_psk_type(s: &str) -> Option<PskType> {
    match s.to_ascii_lowercase().as_str() {
        "bpsk" => Some(PskType::Bpsk),
        "qpsk" => Some(PskType::Qpsk),
        "8psk" | "psk8" => Some(PskType::Psk8),
        "16psk" | "psk16" => Some(PskType::Psk16),
        _ => None,
    }
}

fn parse_qam_config(s: &str) -> Option<QamConfig> {
    match s.to_ascii_lowercase().as_str() {
        "16" | "qam16" => Some(QamConfig::qam16()),
        "64" | "qam64" => Some(QamConfig::qam64()),
        "256" | "qam256" => Some(QamConfig::qam256()),
        "1024" | "qam1024" => Some(QamConfig::qam1024_shaped()),
        _ => None,
    }
}

/// Signal generation types
#[derive(ValueEnum, Clone, Debug)]
pub enum SignalType {
    Sine,
    Noise,
    Sweep,
    Morse,
}

/// Supported text codecs
#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum TextCodecType {
    Huffman,
    Ascii,
}

/// Simple transmission coordinator
pub struct SimpleTransmitter {
    config: TransmitConfig,
}

impl SimpleTransmitter {
    pub fn new(config: TransmitConfig) -> Result<Self> {
        Ok(Self { config })
    }
    
    pub fn transmit(&mut self) -> Result<Vec<Complex>> {
        let start_time = Instant::now();
        let mut all_samples = Vec::new();
        
        // Pink noise burst if requested
        if self.config.pink_noise {
            let pink_samples = self.generate_pink_noise(0.5)?;
            all_samples.extend(pink_samples);
            info!("Added pink noise burst");
        }
        
        // CW preamble if requested
        if self.config.cw_preamble {
            let cw_samples = self.generate_cw_preamble()?;
            all_samples.extend(cw_samples);
            info!("Added CW preamble");
        }

        // Optional voice announcement prepend
        if let Some(path) = &self.config.voice_id {
            match self.generate_voice_announcement(path) {
                Ok(vs) => {
                    all_samples.extend(vs);
                    info!("Added voice ID announcement");
                }
                Err(e) => {
                    warn!("Voice ID not added: {}", e);
                }
            }
        }
        
        // Main data transmission
        let data_samples = self.generate_data_transmission()?;
        all_samples.extend(data_samples);
        info!("Added data transmission");
        
        // Apply power scaling
        if self.config.power != 1.0 {
            for sample in &mut all_samples {
                *sample = Complex::new(sample.real * self.config.power, sample.imag * self.config.power);
            }
        }
        
        let elapsed = start_time.elapsed();
        info!("Transmission generated in {:?}: {} samples", elapsed, all_samples.len());
        
        Ok(all_samples)
    }
    
    fn generate_pink_noise(&self, duration: f64) -> Result<Vec<Complex>> {
        let samples_count = (self.config.sample_rate * duration) as usize;
        let mut samples = Vec::with_capacity(samples_count);
        
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        for _ in 0..samples_count {
            let white: f64 = rng.gen_range(-0.1..0.1);
            samples.push(Complex::new(white, 0.0));
        }
        
        Ok(samples)
    }
    
    fn generate_cw_preamble(&self) -> Result<Vec<Complex>> {
        let cw_config = CwConfig::new(self.config.cw_wpm, self.config.cw_freq, self.config.sample_rate);
        let cw_gen = CwGenerator::new(cw_config);
        
        let preamble_text = format!("DE {} ", self.config.callsign);
        let audio_samples = cw_gen.generate_cw_audio(&preamble_text);
        
        Ok(audio_samples.into_iter()
            .map(|s| Complex::new(s as f64, 0.0))
            .collect())
    }
    
    fn generate_data_transmission(&self) -> Result<Vec<Complex>> {
        // Get input text
        let text = self.get_input_text()?;
        info!("Transmitting: {} characters", text.len());
        
        // Create modulation config
        let mod_config = ModulationConfig::new(
            self.config.sample_rate,
            self.config.symbol_rate,
            self.config.center_freq,
        )?;
        
        // Create modulator
        let mut modulator: Box<dyn Modulator> = match self.config.modulation {
            ModulationType::Bpsk => Box::new(BpskModulator::new(mod_config)?),
            ModulationType::Fsk => Box::new(FskModulator::new(mod_config)?),
            ModulationType::Ofdm => {
                let ofdm_config = OfdmConfig::amateur_radio_64();
                Box::new(OfdmModulator::new(mod_config, ofdm_config)?)
            },
            ModulationType::Afsk => {
                let profile = self.config.afsk_profile.clone().unwrap_or(AfskProfile::Bell202);
                let afsk_cfg = match profile {
                    AfskProfile::Bell202 => AfskConfig::bell_202(),
                    AfskProfile::Bell103 => AfskConfig::bell_103(),
                    AfskProfile::Vhf => AfskConfig::vhf_packet(),
                    AfskProfile::Hf => AfskConfig::hf_packet(),
                };
                Box::new(AfskModulator::new(mod_config, afsk_cfg)?)
            },
            ModulationType::Psk => {
                let cli_t = self.config.psk_type.clone().unwrap_or_else(|| "qpsk".to_string());
                let psk_type = parse_psk_type(&cli_t).unwrap_or(PskType::Qpsk);
                let differential = self.config.psk_differential || matches!(psk_type, PskType::Psk8 | PskType::Psk16);
                let psk_cfg = PskConfig { psk_type, differential, gray_coding: !self.config.psk_no_gray };
                Box::new(PskModulator::new(mod_config, psk_cfg)?)
            },
            ModulationType::Qam => {
                let cli_t = self.config.qam_type.clone().unwrap_or_else(|| "16".to_string());
                let qam_cfg = parse_qam_config(&cli_t).unwrap_or(QamConfig::qam16());
                Box::new(QamModulator::new(mod_config, qam_cfg)?)
            },
            ModulationType::Experimental => {
                match self.config.experimental_mode.clone().unwrap_or(ExperimentalMode::Fhss) {
                    ExperimentalMode::Fhss => Box::new(FrequencyHoppingModulator::with_default_hops(mod_config)),
                    ExperimentalMode::Rotating => Box::new(RotatingConstellationModulator::new(mod_config, 15.0)),
                    ExperimentalMode::Chaos => Box::new(ChaosModulator::new(mod_config, ChaosConfig::default())),
                    ExperimentalMode::Waterfall => Box::new(WaterfallModulator::new(mod_config, 500.0, 0.1)),
                }
            },
        };
        
        // Encode text with selected codec
        let (encoded_data, compression_ratio) = match self.config.text_codec {
            TextCodecType::Huffman => {
                let mut codec = HuffmanCodec::new_english();
                let encoded = codec.encode(&text).map_err(|e| anyhow::anyhow!("Huffman encoding failed: {}", e))?;
                // Compression ratio: encoded bytes / original text chars
                // Lower is better (less bytes per char = more compression)
                // 1.0 means no compression (1 byte per char)
                let ratio = (encoded.len() as f64) / (text.len() as f64 * 1.0);
                (encoded, ratio)
            }
            TextCodecType::Ascii => {
                let mut codec = AsciiCodec;
                let encoded = codec.encode(&text).map_err(|e| anyhow::anyhow!("ASCII encoding failed: {}", e))?;
                let ratio = 1.0;
                (encoded, ratio)
            }
        };
        
        eprintln!("Original text: '{}' ({} chars)", text, text.len());
        eprintln!("Encoded: {} bytes", encoded_data.len());
        eprintln!("Compression ratio: {:.2}", compression_ratio);
        
        // Create frame
        let frame = Frame::new(1, 0, encoded_data, 0);
        let frame_bytes = frame.to_bytes();
        
        eprintln!("Frame created: {} bytes", frame_bytes.len());
        eprintln!("Frame header should show payload length: {}", frame.payload.len());
        eprintln!("Frame bytes: {}", frame_bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        
        // Add sync preamble before frame data
        let mut tx_data = Vec::new();
        
        // Add sync pattern: alternating 0x55 (01010101) for bit sync, then 0xAA (10101010), then frame start marker
        let sync_pattern = vec![0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E]; // Standard HDLC-like sync
        tx_data.extend_from_slice(&sync_pattern);
        tx_data.extend_from_slice(&frame_bytes);
        
        eprintln!("Transmitting {} bytes (sync: {}, frame: {})", 
                 tx_data.len(), sync_pattern.len(), frame_bytes.len());
        
        // Modulate
        let mut samples = Vec::new();
        modulator.modulate(&tx_data, &mut samples)?;
        
        Ok(samples)
    }
    
    fn get_input_text(&self) -> Result<String> {
        if let Some(ref text) = self.config.text {
            Ok(text.clone())
        } else if let Some(ref file) = self.config.file {
            std::fs::read_to_string(file)
                .with_context(|| format!("Failed to read file: {:?}", file))
        } else {
            Ok(format!("Hello from {} using OpenHam", self.config.callsign))
        }
    }

    fn generate_voice_announcement(&self, path: &PathBuf) -> Result<Vec<Complex>> {
        let announcer = VoiceAnnouncer::new(self.config.sample_rate);
        let samples_f32 = announcer
            .load_announcement_from_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to load voice ID: {}", e))?;
        Ok(samples_f32
            .into_iter()
            .map(|s| Complex::new(s as f64, 0.0))
            .collect())
    }
}

/// Simple receiver
pub struct SimpleReceiver {
    config: ReceiveConfig,
}

impl SimpleReceiver {
    pub fn new(config: ReceiveConfig) -> Result<Self> {
        Ok(Self { config })
    }
    
    pub fn receive(&mut self, samples: &[Complex]) -> Result<Vec<String>> {
        let mod_config = ModulationConfig::new(
            self.config.sample_rate,
            self.config.symbol_rate,
            self.config.center_freq,
        )?;
        
        let mut demodulators: Vec<(String, Box<dyn Demodulator>)> = Vec::new();
        
        if self.config.auto_detect {
            // Try multiple demodulators
            demodulators.push(("BPSK".to_string(), Box::new(BpskDemodulator::new(mod_config.clone())?)));
            demodulators.push(("FSK".to_string(), Box::new(FskDemodulator::new(mod_config.clone())?)));
            let ofdm_config = OfdmConfig::amateur_radio_64();
            demodulators.push(("OFDM".to_string(), Box::new(OfdmDemodulator::new(mod_config.clone(), ofdm_config)?)));
            // Common additional demodulators
            demodulators.push(("QPSK".to_string(), Box::new(PskDemodulator::new(mod_config.clone(), PskConfig::qpsk())?)));
            demodulators.push(("16QAM".to_string(), Box::new(QamDemodulator::new(mod_config.clone(), QamConfig::qam16())?)));
            demodulators.push(("AFSK".to_string(), Box::new(AfskDemodulator::new(mod_config.clone(), AfskConfig::bell_202())?)));
        } else {
            // Single demodulator
            let demodulator: Box<dyn Demodulator> = match self.config.modulation.as_str() {
                "bpsk" => Box::new(BpskDemodulator::new(mod_config)?),
                "fsk" => Box::new(FskDemodulator::new(mod_config)?),
                "ofdm" => {
                    let ofdm_config = OfdmConfig::amateur_radio_64();
                    Box::new(OfdmDemodulator::new(mod_config, ofdm_config)?)
                },
                "afsk" => {
                    let profile = self.config.afsk_profile.clone().unwrap_or(AfskProfile::Bell202);
                    let afsk_cfg = match profile {
                        AfskProfile::Bell202 => AfskConfig::bell_202(),
                        AfskProfile::Bell103 => AfskConfig::bell_103(),
                        AfskProfile::Vhf => AfskConfig::vhf_packet(),
                        AfskProfile::Hf => AfskConfig::hf_packet(),
                    };
                    Box::new(AfskDemodulator::new(mod_config, afsk_cfg)?)
                },
                "psk" => {
                    let kind = self.config.psk_type.clone().unwrap_or_else(|| "qpsk".to_string());
                    let psk_type = parse_psk_type(&kind).unwrap_or(PskType::Qpsk);
                    let differential = self.config.psk_differential || matches!(psk_type, PskType::Psk8 | PskType::Psk16);
                    let psk_cfg = PskConfig { psk_type, differential, gray_coding: !self.config.psk_no_gray };
                    Box::new(PskDemodulator::new(mod_config, psk_cfg)?)
                },
                "qam" => {
                    let order = self.config.qam_type.clone().unwrap_or_else(|| "16".to_string());
                    let qam_cfg = parse_qam_config(&order).unwrap_or(QamConfig::qam16());
                    Box::new(QamDemodulator::new(mod_config, qam_cfg)?)
                },
                "experimental" => {
                    warn!("Experimental demodulators are not supported; using BPSK demodulator");
                    Box::new(BpskDemodulator::new(mod_config)?)
                },
                _ => {
                    warn!("Unknown modulation '{}'", self.config.modulation);
                    Box::new(BpskDemodulator::new(mod_config)?)
                }
            };
            demodulators.push((self.config.modulation.clone(), demodulator));
        }
        
        let mut decoded_messages = Vec::new();
        
        // Try each demodulator
        for (name, demodulator) in &mut demodulators {
            let mut output_data = Vec::new();
            
            // Add debug output
            eprintln!("Trying {} demodulator on {} samples", name, samples.len());
            
            match demodulator.demodulate(samples, &mut output_data) {
                Ok(()) if !output_data.is_empty() => {
                    eprintln!("{} demodulator produced {} bytes", name, output_data.len());
                    debug!("{} demodulator produced {} bytes", name, output_data.len());
                    
                    // Print first few bytes for debugging
                    if output_data.len() > 0 {
                        let preview: Vec<String> = output_data.iter().take(20)
                            .map(|b| format!("{:02x}", b)).collect();
                        eprintln!("First bytes: {}", preview.join(" "));
                    }
                    
                    // Look for sync pattern and extract frame (robust across bit alignments)
                    if let Some(frame_bytes) = extract_frame_data_any_alignment(&output_data) {
                        eprintln!("Found frame sync; trying {} bytes after sync", frame_bytes.len());
                        // Try to decode frame from data after sync pattern
                        match Frame::from_bytes(&frame_bytes) {
                            Ok(frame) => {
                                eprintln!("Successfully decoded {} frame with {} bytes", name, frame.payload.len());
                                eprintln!("Frame payload: {}", frame.payload.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                                info!("Decoded {} frame with {} bytes", name, frame.payload.len());
                                
                                let text = self.decode_payload(&frame.payload)?;
                                decoded_messages.push(format!("[{}] {}", name, text));
                            },
                            Err(e1) => {
                                eprintln!("Frame decode failed after sync: {}", e1);
                                // Try with bitwise inversion
                                let inverted_data: Vec<u8> = frame_bytes.iter().map(|b| !b).collect();
                                eprintln!("Trying inverted frame data: {}", inverted_data.iter().take(20).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                                match Frame::from_bytes(&inverted_data) {
                                    Ok(frame) => {
                                        eprintln!("Successfully decoded {} frame with inverted data ({} bytes)", name, frame.payload.len());
                                        eprintln!("Inverted frame payload: {}", frame.payload.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
                                        info!("Decoded {} frame with inverted data ({} bytes)", name, frame.payload.len());
                                        
                                        let text = self.decode_payload(&frame.payload)?;
                                        decoded_messages.push(format!("[{}] {}", name, text));
                                    },
                                    Err(e2) => {
                                        eprintln!("Frame decode failed for {} even after inversion ({} / {})", name, e1, e2);
                                        debug!("Frame decode failed for {} (normal and inverted): {}", name, e2);
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("No frame sync pattern found in {} bytes", output_data.len());
                    }
                },
                Ok(()) => {
                    eprintln!("{} demodulator produced no data", name);
                    debug!("{} demodulator produced no data", name);
                },
                Err(e) => {
                    eprintln!("{} demodulator failed: {}", name, e);
                    debug!("{} demodulator failed: {}", name, e);
                }
            }
        }
        
        Ok(decoded_messages)
    }
    
    fn decode_payload(&self, data: &[u8]) -> Result<String> {
        eprintln!("Decoding payload: {} bytes: {}", data.len(), 
                 data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        
        // Determine preferred order based on hint
        let mut attempts: Vec<&str> = Vec::new();
        if let Some(ref hint) = self.config.text_codec {
            match hint {
                TextCodecType::Huffman => attempts.extend(["huffman", "utf8", "hex"]),
                TextCodecType::Ascii => attempts.extend(["utf8", "huffman", "hex"]),
            }
        } else {
            attempts.extend(["huffman", "utf8", "hex"]);
        }

        for method in attempts {
            match method {
                "huffman" => {
                    let mut h = HuffmanCodec::new_english();
                    match h.decode(data) {
                        Ok(text) => {
                            eprintln!("Huffman decode successful: '{}'", text);
                            return Ok(text);
                        }
                        Err(e) => eprintln!("Huffman decode failed: {}", e),
                    }
                }
                "utf8" => {
                    match String::from_utf8(data.to_vec()) {
                        Ok(text) => {
                            eprintln!("UTF-8 decode successful: '{}'", text);
                            return Ok(text);
                        }
                        Err(e) => eprintln!("UTF-8 decode failed: {}", e),
                    }
                }
                _ => {}
            }
        }
        
        // Fallback to hex
        let hex_result = format!("HEX:{}", data.iter().map(|b| format!("{:02x}", b)).collect::<String>());
        eprintln!("Using hex fallback: '{}'", hex_result);
        Ok(hex_result)
    }
}

/// Find the start of frame data after sync pattern
fn extract_frame_data_any_alignment(data: &[u8]) -> Option<Vec<u8>> {
    // Define sync patterns
    const SYNC: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0xAA, 0xAA, 0x7E, 0x7E];
    const SYNC_INV: [u8; 8] = [0xAA, 0xAA, 0xAA, 0xAA, 0x55, 0x55, 0x81, 0x81];

    // Helper to search given bit order (msb_first=true uses (7-i), false uses i)
    let try_search = |msb_first: bool| -> Option<Vec<u8>> {
        // Convert to bit vector with chosen bit order
        let mut bits: Vec<u8> = Vec::with_capacity(data.len() * 8);
        for &b in data {
            if msb_first {
                for i in (0..8).rev() { bits.push(((b >> i) & 1) as u8); }
            } else {
                for i in 0..8 { bits.push(((b >> i) & 1) as u8); }
            }
        }

        for shift in 0..8 {
            if bits.len() <= shift { break; }
            let aligned_bits = &bits[shift..];
            let aligned_len_bytes = aligned_bits.len() / 8;
            if aligned_len_bytes < SYNC.len() { continue; }

            // Repack into bytes
            let mut aligned_bytes: Vec<u8> = Vec::with_capacity(aligned_len_bytes);
            for chunk in aligned_bits.chunks(8) {
                if chunk.len() < 8 { break; }
                let mut val = 0u8;
                if msb_first {
                    for (i, &bit) in chunk.iter().enumerate() { if bit != 0 { val |= 1 << (7 - i); } }
                } else {
                    for (i, &bit) in chunk.iter().enumerate() { if bit != 0 { val |= 1 << i; } }
                }
                aligned_bytes.push(val);
            }

            // Search patterns in aligned bytes
            let search = |hay: &[u8], needle: &[u8]| -> Option<usize> {
                if hay.len() < needle.len() { return None; }
                for i in 0..=(hay.len() - needle.len()) {
                    if &hay[i..i + needle.len()] == needle { return Some(i); }
                }
                None
            };

            if let Some(pos) = search(&aligned_bytes, &SYNC) {
                return Some(aligned_bytes[pos + SYNC.len()..].to_vec());
            }
            if let Some(pos) = search(&aligned_bytes, &SYNC_INV) {
                return Some(aligned_bytes[pos + SYNC_INV.len()..].to_vec());
            }
            if let Some(pos) = search(&aligned_bytes, &[0x7E, 0x7E]) {
                return Some(aligned_bytes[pos + 2..].to_vec());
            }
            if let Some(pos) = search(&aligned_bytes, &[0x81, 0x81]) {
                return Some(aligned_bytes[pos + 2..].to_vec());
            }
        }
        None
    };

    // Try MSB-first, then LSB-first as fallback
    try_search(true).or_else(|| try_search(false))
}

/// Generate test signals
fn generate_test_signal(config: &GenerateConfig) -> Result<Vec<Complex>> {
    let samples_count = (config.sample_rate * config.duration) as usize;
    let mut samples = Vec::with_capacity(samples_count);
    
    info!("Generating {:?} test signal: {} samples", config.signal, samples_count);
    
    match config.signal {
        SignalType::Sine => {
            let omega = 2.0 * std::f64::consts::PI * config.frequency / config.sample_rate;
            for i in 0..samples_count {
                let phase = omega * i as f64;
                samples.push(Complex::new(phase.cos(), phase.sin()));
            }
        },
        SignalType::Noise => {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            for _ in 0..samples_count {
                let real: f64 = rng.gen_range(-1.0..1.0);
                let imag: f64 = rng.gen_range(-1.0..1.0);
                samples.push(Complex::new(real, imag));
            }
        },
        SignalType::Sweep => {
            let start_freq = 500.0;
            let end_freq = 2500.0;
            let freq_step = (end_freq - start_freq) / samples_count as f64;
            let mut phase = 0.0;
            
            for i in 0..samples_count {
                let freq = start_freq + freq_step * i as f64;
                let omega = 2.0 * std::f64::consts::PI * freq / config.sample_rate;
                phase += omega;
                samples.push(Complex::new(phase.cos(), phase.sin()));
            }
        },
        SignalType::Morse => {
            let cw_config = CwConfig::new(20, config.frequency, config.sample_rate);
            let cw_gen = CwGenerator::new(cw_config);
            let morse_text = "CQ CQ DE TEST";
            let audio_samples = cw_gen.generate_cw_audio(morse_text);
            
            samples = audio_samples.into_iter()
                .map(|s| Complex::new(s as f64, 0.0))
                .collect();
        },
    }
    
    Ok(samples)
}

/// Write samples to WAV file
fn write_wav_file(samples: &[Complex], path: &PathBuf, sample_rate: f64) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("Failed to create WAV file: {:?}", path))?;
    
    for sample in samples {
        let amplitude = (sample.real * 32767.0).clamp(-32767.0, 32767.0) as i16;
        writer.write_sample(amplitude)?;
    }
    
    writer.finalize()?;
    info!("Wrote {} samples to {:?}", samples.len(), path);
    Ok(())
}

/// Read samples from WAV file
fn read_wav_file(path: &PathBuf) -> Result<Vec<Complex>> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("Failed to open WAV file: {:?}", path))?;
    
    let samples: Result<Vec<_>, _> = reader.samples::<i16>().collect();
    let samples = samples.with_context(|| "Failed to read audio samples")?;
    
    info!("Read {} samples from {:?}", samples.len(), path);
    Ok(samples.into_iter()
        .map(|s| Complex::new(s as f64 / 32767.0, 0.0))
        .collect())
}

/// Show system capabilities
fn show_info() {
    println!("\n=== OpenHam Digital Modes Tool ===");
    println!("Version: 0.1.0");
    
    println!("\n=== Supported Modulation Schemes ===");
    println!("  • BPSK  - Binary Phase Shift Keying");
    println!("  • FSK   - Frequency Shift Keying");
    println!("  • OFDM  - Orthogonal Frequency Division Multiplexing");
    println!("  • AFSK  - Audio FSK (Bell 202/103)");
    println!("  • PSK   - QPSK/8PSK/16PSK (with --psk-type)");
    println!("  • QAM   - 16/64/256/1024-QAM (with --qam-type)");
    println!("  • Experimental TX (FHSS/Chaos/Rotating/Waterfall)");
    
    println!("\n=== Features ===");
    println!("  • Huffman text compression");
    println!("  • CW preambles");
    println!("  • Pink noise squelch triggers");
    println!("  • Auto-detection mode");
    println!("  • WAV file input/output");
    
    println!("\n=== Example Usage ===");
    println!("  Transmit: openham tx -o output.wav -t \"Hello World\" -c S56SPZ --cw-preamble");
    println!("  PSK TX:  openham tx -m psk --psk-type qpsk -o out.wav -t TEXT -c CALL");
    println!("  QAM TX:  openham tx -m qam --qam-type 64 -o out.wav -t TEXT -c CALL");
    println!("  Receive:  openham rx -i input.wav --auto-detect");
    println!("  Generate: openham generate -o test.wav -s sine -f 1000");
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();
    
    info!("OpenHam digital modes tool starting");
    
    match cli.command {
        Commands::Tx(config) => {
            info!("Starting transmission with {:?} modulation", config.modulation);
            
            let mut transmitter = SimpleTransmitter::new(config.clone())?;
            let samples = transmitter.transmit()?;
            
            write_wav_file(&samples, &config.output, config.sample_rate)?;
            
            println!("✓ Transmission complete: {} samples written to {:?}", 
                     samples.len(), config.output);
        },
        
        Commands::Rx(config) => {
            info!("Starting reception from {:?}", config.input);
            
            let samples = read_wav_file(&config.input)?;
            let mut receiver = SimpleReceiver::new(config.clone())?;
            let messages = receiver.receive(&samples)?;
            
            if messages.is_empty() {
                println!("No messages decoded");
            } else {
                println!("✓ Decoded {} message(s):", messages.len());
                for (i, message) in messages.iter().enumerate() {
                    println!("  {}: {}", i + 1, message);
                }
                if let Some(ref output) = config.output {
                    // If only one message and it has prefix like "[BPSK] ", strip it for exact match use-cases
                    let out_text = if messages.len() == 1 {
                        let m = &messages[0];
                        if let Some(pos) = m.find("] ") { m[(pos+2)..].to_string() } else { m.clone() }
                    } else {
                        messages.join("\n")
                    };
                    std::fs::write(output, out_text)?;
                    println!("✓ Decoded text written to {:?}", output);
                }
            }
        },
        
        Commands::Generate(config) => {
            info!("Generating {:?} test signal", config.signal);
            
            let samples = generate_test_signal(&config)?;
            write_wav_file(&samples, &config.output, config.sample_rate)?;
            
            println!("✓ Test signal generated: {} samples written to {:?}",
                     samples.len(), config.output);
        },
        
        Commands::Info => {
            show_info();
        },
    }
    
    Ok(())
}
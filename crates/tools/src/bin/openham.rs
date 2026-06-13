//! OpenHam - Unified transmission and reception tool for all digital modes
//! 
//! This is the main entry point that provides a unified interface for:
//! - Standard modulation schemes (AFSK, BPSK, FSK, PSK, QAM, OFDM)
//! - Experimental encoders (chaos, frequency hopping, constellation rotation, waterfall)
//! - Multimedia framing with compression and fragmentation
//! - CW preambles and voice announcements with pink noise triggers
//! - Both transmission and reception capabilities with auto-detection

use clap::{Parser, Subcommand, ValueEnum};
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

use openham_core::buffer::Complex;
use openham_modem::prelude::*;
use openham_frame::prelude::*;
use openham_codecs::prelude::*;

/// OpenHam unified digital modes tool
#[derive(Parser)]
#[command(name = "openham")]
#[command(about = "OpenHam unified digital modes transmission and reception tool")]
#[command(version = "0.1.0")]
#[command(long_about = "
OpenHam is a comprehensive amateur radio digital modes tool supporting:
- Multiple modulation schemes (BPSK, FSK, AFSK, PSK, QAM, OFDM)
- Experimental encoders for research and development
- Multimedia frame transmission with compression
- CW preambles and voice announcements
- Real-time listening and auto-detection modes
")]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
    
    /// Enable debug output
    #[arg(long, global = true)]
    debug: bool,
    
    /// Configuration file
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Transmit data using various modulation schemes
    Tx(TransmitConfig),
    /// Receive and decode transmissions
    Rx(ReceiveConfig),
    /// Continuous listening mode with auto-detection
    Listen(ListenConfig),
    /// Analyze signal files
    Analyze(AnalyzeConfig),
    /// Generate test signals and patterns
    Generate(GenerateConfig),
    /// Show configuration and capabilities
    Info(InfoConfig),
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
    
    /// Text encoding scheme
    #[arg(long, default_value = "huffman")]
    pub encoding: EncodingType,
    
    /// Frame type
    #[arg(long, default_value = "standard")]
    pub frame_type: FrameType,

    /// Add AWGN to the output at this target SNR in dB (for generating noisy
    /// test signals). Omit for a clean signal.
    #[arg(long)]
    pub snr_db: Option<f64>,

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
    
    /// CW preamble text (default: "DE {callsign}")
    #[arg(long)]
    pub cw_text: Option<String>,
    
    /// CW speed in WPM
    #[arg(long, default_value = "20")]
    pub cw_wpm: u32,
    
    /// CW tone frequency in Hz
    #[arg(long, default_value = "600")]
    pub cw_freq: f64,
    
    /// Enable voice announcement
    #[arg(long)]
    pub voice_announce: bool,
    
    /// Voice announcement audio file
    #[arg(long)]
    pub voice_file: Option<PathBuf>,
    
    /// Enable pink noise trigger (for squelch)
    #[arg(long)]
    pub pink_noise: bool,
    
    /// Pink noise duration in seconds
    #[arg(long, default_value = "0.5")]
    pub pink_noise_duration: f64,
    
    /// Enable compression for multimedia frames
    #[arg(long)]
    pub compress: bool,
    
    /// Fragment large messages
    #[arg(long)]
    pub fragment: bool,
    
    /// Maximum fragment size in bytes
    #[arg(long, default_value = "1024")]
    pub fragment_size: usize,
    
    /// Transmission power level (0.0-1.0)
    #[arg(long, default_value = "0.8")]
    pub power: f64,
    
    /// Add silence padding in seconds
    #[arg(long, default_value = "1.0")]
    pub padding: f64,
}

/// Reception configuration
#[derive(Parser, Clone, Serialize, Deserialize)]
pub struct ReceiveConfig {
    /// Input audio file
    #[arg(short, long)]
    pub input: PathBuf,
    
    /// Output text file (optional)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// Modulation scheme to decode (or "auto" for detection)
    #[arg(short, long, default_value = "auto")]
    pub modulation: String,

    /// Text encoding to decode with (must match the transmitter)
    #[arg(long, default_value = "huffman")]
    pub encoding: EncodingType,

    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Center frequency in Hz (or "auto" for scanning)
    #[arg(long, default_value = "1500")]
    pub center_freq: f64,
    
    /// Symbol rate in Hz (or "auto" for detection)
    #[arg(long, default_value = "125")]
    pub symbol_rate: f64,
    
    /// Auto-detect frame format
    #[arg(long)]
    pub auto_detect: bool,
    
    /// Sensitivity threshold (0.0-1.0)
    #[arg(long, default_value = "0.3")]
    pub threshold: f64,
    
    /// Enable all decoders
    #[arg(long)]
    pub all_modes: bool,
}

/// Listening mode configuration
#[derive(Parser, Clone)]
pub struct ListenConfig {
    /// Audio input device or file
    #[arg(short, long, default_value = "default")]
    pub input: String,
    
    /// Output directory for decoded messages
    #[arg(short, long)]
    pub output_dir: Option<PathBuf>,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Center frequency in Hz
    #[arg(long, default_value = "1500")]
    pub center_freq: f64,
    
    /// Auto-detect all supported modes
    #[arg(long)]
    pub auto_detect: bool,
    
    /// Squelch threshold (0.0-1.0)
    #[arg(long, default_value = "0.1")]
    pub squelch: f64,
    
    /// Scan frequency range
    #[arg(long)]
    pub frequency_scan: bool,
    
    /// Frequency scan range in Hz
    #[arg(long, default_value = "3000")]
    pub scan_range: f64,
}

/// Analysis configuration
#[derive(Parser, Clone)]
pub struct AnalyzeConfig {
    /// Input audio file
    #[arg(short, long)]
    pub input: PathBuf,
    
    /// Output analysis file
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// Analysis type
    #[arg(short, long, default_value = "spectrum")]
    pub analysis: AnalysisType,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// FFT size for spectrum analysis
    #[arg(long, default_value = "1024")]
    pub fft_size: usize,
    
    /// Generate plots
    #[arg(long)]
    pub plot: bool,
}

/// Signal generation configuration
#[derive(Parser, Clone)]
pub struct GenerateConfig {
    /// Output audio file
    #[arg(short, long)]
    pub output: PathBuf,
    
    /// Signal type to generate
    #[arg(short, long, default_value = "test-pattern")]
    pub signal: SignalType,
    
    /// Duration in seconds
    #[arg(short, long, default_value = "10.0")]
    pub duration: f64,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Frequency in Hz
    #[arg(long, default_value = "1000")]
    pub frequency: f64,
    
    /// Amplitude (0.0-1.0)
    #[arg(long, default_value = "0.5")]
    pub amplitude: f64,
}

/// Info configuration
#[derive(Parser, Clone)]
pub struct InfoConfig {
    /// Show modulation schemes
    #[arg(long)]
    pub modulations: bool,
    
    /// Show frame types
    #[arg(long)]
    pub frames: bool,
    
    /// Show encodings
    #[arg(long)]
    pub encodings: bool,
    
    /// Show all capabilities
    #[arg(long)]
    pub all: bool,
}

/// Supported modulation types
#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum ModulationType {
    Afsk,
    Bpsk,
    Fsk,
    Psk4,
    Psk8,
    Qam16,
    Qam64,
    Ofdm64,
    Ofdm128,
    // Experimental
    Chaos,
    FreqHop,
    Rotating,
    Waterfall,
}

/// Supported encoding types
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodingType {
    Raw,
    Huffman,
    Ascii,
    Utf8,
}

/// Supported frame types
#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
pub enum FrameType {
    Standard,
    Multimedia,
    Beacon,
    Emergency,
}

/// Analysis types
#[derive(ValueEnum, Clone, Debug)]
pub enum AnalysisType {
    Spectrum,
    Waterfall,
    Constellation,
    EyeDiagram,
    All,
}

/// Signal generation types
#[derive(ValueEnum, Clone, Debug)]
pub enum SignalType {
    Sine,
    Noise,
    TestPattern,
    Sweep,
    Morse,
    TwoTone,
    Chirp,
}

/// Main transmission coordinator with full feature support
pub struct TransmissionCoordinator {
    config: TransmitConfig,
    modulator: Box<dyn Modulator>,
    announcement_samples: Vec<Complex>,
}

impl TransmissionCoordinator {
    /// Create new transmission coordinator with all features
    pub fn new(config: TransmitConfig) -> Result<Self> {
        info!("Creating transmission coordinator with {:?} modulation", config.modulation);
        
        // Create modulation configuration
        let mod_config = ModulationConfig::new(
            config.sample_rate,
            config.symbol_rate,
            config.center_freq,
        )?;
        
        // Create modulator based on type
        let modulator: Box<dyn Modulator> = match config.modulation {
            ModulationType::Bpsk => Box::new(BpskModulator::new(mod_config)?),
            ModulationType::Fsk => Box::new(FskModulator::new(mod_config)?),
            ModulationType::Afsk => {
                // Use default AFSK configuration for Bell 202
                let afsk_config = AfskConfig::bell_202();
                Box::new(AfskModulator::new(mod_config, afsk_config)?)
            },
            ModulationType::Psk4 => {
                let psk_config = PskConfig::qpsk();
                Box::new(PskModulator::new(mod_config, psk_config)?)
            },
            ModulationType::Psk8 => {
                let psk_config = PskConfig::psk8();
                Box::new(PskModulator::new(mod_config, psk_config)?)
            },
            ModulationType::Qam16 => {
                let qam_config = QamConfig::qam16();
                Box::new(QamModulator::new(mod_config, qam_config)?)
            },
            ModulationType::Qam64 => {
                let qam_config = QamConfig::qam64();
                Box::new(QamModulator::new(mod_config, qam_config)?)
            },
            ModulationType::Ofdm64 => {
                let ofdm_config = OfdmConfig::amateur_radio_64();
                Box::new(OfdmModulator::new(mod_config, ofdm_config)?)
            },
            ModulationType::Ofdm128 => {
                // Use the 64-carrier config for now until we implement 128
                let ofdm_config = OfdmConfig::amateur_radio_64();
                Box::new(OfdmModulator::new(mod_config, ofdm_config)?)
            },
            ModulationType::Chaos => {
                let chaos_config = ChaosConfig::default();
                Box::new(ChaosModulator::new(mod_config, chaos_config))
            },
            ModulationType::FreqHop => {
                let hop_frequencies = vec![1000.0, 1200.0, 1400.0, 1600.0, 1800.0, 2000.0];
                Box::new(FrequencyHoppingModulator::new(mod_config, hop_frequencies, 0.1))
            },
            ModulationType::Rotating => {
                Box::new(RotatingConstellationModulator::new(mod_config, 15.0))
            },
            ModulationType::Waterfall => {
                Box::new(WaterfallModulator::new(mod_config, 500.0, 2.0))
            },
        };
        
        Ok(Self {
            config,
            modulator,
            announcement_samples: Vec::new(),
        })
    }
    
    /// Generate complete transmission with all announcements
    pub fn generate_transmission(&mut self) -> Result<Vec<Complex>> {
        let start_time = Instant::now();
        let mut all_samples = Vec::new();
        
        // 1. Generate announcements first
        self.generate_announcements()?;
        all_samples.extend_from_slice(&self.announcement_samples);
        
        // 2. Get and encode input data
        let text = self.get_input_text()?;
        info!("Transmitting: {} characters", text.len());
        
        let encoded_data = self.encode_text(&text)?;
        debug!("Encoded to {} bytes using {:?}", encoded_data.len(), self.config.encoding);
        
        // 3. Build frames with optional fragmentation
        let frames = self.build_frames(&encoded_data)?;
        info!("Created {} frame(s)", frames.len());
        
        // 4. Modulate each frame, wrapped with a preamble + sync word so a live
        //    receiver can acquire it at an arbitrary point in the audio stream.
        for (i, frame) in frames.iter().enumerate() {
            let frame_bytes = add_preamble_sync(&frame.to_bytes());
            let mut frame_samples = Vec::new();
            
            // Add inter-frame spacing for multiple frames
            if i > 0 {
                let spacing_samples = (self.config.sample_rate * 0.5) as usize;
                frame_samples.resize(spacing_samples, Complex::new(0.0, 0.0));
            }
            
            self.modulator.modulate(&frame_bytes, &mut frame_samples)?;
            let frame_sample_count = frame_samples.len();
            all_samples.extend(frame_samples);
            debug!("Modulated frame {} with {} samples", i, frame_sample_count);
        }
        
        // 5. Add padding silence
        let padding_samples = (self.config.sample_rate * self.config.padding) as usize;
        all_samples.resize(all_samples.len() + padding_samples, Complex::new(0.0, 0.0));
        
        // 6. Apply power scaling
        if self.config.power != 1.0 {
            for sample in &mut all_samples {
                *sample = Complex::new(sample.real * self.config.power, sample.imag * self.config.power);
            }
        }
        
        let elapsed = start_time.elapsed();
        info!("Complete transmission generated in {:?}: {} samples total", 
              elapsed, all_samples.len());
        
        Ok(all_samples)
    }
    
    /// Generate announcements (pink noise, CW, voice)
    fn generate_announcements(&mut self) -> Result<()> {
        self.announcement_samples.clear();
        
        // Pink noise burst for squelch triggering
        if self.config.pink_noise {
            let pink_samples = self.generate_pink_noise()?;
            self.announcement_samples.extend(pink_samples);
            info!("Added pink noise burst: {} samples", self.announcement_samples.len());
        }
        
        // CW preamble
        if self.config.cw_preamble {
            let cw_samples = self.generate_cw_preamble()?;
            self.announcement_samples.extend(cw_samples);
            info!("Added CW preamble");
        }
        
        // Voice announcement
        if self.config.voice_announce {
            if let Some(voice_samples) = self.generate_voice_announcement()? {
                self.announcement_samples.extend(voice_samples);
                info!("Added voice announcement");
            }
        }
        
        Ok(())
    }
    
    /// Generate pink noise burst
    fn generate_pink_noise(&self) -> Result<Vec<Complex>> {
        let samples_count = (self.config.sample_rate * self.config.pink_noise_duration) as usize;
        let mut samples = Vec::with_capacity(samples_count);
        
        // Simple pink noise approximation using multiple octaves
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        for _ in 0..samples_count {
            let mut pink = 0.0;
            
            // Add multiple octaves with decreasing amplitude
            for octave in 0..6 {
                let white: f64 = rng.gen_range(-1.0..1.0);
                let amplitude = 1.0 / (2.0_f64.powi(octave));
                pink += white * amplitude;
            }
            
            pink *= 0.1; // Scale down
            samples.push(Complex::new(pink, 0.0));
        }
        
        Ok(samples)
    }
    
    /// Generate CW preamble
    fn generate_cw_preamble(&self) -> Result<Vec<Complex>> {
        let cw_config = CwConfig::new(self.config.cw_wpm, self.config.cw_freq, self.config.sample_rate);
        let cw_gen = CwGenerator::new(cw_config);
        
        let preamble_text = self.config.cw_text.clone()
            .unwrap_or_else(|| format!("DE {} ", self.config.callsign));
        
        let audio_samples = cw_gen.generate_cw_audio(&preamble_text);
        Ok(audio_samples.into_iter()
            .map(|s| Complex::new(s as f64, 0.0))
            .collect())
    }
    
    /// Generate voice announcement
    fn generate_voice_announcement(&self) -> Result<Option<Vec<Complex>>> {
        if let Some(ref voice_file) = self.config.voice_file {
            match self.read_audio_file(voice_file) {
                Ok(samples) => Ok(Some(samples)),
                Err(e) => {
                    warn!("Could not load voice file {:?}: {}", voice_file, e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }
    
    /// Read audio file
    fn read_audio_file(&self, path: &PathBuf) -> Result<Vec<Complex>> {
        let mut reader = hound::WavReader::open(path)
            .with_context(|| format!("Failed to open audio file: {:?}", path))?;
        
        let samples: Result<Vec<_>, _> = reader.samples::<i16>().collect();
        let samples = samples.with_context(|| "Failed to read audio samples")?;
        
        Ok(samples.into_iter()
            .map(|s| Complex::new(s as f64 / 32767.0, 0.0))
            .collect())
    }
    
    /// Get input text from configuration
    fn get_input_text(&self) -> Result<String> {
        if let Some(ref text) = self.config.text {
            Ok(text.clone())
        } else if let Some(ref file) = self.config.file {
            std::fs::read_to_string(file)
                .with_context(|| format!("Failed to read input file: {:?}", file))
        } else {
            Ok(format!("Hello from {} using OpenHam", self.config.callsign))
        }
    }
    
    /// Encode text using specified encoding
    fn encode_text(&self, text: &str) -> Result<Vec<u8>> {
        match self.config.encoding {
            EncodingType::Raw => Ok(text.as_bytes().to_vec()),
            EncodingType::Huffman => {
                let mut codec = HuffmanCodec::new_english();
                codec.encode(text).map_err(|e| anyhow::anyhow!("Huffman encoding failed: {}", e))
            },
            EncodingType::Ascii => {
                if text.is_ascii() {
                    Ok(text.as_bytes().to_vec())
                } else {
                    anyhow::bail!("Text contains non-ASCII characters")
                }
            },
            EncodingType::Utf8 => Ok(text.as_bytes().to_vec()),
        }
    }
    
    /// Build frames from encoded data with optional fragmentation
    fn build_frames(&self, data: &[u8]) -> Result<Vec<Frame>> {
        if self.config.fragment && data.len() > self.config.fragment_size {
            let mut frames = Vec::new();
            let mut sequence = 0u16;
            
            for chunk in data.chunks(self.config.fragment_size) {
                let mut flags = 0u8;
                if sequence == 0 {
                    flags |= 0x01; // First fragment
                }
                
                let frame = match self.config.frame_type {
                    FrameType::Standard => Frame::new(1, sequence, chunk.to_vec(), flags),
                    FrameType::Multimedia => {
                        let multimedia_frame = MultimediaFrame::create_binary_frame(
                            "payload.bin".to_string(),
                            chunk,
                            self.config.callsign.clone(),
                            self.config.compress,
                        ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

                        let serialized = multimedia_frame.to_bytes()
                            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                        Frame::new(2, sequence, serialized, flags)
                    },
                    FrameType::Beacon => Frame::new(3, sequence, chunk.to_vec(), flags | 0x80),
                    FrameType::Emergency => Frame::new(4, sequence, chunk.to_vec(), flags | 0x40),
                };
                
                frames.push(frame);
                sequence += 1;
            }
            
            // Mark last frame
            if let Some(last_frame) = frames.last_mut() {
                let mut new_flags = last_frame.header.flags | 0x02; // Last fragment
                let new_frame = Frame::new(
                    last_frame.header.frame_type,
                    last_frame.header.sequence,
                    last_frame.payload.clone(),
                    new_flags
                );
                *last_frame = new_frame;
            }
            
            Ok(frames)
        } else {
            // Single frame
            let frame = match self.config.frame_type {
                FrameType::Standard => Frame::new(1, 0, data.to_vec(), 0),
                FrameType::Multimedia => {
                    let multimedia_frame = MultimediaFrame::create_binary_frame(
                        "payload.bin".to_string(),
                        data,
                        self.config.callsign.clone(),
                        self.config.compress,
                    ).map_err(|e| anyhow::anyhow!(e.to_string()))?;

                    let serialized = multimedia_frame.to_bytes()
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    Frame::new(2, 0, serialized, 0)
                },
                FrameType::Beacon => Frame::new(3, 0, data.to_vec(), 0x80),
                FrameType::Emergency => Frame::new(4, 0, data.to_vec(), 0x40),
            };
            
            Ok(vec![frame])
        }
    }
}

/// Reception coordinator with auto-detection and multiple demodulators
pub struct ReceptionCoordinator {
    config: ReceiveConfig,
    demodulators: Vec<(String, Box<dyn Demodulator>)>,
}

impl ReceptionCoordinator {
    /// Create new reception coordinator
    pub fn new(config: ReceiveConfig) -> Result<Self> {
        info!("Creating reception coordinator");
        
        let mod_config = ModulationConfig::new(
            config.sample_rate,
            config.symbol_rate,
            config.center_freq,
        )?;
        
        let mut demodulators: Vec<(String, Box<dyn Demodulator>)> = Vec::new();
        
        if config.modulation == "auto" || config.all_modes {
            // Add all supported demodulators for auto-detection
            demodulators.push(("BPSK".to_string(), Box::new(BpskDemodulator::new(mod_config.clone())?)));
            demodulators.push(("FSK".to_string(), Box::new(FskDemodulator::new(mod_config.clone())?)));
            
            // Add PSK variants
            let psk4_config = PskConfig::qpsk();
            demodulators.push(("PSK4".to_string(), Box::new(PskDemodulator::new(mod_config.clone(), psk4_config)?)));
            
            // Add AFSK (Bell-202: 1200 baud, must match the transmitter)
            let afsk_config = AfskConfig::bell_202();
            demodulators.push(("AFSK".to_string(), Box::new(AfskDemodulator::new(mod_config.clone(), afsk_config)?)));
            
            // Add OFDM
            let ofdm_config = OfdmConfig::amateur_radio_64();
            demodulators.push(("OFDM64".to_string(), Box::new(OfdmDemodulator::new(mod_config.clone(), ofdm_config)?)));
            
            info!("Auto-detection mode: {} demodulators active", demodulators.len());
        } else {
            // Single demodulator based on specified type
            let demodulator: Box<dyn Demodulator> = match config.modulation.as_str() {
                "bpsk" => Box::new(BpskDemodulator::new(mod_config)?),
                "fsk" => Box::new(FskDemodulator::new(mod_config)?),
                "afsk" => {
                    // Bell-202: 1200 baud, must match the transmitter.
                    let afsk_config = AfskConfig::bell_202();
                    Box::new(AfskDemodulator::new(mod_config, afsk_config)?)
                },
                "psk4" => {
                    let psk_config = PskConfig::qpsk();
                    Box::new(PskDemodulator::new(mod_config, psk_config)?)
                },
                "ofdm64" => {
                    let ofdm_config = OfdmConfig::amateur_radio_64();
                    Box::new(OfdmDemodulator::new(mod_config, ofdm_config)?)
                },
                _ => anyhow::bail!("Unsupported modulation: {}", config.modulation),
            };
            demodulators.push((config.modulation.clone(), demodulator));
        }
        
        Ok(Self {
            config,
            demodulators,
        })
    }
    
    /// Receive and decode from samples
    pub fn receive(&mut self, samples: &[Complex]) -> Result<Vec<DecodedMessage>> {
        let mut decoded_messages = Vec::new();
        
        info!("Processing {} samples with {} demodulators", samples.len(), self.demodulators.len());

        let encoding = self.config.encoding;

        // Try each demodulator
        for (name, demodulator) in &mut self.demodulators {
            let mut demod_bytes = Vec::new();

            match demodulator.demodulate(samples, &mut demod_bytes) {
                Ok(()) if !demod_bytes.is_empty() => {
                    debug!("{} demodulator produced {} bytes", name, demod_bytes.len());

                    // Acquire frames from the demodulated bit stream: correlate
                    // the sync word at every bit offset, so a transmission that
                    // started anywhere in the capture is located. (Re-expanding
                    // the bytes to bits preserves the recovered bit sequence.)
                    let bit_stream = bytes_to_bits(&demod_bytes);
                    let frames = Acquisition::new().find_frames(&bit_stream);
                    debug!("{} acquired {} frame(s)", name, frames.len());

                    for frame in frames {
                        let text = Self::decode_payload(
                            &frame.payload,
                            frame.header.frame_type,
                            encoding,
                        )?;
                        let quality = demodulator.signal_quality();

                        decoded_messages.push(DecodedMessage {
                            modulation: name.clone(),
                            text,
                            frame_type: frame.header.frame_type,
                            sequence: frame.header.sequence,
                            signal_quality: quality,
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                },
                Ok(()) => {
                    debug!("{} demodulator produced no bytes", name);
                },
                Err(e) => {
                    debug!("{} demodulator failed: {}", name, e);
                }
            }
        }
        
        Ok(decoded_messages)
    }
    
    /// Decode frame payload based on frame type and the configured text encoding.
    fn decode_payload(data: &[u8], frame_type: u8, encoding: EncodingType) -> Result<String> {
        match frame_type {
            2 => {
                // Multimedia frame
                match MultimediaFrame::from_bytes(data) {
                    Ok(multimedia_frame) => {
                        let payload = multimedia_frame
                            .decompress_payload()
                            .unwrap_or_else(|_| multimedia_frame.payload.clone());
                        if let Ok(text) = String::from_utf8(payload) {
                            return Ok(format!("[MULTIMEDIA] {}", text));
                        }
                    },
                    Err(e) => debug!("Multimedia frame decode failed: {}", e),
                }
            },
            3 => {
                // Beacon frame
                if let Ok(text) = String::from_utf8(data.to_vec()) {
                    return Ok(format!("[BEACON] {}", text));
                }
            },
            4 => {
                // Emergency frame
                if let Ok(text) = String::from_utf8(data.to_vec()) {
                    return Ok(format!("[EMERGENCY] {}", text));
                }
            },
            _ => {
                // Standard frame - try different decodings
            }
        }
        
        // Standard frame: decode with the configured text encoding (it must
        // match the transmitter). Only Huffman is compressed; the others are
        // plain UTF-8 text.
        match encoding {
            EncodingType::Huffman => {
                let mut huffman = HuffmanCodec::new_english();
                if let Ok(text) = huffman.decode(data) {
                    return Ok(text);
                }
            }
            EncodingType::Raw | EncodingType::Ascii | EncodingType::Utf8 => {
                if let Ok(text) = String::from_utf8(data.to_vec()) {
                    return Ok(text);
                }
            }
        }

        // Fallback to hex representation
        Ok(format!("HEX:{}", data.iter().map(|b| format!("{:02x}", b)).collect::<String>()))
    }
}

/// Decoded message with metadata
#[derive(Debug, Clone)]
pub struct DecodedMessage {
    pub modulation: String,
    pub text: String,
    pub frame_type: u8,
    pub sequence: u16,
    pub signal_quality: SignalQuality,
    pub timestamp: std::time::SystemTime,
}

/// Write audio samples to WAV file
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

/// Read audio samples from WAV file
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
                let amplitude = config.amplitude;
                samples.push(Complex::new(amplitude * phase.cos(), amplitude * phase.sin()));
            }
        },
        SignalType::Noise => {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            for _ in 0..samples_count {
                let real: f64 = rng.gen_range(-config.amplitude..config.amplitude);
                let imag: f64 = rng.gen_range(-config.amplitude..config.amplitude);
                samples.push(Complex::new(real, imag));
            }
        },
        SignalType::TwoTone => {
            let omega1 = 2.0 * std::f64::consts::PI * config.frequency / config.sample_rate;
            let omega2 = 2.0 * std::f64::consts::PI * (config.frequency + 500.0) / config.sample_rate;
            for i in 0..samples_count {
                let phase1 = omega1 * i as f64;
                let phase2 = omega2 * i as f64;
                let amplitude = config.amplitude * 0.5;
                let real = amplitude * (phase1.cos() + phase2.cos());
                samples.push(Complex::new(real, 0.0));
            }
        },
        SignalType::Sweep => {
            let start_freq = config.frequency;
            let end_freq = config.frequency + 1000.0;
            let freq_step = (end_freq - start_freq) / samples_count as f64;
            let mut phase = 0.0;
            
            for i in 0..samples_count {
                let freq = start_freq + freq_step * i as f64;
                let omega = 2.0 * std::f64::consts::PI * freq / config.sample_rate;
                phase += omega;
                let amplitude = config.amplitude;
                samples.push(Complex::new(amplitude * phase.cos(), amplitude * phase.sin()));
            }
        },
        SignalType::Chirp => {
            let start_freq = config.frequency;
            let end_freq = config.frequency + 2000.0;
            let beta = (end_freq - start_freq) / config.duration;
            
            for i in 0..samples_count {
                let t = i as f64 / config.sample_rate;
                let freq = start_freq + beta * t;
                let phase = 2.0 * std::f64::consts::PI * freq * t;
                let amplitude = config.amplitude;
                samples.push(Complex::new(amplitude * phase.cos(), amplitude * phase.sin()));
            }
        },
        SignalType::TestPattern => {
            // Generate alternating high/low pattern
            let pattern_freq = config.frequency;
            let omega = 2.0 * std::f64::consts::PI * pattern_freq / config.sample_rate;
            for i in 0..samples_count {
                let phase = omega * i as f64;
                let bit = if phase.sin() > 0.0 { 1.0 } else { -1.0 };
                let amplitude = config.amplitude;
                samples.push(Complex::new(amplitude * bit, 0.0));
            }
        },
        SignalType::Morse => {
            // Generate "CQ CQ DE TEST" in Morse
            let cw_config = CwConfig::new(20, config.frequency, config.sample_rate);
            let cw_gen = CwGenerator::new(cw_config);
            let morse_text = "CQ CQ DE TEST";
            let audio_samples = cw_gen.generate_cw_audio(morse_text);
            
            samples = audio_samples.into_iter()
                .map(|s| Complex::new(s as f64 * config.amplitude, 0.0))
                .collect();
        },
    }
    
    Ok(samples)
}

/// Show system information and capabilities
fn show_info(config: &InfoConfig) {
    if config.all || config.modulations {
        println!("\n=== Supported Modulation Schemes ===");
        println!("Standard:");
        println!("  • AFSK     - Audio Frequency Shift Keying");
        println!("  • BPSK     - Binary Phase Shift Keying");
        println!("  • FSK      - Frequency Shift Keying");
        println!("  • PSK4     - 4-Phase Shift Keying");
        println!("  • PSK8     - 8-Phase Shift Keying");
        println!("  • QAM16    - 16-Quadrature Amplitude Modulation");
        println!("  • QAM64    - 64-Quadrature Amplitude Modulation");
        println!("  • OFDM64   - 64-subcarrier OFDM");
        println!("  • OFDM128  - 128-subcarrier OFDM");
        println!("\nExperimental:");
        println!("  • Chaos    - Chaos-based spread spectrum");
        println!("  • FreqHop  - Frequency hopping");
        println!("  • Rotating - Rotating constellation");
        println!("  • Waterfall- Waterfall encoding");
    }
    
    if config.all || config.frames {
        println!("\n=== Frame Types ===");
        println!("  • Standard   - Basic frame format");
        println!("  • Multimedia - Compressed multimedia frames");
        println!("  • Beacon     - Beacon/identification frames");
        println!("  • Emergency  - Emergency priority frames");
    }
    
    if config.all || config.encodings {
        println!("\n=== Text Encodings ===");
        println!("  • Raw      - Raw binary data");
        println!("  • Huffman  - Huffman compression");
        println!("  • ASCII    - ASCII text encoding");
        println!("  • UTF8     - UTF-8 text encoding");
    }
    
    if config.all {
        println!("\n=== Announcement Features ===");
        println!("  • Pink noise burst for squelch triggering");
        println!("  • CW preambles with configurable speed and frequency");
        println!("  • Voice announcements from audio files");
        println!("  • Frame fragmentation for large messages");
        println!("  • Auto-detection and multi-mode reception");
        
        println!("\n=== File Formats ===");
        println!("  • Input/Output: WAV files (16-bit, mono)");
        println!("  • Configuration: JSON/TOML files");
        println!("  • Text output: UTF-8 text files");
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging based on verbosity
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
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
    
    info!("OpenHam unified digital modes tool starting");
    
    // Load configuration file if specified
    if let Some(config_path) = &cli.config {
        if config_path.exists() {
            info!("Loading configuration from {:?}", config_path);
            // TODO: Implement configuration file loading
        } else {
            warn!("Configuration file {:?} not found", config_path);
        }
    }
    
    match cli.command {
        Commands::Tx(config) => {
            info!("Starting transmission with {:?} modulation", config.modulation);
            
            let mut coordinator = TransmissionCoordinator::new(config.clone())?;
            let mut samples = coordinator.generate_transmission()?;

            if let Some(snr_db) = config.snr_db {
                use openham_core::channel::add_awgn_real_snr;
                use rand::SeedableRng;
                let mut rng = rand::rngs::StdRng::seed_from_u64(0xC0FFEE);
                add_awgn_real_snr(&mut samples, snr_db, &mut rng);
                info!("Added AWGN at {snr_db} dB SNR");
            }

            write_wav_file(&samples, &config.output, config.sample_rate)?;

            println!("✓ Transmission complete: {} samples written to {:?}",
                     samples.len(), config.output);
        },
        
        Commands::Rx(config) => {
            info!("Starting reception from {:?}", config.input);
            
            let samples = read_wav_file(&config.input)?;
            let mut coordinator = ReceptionCoordinator::new(config.clone())?;
            let messages = coordinator.receive(&samples)?;
            
            if messages.is_empty() {
                println!("No messages decoded");
            } else {
                println!("✓ Decoded {} message(s):", messages.len());
                for (i, message) in messages.iter().enumerate() {
                    println!("\nMessage {} ({}): SNR: {:.1} dB, EVM: {:.1}%",
                             i + 1, message.modulation, 
                             message.signal_quality.snr_db,
                             message.signal_quality.evm_percent);
                    println!("  {}", message.text);
                }
                
                if let Some(ref output) = config.output {
                    let all_text = messages.iter()
                        .map(|m| format!("{}: {}", m.modulation, m.text))
                        .collect::<Vec<_>>()
                        .join("\n");
                    std::fs::write(output, all_text)?;
                    println!("✓ Decoded text written to {:?}", output);
                }
            }
        },
        
        Commands::Listen(_config) => {
            println!("⚠ Continuous listening mode not yet implemented");
            println!("This would enable real-time audio processing with auto-detection");
            // TODO: Implement real-time audio processing
        },
        
        Commands::Analyze(_config) => {
            println!("⚠ Signal analysis not yet implemented");
            println!("This would provide spectrum analysis, waterfall plots, etc.");
            // TODO: Implement signal analysis
        },
        
        Commands::Generate(config) => {
            info!("Generating {:?} test signal", config.signal);
            
            let samples = generate_test_signal(&config)?;
            write_wav_file(&samples, &config.output, config.sample_rate)?;
            
            println!("✓ Test signal generated: {} samples written to {:?}",
                     samples.len(), config.output);
        },
        
        Commands::Info(config) => {
            show_info(&config);
        },
    }
    
    Ok(())
}
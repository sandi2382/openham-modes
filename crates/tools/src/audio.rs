//! Real-time soundcard capture and playback via cpal.
//!
//! [`play_real_samples`] plays a buffer of real mono audio to the default
//! output device, and [`LiveCapture`] continuously reads the default input
//! device into a buffer. Both use the f32 sample format (universally supported)
//! at a requested sample rate; the modem runs at that same rate so transmit and
//! receive agree.

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, SupportedStreamConfig};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Print the available audio input/output devices.
pub fn list_devices() -> Result<()> {
    let host = cpal::default_host();
    let def_out = host.default_output_device().and_then(|d| d.name().ok());
    let def_in = host.default_input_device().and_then(|d| d.name().ok());

    println!("Output devices:");
    if let Ok(devs) = host.output_devices() {
        for d in devs {
            let n = d.name().unwrap_or_else(|_| "<unknown>".into());
            let mark = if Some(&n) == def_out.as_ref() { "  (default)" } else { "" };
            println!("  - {n}{mark}");
        }
    }
    println!("Input devices:");
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            let n = d.name().unwrap_or_else(|_| "<unknown>".into());
            let mark = if Some(&n) == def_in.as_ref() { "  (default)" } else { "" };
            println!("  - {n}{mark}");
        }
    }
    Ok(())
}

/// Resolve an output device by name (`None`/"default" => system default).
fn output_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device> {
    match name {
        None | Some("default") => host
            .default_output_device()
            .ok_or_else(|| anyhow!("no default output device")),
        Some(n) => host
            .output_devices()?
            .find(|d| d.name().map(|dn| dn == n).unwrap_or(false))
            .ok_or_else(|| anyhow!("output device '{n}' not found (see `openham listen --list-devices`)")),
    }
}

/// Resolve an input device by name (`None`/"default" => system default).
fn input_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device> {
    match name {
        None | Some("default") => host
            .default_input_device()
            .ok_or_else(|| anyhow!("no default input device")),
        Some(n) => host
            .input_devices()?
            .find(|d| d.name().map(|dn| dn == n).unwrap_or(false))
            .ok_or_else(|| anyhow!("input device '{n}' not found (see `openham listen --list-devices`)")),
    }
}

/// Find an f32 device configuration that supports `sample_rate`.
fn f32_config(
    device: &cpal::Device,
    sample_rate: u32,
    output: bool,
) -> Result<SupportedStreamConfig> {
    let ranges: Vec<_> = if output {
        device.supported_output_configs().context("query output configs")?.collect()
    } else {
        device.supported_input_configs().context("query input configs")?.collect()
    };
    for r in &ranges {
        if r.sample_format() == SampleFormat::F32
            && r.min_sample_rate().0 <= sample_rate
            && sample_rate <= r.max_sample_rate().0
        {
            return Ok(r.clone().with_sample_rate(SampleRate(sample_rate)));
        }
    }
    Err(anyhow!(
        "default device has no f32 configuration supporting {sample_rate} Hz; \
         pick a --sample-rate the device supports"
    ))
}

/// Play real mono samples (≈ [-1, 1]) to the default output device at
/// `sample_rate` Hz, blocking until playback finishes.
pub fn play_real_samples(samples: &[f32], sample_rate: u32, device: Option<&str>) -> Result<()> {
    let host = cpal::default_host();
    let device = output_device(&host, device)?;
    let supported = f32_config(&device, sample_rate, true)?;
    let channels = supported.channels() as usize;
    let config: cpal::StreamConfig = supported.config();

    let data: Arc<Vec<f32>> = Arc::new(samples.to_vec());
    let pos = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicBool::new(false));

    let (data_cb, pos_cb, done_cb) = (data.clone(), pos.clone(), done.clone());
    let stream = device
        .build_output_stream(
            &config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut p = pos_cb.load(Ordering::Relaxed);
                for frame in out.chunks_mut(channels) {
                    let v = data_cb.get(p).copied().unwrap_or(0.0);
                    for s in frame.iter_mut() {
                        *s = v;
                    }
                    if p < data_cb.len() {
                        p += 1;
                    }
                }
                pos_cb.store(p, Ordering::Relaxed);
                if p >= data_cb.len() {
                    done_cb.store(true, Ordering::Relaxed);
                }
            },
            |e| eprintln!("audio output error: {e}"),
            None,
        )
        .context("building output stream")?;
    stream.play().context("starting playback")?;

    while !done.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // Let the device drain its buffer before the stream is dropped.
    std::thread::sleep(std::time::Duration::from_millis(250));
    Ok(())
}

/// Continuous capture from the default input device. Captured samples accumulate
/// in an internal buffer; call [`LiveCapture::take`] to drain what has arrived.
pub struct LiveCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub device_name: String,
}

impl LiveCapture {
    pub fn start(sample_rate: u32, device: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();
        let device = input_device(&host, device)?;
        let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());
        let supported = f32_config(&device, sample_rate, false)?;
        let channels = supported.channels() as usize;
        let config: cpal::StreamConfig = supported.config();

        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let buf_cb = buffer.clone();
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut b) = buf_cb.lock() {
                        // Down-mix to mono: take the first channel of each frame.
                        for frame in data.chunks(channels) {
                            b.push(frame[0]);
                        }
                    }
                },
                |e| eprintln!("audio input error: {e}"),
                None,
            )
            .context("building input stream")?;
        stream.play().context("starting capture")?;

        Ok(Self {
            buffer,
            _stream: stream,
            sample_rate,
            device_name,
        })
    }

    /// Drain and return all samples captured since the last call.
    pub fn take(&self) -> Vec<f32> {
        self.buffer
            .lock()
            .map(|mut b| std::mem::take(&mut *b))
            .unwrap_or_default()
    }
}

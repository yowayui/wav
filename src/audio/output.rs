use crate::error::{Result, WavError};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use ringbuf::{HeapProducer, HeapRb};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc,
};

const RING_CAPACITY: usize = 32_768; // ~0.34s at 48kHz stereo f32

pub struct AudioOutput {
    _stream: Stream,
    pub producer: HeapProducer<f32>,
    pub sample_rate: u32,
    pub channels: usize,
    volume: Arc<AtomicU8>,
    pub paused: Arc<AtomicBool>,
    pub reset: Arc<AtomicBool>,
}

impl AudioOutput {
    pub fn open(device: &Device, volume_percent: u8) -> Result<Self> {
        let config = device
            .default_output_config()
            .map_err(|e| WavError::Audio(format!("output config: {e}")))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let rb = HeapRb::<f32>::new(RING_CAPACITY);
        let (producer, mut consumer) = rb.split();

        let volume = Arc::new(AtomicU8::new(volume_percent));
        let paused = Arc::new(AtomicBool::new(false));
        let reset = Arc::new(AtomicBool::new(false));

        let vol_clone = Arc::clone(&volume);
        let paused_clone = Arc::clone(&paused);
        let reset_clone = Arc::clone(&reset);

        let stream_config = StreamConfig {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        // CPAL callback: no I/O, no logging, no blocking
        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| {
                    if reset_clone.swap(false, Ordering::Relaxed) {
                        consumer.clear();
                    }
                    if paused_clone.load(Ordering::Relaxed) {
                        for s in data.iter_mut() {
                            *s = 0.0;
                        }
                        return;
                    }
                    let vol = vol_clone.load(Ordering::Relaxed) as f32 / 100.0;
                    let available = consumer.pop_slice(data);
                    for s in &mut data[..available] {
                        *s *= vol;
                    }
                    // Underrun: fill with silence
                    for s in &mut data[available..] {
                        *s = 0.0;
                    }
                },
                |_e| {}, // can't log in callback
                None,
            )
            .map_err(|e| WavError::Audio(format!("build stream: {e}")))?;

        stream
            .play()
            .map_err(|e| WavError::Audio(format!("play stream: {e}")))?;

        Ok(Self {
            _stream: stream,
            producer,
            sample_rate,
            channels,
            volume,
            paused,
            reset,
        })
    }

    pub fn set_volume(&self, percent: u8) {
        self.volume.store(percent.min(100), Ordering::Relaxed);
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn reset_buffer(&self) {
        self.reset.store(true, Ordering::Relaxed);
    }
}

/// List available output devices on the default host.
pub fn list_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let devices = host
        .output_devices()
        .map_err(|e| WavError::Audio(format!("enumerate devices: {e}")))?;
    Ok(devices.filter_map(|d| d.name().ok()).collect())
}

#[allow(dead_code)]
pub fn default_device() -> Option<Device> {
    cpal::default_host().default_output_device()
}

use crate::error::{Result, WavError};
use std::path::Path;
use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Signal},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    sample::{i24, u24, Sample},
    units::Time,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub codec_name: &'static str,
    pub duration_secs: Option<f64>,
}

pub struct Decoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    pub info: StreamInfo,
}

impl Decoder {
    pub fn open(path: &Path, gapless: bool) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions {
                    enable_gapless: gapless,
                    ..Default::default()
                },
                &MetadataOptions::default(),
            )
            .map_err(|e| WavError::Audio(format!("probe failed: {e}")))?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| WavError::Audio("no supported audio track".into()))?;

        let track_id = track.id;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| WavError::Audio("unknown sample rate".into()))?;
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        let codec_name = symphonia::default::get_codecs()
            .get_codec(track.codec_params.codec)
            .map(|d| d.short_name)
            .unwrap_or("unknown");

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| WavError::Audio(format!("decoder init failed: {e}")))?;

        let duration_secs = track
            .codec_params
            .n_frames
            .map(|f| f as f64 / sample_rate as f64);

        Ok(Self {
            format,
            decoder,
            track_id,
            info: StreamInfo {
                sample_rate,
                channels,
                codec_name,
                duration_secs,
            },
        })
    }

    /// Decode next packet → interleaved f32. Returns `None` at end-of-stream.
    pub fn next_packet_f32(&mut self) -> Result<Option<Vec<f32>>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(WavError::Audio(format!("demux: {e}"))),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => return Ok(Some(audio_buf_to_f32(decoded))),
                Err(SymphoniaError::IoError(_)) => return Ok(None),
                Err(SymphoniaError::DecodeError(e)) => {
                    tracing::warn!("decode error (skipping): {e}");
                    continue;
                }
                Err(e) => return Err(WavError::Audio(format!("decode: {e}"))),
            }
        }
    }

    pub fn seek(&mut self, seconds: f64) -> Result<()> {
        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::from(seconds),
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| WavError::Audio(format!("seek: {e}")))?;
        self.decoder.reset();
        Ok(())
    }
}

fn audio_buf_to_f32(buf: AudioBufferRef<'_>) -> Vec<f32> {
    match buf {
        AudioBufferRef::F32(b) => interleave(b.as_ref()),
        AudioBufferRef::F64(b) => {
            let b = b.as_ref();
            let nch = b.spec().channels.count();
            let mut out = Vec::with_capacity(b.frames() * nch);
            for f in 0..b.frames() {
                for ch in 0..nch {
                    out.push(b.chan(ch)[f] as f32);
                }
            }
            out
        }
        AudioBufferRef::S32(b) => conv(b.as_ref(), |s: i32| s as f32 / i32::MAX as f32),
        AudioBufferRef::S24(b) => conv(b.as_ref(), |s: i24| s.inner() as f32 / 8_388_607.0),
        AudioBufferRef::S16(b) => conv(b.as_ref(), |s: i16| s as f32 / i16::MAX as f32),
        AudioBufferRef::U32(b) => conv(b.as_ref(), |s: u32| s as f32 / u32::MAX as f32 * 2.0 - 1.0),
        AudioBufferRef::U24(b) => conv(b.as_ref(), |s: u24| {
            s.inner() as f32 / 16_777_215.0 * 2.0 - 1.0
        }),
        AudioBufferRef::U16(b) => conv(b.as_ref(), |s: u16| s as f32 / u16::MAX as f32 * 2.0 - 1.0),
        AudioBufferRef::U8(b) => conv(b.as_ref(), |s: u8| s as f32 / u8::MAX as f32 * 2.0 - 1.0),
        // S8 is rare but must be covered
        AudioBufferRef::S8(b) => conv(b.as_ref(), |s: i8| s as f32 / i8::MAX as f32),
    }
}

fn interleave(buf: &AudioBuffer<f32>) -> Vec<f32> {
    let nch = buf.spec().channels.count();
    let mut out = Vec::with_capacity(buf.frames() * nch);
    for f in 0..buf.frames() {
        for ch in 0..nch {
            out.push(buf.chan(ch)[f]);
        }
    }
    out
}

fn conv<S: Sample + Copy, F: Fn(S) -> f32>(buf: &AudioBuffer<S>, f: F) -> Vec<f32> {
    let nch = buf.spec().channels.count();
    let mut out = Vec::with_capacity(buf.frames() * nch);
    for fr in 0..buf.frames() {
        for ch in 0..nch {
            out.push(f(buf.chan(ch)[fr]));
        }
    }
    out
}

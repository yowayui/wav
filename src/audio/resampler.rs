use crate::error::{Result, WavError};
use rubato::{FftFixedIn, Resampler};

pub struct StreamResampler {
    inner: FftFixedIn<f32>,
    channels: usize,
    chunk_size: usize,
    leftovers: Vec<f32>,
}

impl StreamResampler {
    pub fn new(src_rate: u32, dst_rate: u32, channels: usize) -> Result<Self> {
        let chunk_size = 1024;
        let inner = FftFixedIn::<f32>::new(
            src_rate as usize,
            dst_rate as usize,
            chunk_size,
            2,
            channels,
        )
        .map_err(|e| WavError::Audio(format!("resampler init: {e}")))?;

        Ok(Self {
            inner,
            channels,
            chunk_size,
            leftovers: Vec::new(),
        })
    }

    /// Resample interleaved input → interleaved output.
    pub fn process(&mut self, interleaved: &[f32]) -> Result<Vec<f32>> {
        let mut buf = std::mem::take(&mut self.leftovers);
        buf.extend_from_slice(interleaved);

        let samples_per_chunk = self.chunk_size * self.channels;
        let mut output = Vec::new();

        while buf.len() >= samples_per_chunk {
            let planar: Vec<Vec<f32>> = (0..self.channels)
                .map(|ch| {
                    buf[..samples_per_chunk]
                        .chunks_exact(self.channels)
                        .map(|frame| frame[ch])
                        .collect()
                })
                .collect();

            buf.drain(..samples_per_chunk);

            let resampled = self
                .inner
                .process(
                    &planar.iter().map(|v| v.as_slice()).collect::<Vec<_>>(),
                    None,
                )
                .map_err(|e| WavError::Audio(format!("resample: {e}")))?;

            let frames = resampled[0].len();
            for f in 0..frames {
                for ch in 0..self.channels {
                    output.push(resampled[ch][f]);
                }
            }
        }

        self.leftovers = buf;
        Ok(output)
    }

    /// Flush remaining leftovers (zero-padded). Call once at end-of-stream.
    pub fn flush(&mut self) -> Result<Vec<f32>> {
        if self.leftovers.is_empty() {
            return Ok(Vec::new());
        }
        let samples_per_chunk = self.chunk_size * self.channels;
        // Pad to full chunk then process
        self.leftovers.resize(samples_per_chunk, 0.0);
        let padded = std::mem::take(&mut self.leftovers);
        self.process(&padded)
    }
}

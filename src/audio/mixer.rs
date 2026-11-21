/// Simple channel adapter: convert mono→stereo or stereo→mono as needed.
/// Handles most common case. More exotic layouts handled by decoder normalization.
pub fn adapt_channels(samples: Vec<f32>, src_ch: usize, dst_ch: usize) -> Vec<f32> {
    if src_ch == dst_ch {
        return samples;
    }
    match (src_ch, dst_ch) {
        (1, 2) => {
            // Mono → stereo: duplicate each sample
            let mut out = Vec::with_capacity(samples.len() * 2);
            for s in &samples {
                out.push(*s);
                out.push(*s);
            }
            out
        }
        (2, 1) => {
            // Stereo → mono: average L+R
            samples
                .chunks_exact(2)
                .map(|pair| (pair[0] + pair[1]) * 0.5)
                .collect()
        }
        _ => {
            // Generic: truncate or zero-pad per frame
            let src_frames = samples.len() / src_ch;
            let mut out = Vec::with_capacity(src_frames * dst_ch);
            for f in 0..src_frames {
                for ch in 0..dst_ch {
                    if ch < src_ch {
                        out.push(samples[f * src_ch + ch]);
                    } else {
                        out.push(0.0);
                    }
                }
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_to_stereo() {
        let mono = vec![0.1, 0.2, 0.3];
        let stereo = adapt_channels(mono, 1, 2);
        assert_eq!(stereo, vec![0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
    }

    #[test]
    fn stereo_to_mono() {
        let stereo = vec![0.4, 0.2, 0.6, 0.0];
        let mono = adapt_channels(stereo, 2, 1);
        assert!((mono[0] - 0.3).abs() < 1e-6);
        assert!((mono[1] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn passthrough_when_same() {
        let v = vec![1.0, 2.0];
        let out = adapt_channels(v.clone(), 2, 2);
        assert_eq!(out, v);
    }
}

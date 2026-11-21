use crate::error::{Result, WavError};
use lofty::{Accessor, AudioFile, TaggedFileExt};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TrackMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    /// Duration in seconds
    pub duration: Option<f64>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub codec: Option<String>,
}

#[allow(dead_code)]
impl TrackMeta {
    pub fn display_title<'a>(&'a self, path: &'a Path) -> &'a str {
        self.title
            .as_deref()
            .or_else(|| path.file_stem().and_then(|s| s.to_str()))
            .unwrap_or("Unknown")
    }

    pub fn display_artist(&self) -> &str {
        self.artist.as_deref().unwrap_or("Unknown Artist")
    }

    pub fn display_album(&self) -> &str {
        self.album.as_deref().unwrap_or("Unknown Album")
    }
}

pub fn read_metadata(path: &Path) -> Result<TrackMeta> {
    let tagged =
        lofty::read_from_path(path).map_err(|e| WavError::Metadata(format!("read: {e}")))?;

    let props = tagged.properties();
    let duration = Some(props.duration().as_secs_f64());
    let sample_rate = props.sample_rate();
    let bit_depth = props.bit_depth();

    let mut meta = TrackMeta {
        duration,
        sample_rate,
        bit_depth,
        ..Default::default()
    };

    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        meta.title = tag.title().map(|s| s.to_string());
        meta.artist = tag.artist().map(|s| s.to_string());
        meta.album = tag.album().map(|s| s.to_string());
        meta.genre = tag.genre().map(|s| s.to_string());
        meta.track_number = tag.track();
    }

    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn display_title_fallback_to_filename() {
        let meta = TrackMeta::default();
        let path = PathBuf::from("/music/great_song.mp3");
        assert_eq!(meta.display_title(&path), "great_song");
    }

    #[test]
    fn display_artist_fallback() {
        let meta = TrackMeta::default();
        assert_eq!(meta.display_artist(), "Unknown Artist");
    }
}

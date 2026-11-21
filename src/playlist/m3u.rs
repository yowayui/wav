use crate::error::{Result, WavError};
use std::path::{Path, PathBuf};

/// Parse an M3U or M3U8 file, resolving relative paths against the playlist's directory.
pub fn parse(playlist_path: &Path) -> Result<Vec<PathBuf>> {
    let text = std::fs::read_to_string(playlist_path)?;
    let base = playlist_path.parent().unwrap_or(Path::new("."));

    let tracks = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let p = Path::new(l);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                base.join(p)
            }
        })
        .collect();

    Ok(tracks)
}

/// Write paths to an M3U file (UTF-8, relative if paths share the same parent).
pub fn write(playlist_path: &Path, tracks: &[PathBuf]) -> Result<()> {
    let base = playlist_path.parent().unwrap_or(Path::new("."));
    let mut out = String::from("#EXTM3U\n");
    for track in tracks {
        let entry = if let Ok(rel) = track.strip_prefix(base) {
            rel.to_string_lossy().into_owned()
        } else {
            track.to_string_lossy().into_owned()
        };
        out.push_str(&entry);
        out.push('\n');
    }
    std::fs::write(playlist_path, out).map_err(WavError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_relative_paths() {
        let dir = TempDir::new().unwrap();
        let playlist = dir.path().join("list.m3u");
        let tracks = vec![dir.path().join("a.mp3"), dir.path().join("sub/b.flac")];
        write(&playlist, &tracks).unwrap();
        let loaded = parse(&playlist).unwrap();
        assert_eq!(loaded.len(), 2);
        // Paths are resolved back to absolute equivalents
        assert_eq!(loaded[0], tracks[0]);
        assert_eq!(loaded[1], tracks[1]);
    }

    #[test]
    fn parse_skips_comments_and_blanks() {
        let dir = TempDir::new().unwrap();
        let playlist = dir.path().join("list.m3u");
        std::fs::write(&playlist, "#EXTM3U\n# comment\n\nsong.mp3\n").unwrap();
        let loaded = parse(&playlist).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].file_name().unwrap(), "song.mp3");
    }

    #[test]
    fn parse_absolute_paths_unchanged() {
        let dir = TempDir::new().unwrap();
        let playlist = dir.path().join("list.m3u");
        std::fs::write(&playlist, "/absolute/path/song.mp3\n").unwrap();
        let loaded = parse(&playlist).unwrap();
        assert_eq!(loaded[0], PathBuf::from("/absolute/path/song.mp3"));
    }
}

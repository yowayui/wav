use std::path::Path;
use walkdir::WalkDir;

/// Audio file extensions we recognize.
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "alac"];

/// Yields paths to audio files under `root`, skipping hidden subdirectories.
pub fn scan_audio_files(root: &Path) -> impl Iterator<Item = std::path::PathBuf> {
    WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            // Never skip the root itself (depth 0); skip hidden subdirs
            if e.depth() > 0 && e.file_type().is_dir() {
                return !e.file_name().to_string_lossy().starts_with('.');
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    let lower = ext.to_ascii_lowercase();
                    AUDIO_EXTENSIONS.contains(&lower.as_str())
                })
                .unwrap_or(false)
        })
        .map(|e| e.into_path())
}

/// Returns (file_size, modified_at_secs) for change detection without hashing.
pub fn file_fingerprint(path: &Path) -> Option<(i64, i64)> {
    let meta = std::fs::metadata(path).ok()?;
    let size = meta.len() as i64;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some((size, modified))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn scan_finds_audio_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("song.mp3"), b"").unwrap();
        fs::write(dir.path().join("song.flac"), b"").unwrap();
        fs::write(dir.path().join("readme.txt"), b"").unwrap();

        let found: Vec<_> = scan_audio_files(dir.path()).collect();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn scan_skips_hidden_dirs() {
        let dir = TempDir::new().unwrap();
        let hidden = dir.path().join(".hidden");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("song.mp3"), b"").unwrap();
        fs::write(dir.path().join("visible.mp3"), b"").unwrap();

        let found: Vec<_> = scan_audio_files(dir.path()).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file_name().unwrap(), "visible.mp3");
    }

    #[test]
    fn scan_empty_dir_yields_nothing() {
        let dir = TempDir::new().unwrap();
        let found: Vec<_> = scan_audio_files(dir.path()).collect();
        assert!(found.is_empty());
    }

    #[test]
    fn file_fingerprint_works() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("test.mp3");
        fs::write(&p, b"data").unwrap();
        let (sz, mt) = file_fingerprint(&p).unwrap();
        assert_eq!(sz, 4);
        assert!(mt > 0);
    }
}

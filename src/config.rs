use crate::error::{Result, WavError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// XDG-aware application directories.
pub struct Dirs {
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
}

impl Dirs {
    pub fn new() -> Self {
        let home = home_dir();

        let config = xdg_dir("XDG_CONFIG_HOME", &home, ".config").join("wav");
        let data = xdg_dir("XDG_DATA_HOME", &home, ".local/share").join("wav");
        let state = xdg_dir("XDG_STATE_HOME", &home, ".local/state").join("wav");
        let cache = xdg_dir("XDG_CACHE_HOME", &home, ".cache").join("wav");

        Self {
            config,
            data,
            state,
            cache,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.toml")
    }

    pub fn keybindings_file(&self) -> PathBuf {
        self.config.join("keybindings.toml")
    }

    pub fn database_file(&self) -> PathBuf {
        self.data.join("library.db")
    }

    pub fn ensure_exist(&self) -> Result<()> {
        for dir in [&self.config, &self.data, &self.state, &self.cache] {
            std::fs::create_dir_all(dir)?;
        }
        let config_file = self.config_file();
        if !config_file.exists() {
            let _ = std::fs::write(&config_file, Config::default_template());
        }
        let kb_file = self.keybindings_file();
        if !kb_file.exists() {
            let _ = std::fs::write(
                &kb_file,
                crate::ui::keybindings::Keybindings::default_template(),
            );
        }
        Ok(())
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn xdg_dir(env_var: &str, home: &Path, default_suffix: &str) -> PathBuf {
    std::env::var_os(env_var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(default_suffix))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub playback: PlaybackConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub music_directory: String,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub backend: String,
    pub volume: u8,
    pub replaygain: String,
    pub gapless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackConfig {
    pub resume: bool,
    pub shuffle: bool,
    pub repeat: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub show_album_art: bool,
    pub show_spectrum: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            audio: AudioConfig::default(),
            playback: PlaybackConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        let music_directory = std::env::var("HOME")
            .map(|h| format!("{}/Music", h))
            .unwrap_or_else(|_| "~/Music".to_string());
        Self {
            music_directory,
            theme: "monochrome".to_string(),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: "auto".to_string(),
            volume: 75,
            replaygain: "off".to_string(),
            gapless: false,
        }
    }
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            resume: true,
            shuffle: false,
            repeat: "off".to_string(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_album_art: true,
            show_spectrum: false,
        }
    }
}

impl Config {
    pub fn default_template() -> &'static str {
        r#"# WAV Player Configuration (~/.config/wav/config.toml)

[general]
# Directory containing your music collection (supports ~/ or absolute path)
music_directory = "~/Music"
# TUI color theme: "monochrome"
theme = "monochrome"

[audio]
# Audio output backend: "auto", "alsa", "pulse"
backend = "auto"
# Output volume: 0 - 100
volume = 75
# ReplayGain normalization: "off", "track", "album"
replaygain = "off"
# Gapless playback: true / false
gapless = false

[playback]
# Resume last playing track on startup: true / false
resume = false
# Shuffle tracks: true / false
shuffle = false
# Repeat mode: "off", "one", "all"
repeat = "off"

[ui]
# Show album artwork in terminal: true / false
show_album_art = true
# Show frequency spectrum visualizer: true / false
show_spectrum = false
"#
    }

    /// Load from file, creating default template if missing.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, Self::default_template());
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        toml::from_str(&text).map_err(|e| WavError::Config(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| WavError::Config(e.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        let vol = self.audio.volume;
        if vol > 100 {
            return Err(WavError::Config(format!(
                "audio.volume must be 0-100, got {vol}"
            )));
        }
        let valid_repeat = ["off", "one", "all"];
        if !valid_repeat.contains(&self.playback.repeat.as_str()) {
            return Err(WavError::Config(format!(
                "playback.repeat must be one of {:?}, got {:?}",
                valid_repeat, self.playback.repeat
            )));
        }
        Ok(())
    }
}

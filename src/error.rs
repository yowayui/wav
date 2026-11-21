use thiserror::Error;

#[derive(Debug, Error)]
pub enum WavError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Metadata error: {0}")]
    Metadata(String),

    #[allow(dead_code)]
    #[error("Playlist error: {0}")]
    Playlist(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, WavError>;

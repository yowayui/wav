use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "wav",
    version,
    about = "Your music. Your machine. No distractions.",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Play a file directly
    Play { file: PathBuf },
    /// Scan a directory into the library
    Scan { dir: PathBuf },
    /// Search the library
    Search { query: String },
    /// Show metadata for a file
    Info { file: PathBuf },
    /// List library contents
    Library,
    /// Playlist operations
    Playlist {
        #[command(subcommand)]
        action: PlaylistCommand,
    },
    /// List available audio devices
    Devices,
}

#[derive(Debug, Subcommand)]
pub enum PlaylistCommand {
    /// List all playlists
    List,
    /// Import a playlist file
    Import { file: PathBuf },
    /// Export a playlist by name
    Export { name: String },
}

mod audio;
mod cli;
mod config;
mod error;
mod library;
mod playlist;
mod storage;
mod ui;

use clap::Parser;
use cli::{Cli, Command, PlaylistCommand};
use config::Dirs;
use error::Result;
use library::{metadata::read_metadata, scanner};
use storage::database::Database;
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_target(false)
        .init();

    let dirs = Dirs::new();
    dirs.ensure_exist()?;

    let config = config::Config::load(&dirs.config_file())?;
    config.validate()?;

    let cli = Cli::parse();

    match cli.command {
        None => {
            // Launch TUI
            let db = Database::open(&dirs.database_file())?;
            let app = ui::app::App::new(config, db);
            app.run()?;
        }

        Some(Command::Play { file }) => {
            info!("play: {}", file.display());
            if !file.exists() {
                eprintln!("File not found: {}", file.display());
                std::process::exit(1);
            }
            // Standalone foreground playback — no TUI
            let engine = audio::engine::Engine::start(config.audio.volume, config.audio.gapless);
            engine.play(file.clone());
            println!("Playing: {}", file.display());
            println!("Press Ctrl-C to stop.");

            // Block until end of track or user interrupt
            loop {
                match engine.event_rx.recv() {
                    Ok(audio::engine::EngineEvent::EndOfTrack) => {
                        println!("Done.");
                        engine.quit();
                        break;
                    }
                    Ok(audio::engine::EngineEvent::Error(e)) => {
                        eprintln!("Error: {e}");
                        engine.quit();
                        std::process::exit(1);
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }

        Some(Command::Scan { dir }) => {
            if !dir.exists() {
                eprintln!("Directory not found: {}", dir.display());
                std::process::exit(1);
            }
            let db = Database::open(&dirs.database_file())?;
            println!("Scanning {}...", dir.display());
            let (added, skipped, removed) = do_scan(&db, &dir)?;
            println!(
                "Done. Added/updated: {added}, skipped (unchanged): {skipped}, removed: {removed}"
            );
        }

        Some(Command::Search { query }) => {
            let db = Database::open(&dirs.database_file())?;
            let pattern = format!("%{}%", query.to_lowercase());
            let ids = db.search_tracks(&pattern)?;
            if ids.is_empty() {
                println!("No results for {:?}", query);
            } else {
                println!("Results for {:?}:", query);
                for id in ids {
                    if let Some(t) = db.get_track_by_id(id)? {
                        let artist = t.artist.as_deref().unwrap_or("Unknown Artist");
                        let title = t.title.as_deref().unwrap_or_else(|| {
                            std::path::Path::new(&t.path)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("?")
                        });
                        println!("  {} – {}  [{}]", artist, title, t.path);
                    }
                }
            }
        }

        Some(Command::Info { file }) => {
            if !file.exists() {
                eprintln!("File not found: {}", file.display());
                std::process::exit(1);
            }
            match read_metadata(&file) {
                Ok(meta) => {
                    let fallback_title = file.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                    println!("File   : {}", file.display());
                    println!(
                        "Title  : {}",
                        meta.title.as_deref().unwrap_or(fallback_title)
                    );
                    println!(
                        "Artist : {}",
                        meta.artist.as_deref().unwrap_or("Unknown Artist")
                    );
                    println!(
                        "Album  : {}",
                        meta.album.as_deref().unwrap_or("Unknown Album")
                    );
                    println!("Genre  : {}", meta.genre.as_deref().unwrap_or("?"));
                    if let Some(d) = meta.duration {
                        println!("Duration: {}:{:02}", d as u64 / 60, d as u64 % 60);
                    }
                    if let Some(sr) = meta.sample_rate {
                        println!("Sample rate: {} Hz", sr);
                    }
                    if let Some(bd) = meta.bit_depth {
                        println!("Bit depth: {}", bd);
                    }
                }
                Err(e) => {
                    eprintln!("Could not read metadata: {e}");
                    std::process::exit(1);
                }
            }
        }

        Some(Command::Library) => {
            let db = Database::open(&dirs.database_file())?;
            let tracks = db.get_all_tracks()?;
            if tracks.is_empty() {
                println!("Library is empty. Run `wav scan <DIR>` first.");
            } else {
                println!("{} track(s):", tracks.len());
                for t in &tracks {
                    let artist = t.artist.as_deref().unwrap_or("Unknown Artist");
                    let title = t.title.as_deref().unwrap_or_else(|| {
                        std::path::Path::new(&t.path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("?")
                    });
                    println!("  {} – {}", artist, title);
                }
            }
        }

        Some(Command::Playlist { action }) => match action {
            PlaylistCommand::List => {
                let db = Database::open(&dirs.database_file())?;
                let pls = db.list_playlists()?;
                if pls.is_empty() {
                    println!("No playlists.");
                } else {
                    for pl in &pls {
                        println!("  {}", pl.name);
                    }
                }
            }
            PlaylistCommand::Import { file } => {
                if !file.exists() {
                    eprintln!("File not found: {}", file.display());
                    std::process::exit(1);
                }
                let db = Database::open(&dirs.database_file())?;
                let tracks = playlist::m3u::parse(&file)?;
                let name = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("imported");
                let pl_id = db.create_playlist(name)?;
                let mut pos = 0i64;
                let mut imported = 0usize;
                for track_path in &tracks {
                    let path_str = track_path.to_string_lossy();
                    // Try to find in DB; if not, upsert stub
                    let meta = read_metadata(track_path).unwrap_or_default();
                    if let Some((sz, mt)) = scanner::file_fingerprint(track_path) {
                        let t_id = db.upsert_track(&path_str, &meta, sz, mt)?;
                        db.add_track_to_playlist(pl_id, t_id, pos)?;
                        pos += 1;
                        imported += 1;
                    }
                }
                println!("Imported {imported} track(s) into playlist '{name}'.");
            }
            PlaylistCommand::Export { name } => {
                let db = Database::open(&dirs.database_file())?;
                let pls = db.list_playlists()?;
                let pl = pls.iter().find(|p| p.name == name).ok_or_else(|| {
                    error::WavError::NotFound(format!("playlist '{name}' not found"))
                })?;
                let tracks = db.get_playlist_tracks(pl.id)?;
                let paths: Vec<_> = tracks
                    .iter()
                    .map(|t| std::path::PathBuf::from(&t.path))
                    .collect();
                let out = std::path::PathBuf::from(format!("{name}.m3u"));
                playlist::m3u::write(&out, &paths)?;
                println!("Exported {} track(s) to {}", paths.len(), out.display());
            }
        },

        Some(Command::Devices) => match audio::output::list_devices() {
            Ok(devices) if devices.is_empty() => println!("No output devices found."),
            Ok(devices) => {
                println!("Available output devices:");
                for (i, name) in devices.iter().enumerate() {
                    println!("  {}: {}", i, name);
                }
            }
            Err(e) => {
                eprintln!("Error listing devices: {e}");
                std::process::exit(1);
            }
        },
    }

    Ok(())
}

/// Incremental scan: only extract metadata for files that changed, and prune deleted.
fn do_scan(db: &Database, dir: &std::path::Path) -> Result<(usize, usize, usize)> {
    let mut added = 0usize;
    let mut skipped = 0usize;

    for path in scanner::scan_audio_files(dir) {
        let path_str = path.to_string_lossy().to_string();
        if let Some((size, mtime)) = scanner::file_fingerprint(&path) {
            if db.track_needs_update(&path_str, size, mtime)? {
                let meta = match read_metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("metadata error for {}: {e}", path.display());
                        library::metadata::TrackMeta::default()
                    }
                };
                db.upsert_track(&path_str, &meta, size, mtime)?;
                added += 1;
            } else {
                skipped += 1;
            }
        }
    }

    let removed = db.prune_missing_tracks()?;
    Ok((added, skipped, removed))
}

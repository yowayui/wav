# Architecture

## Overview

WAV is a single Cargo binary with internal modules. No separate daemon. No IPC server.

## Module Map

```
src/
  main.rs          — entry point, CLI dispatch
  cli.rs           — clap argument definitions
  config.rs        — XDG paths, TOML config, defaults
  error.rs         — WavError enum, Result alias

  audio/
    engine.rs      — playback state machine
    decoder.rs     — Symphonia demux/decode → PCM f32
    output.rs      — CPAL audio output thread
    resampler.rs   — Rubato resampling
    mixer.rs       — volume, channel mapping

  library/
    scanner.rs     — walkdir audio file discovery
    metadata.rs    — lofty tag extraction
    search.rs      — SQLite query helpers

  playlist/
    queue.rs       — in-memory playback queue
    m3u.rs         — M3U/M3U8 parse + write

  storage/
    database.rs    — rusqlite connection, migrations
    schema.sql     — DDL (tracks, playlists, history)

  ui/
    app.rs         — ratatui event loop
    event.rs       — crossterm input events
    keybindings.rs — configurable key → action map
    layout.rs      — ratatui layout helpers
    screens/       — per-screen render logic
```

## Audio Pipeline

```
File → Symphonia Demuxer → Decoder → PCM f32
     → Channel Mapping → Rubato Resampler (if needed)
     → Ringbuf Producer (decoder thread)
     → Ringbuf Consumer (CPAL callback, audio thread)
     → Audio Device
```

The decoder thread and the CPAL callback share only the ring buffer.
The CPAL callback never blocks, never allocates, never does I/O.

## Threading Model

| Thread         | Responsibility                          |
|----------------|-----------------------------------------|
| Main           | CLI dispatch, TUI event loop            |
| Decoder worker | Symphonia decode, ring buffer fill      |
| CPAL callback  | Consume ring buffer, write to device    |
| Scanner worker | Walkdir + metadata, DB writes           |

## Configuration

XDG Base Directory Specification:

| Purpose    | Path                          |
|------------|-------------------------------|
| Config     | `$XDG_CONFIG_HOME/wav/`       |
| Data       | `$XDG_DATA_HOME/wav/`         |
| State      | `$XDG_STATE_HOME/wav/`        |
| Cache      | `$XDG_CACHE_HOME/wav/`        |

## Database

SQLite with WAL mode. Schema versioned via `schema_version` table.
Migrations are additive; existing data is never destroyed.
Artwork stored as files in cache, not as blobs in SQLite.

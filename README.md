# WAV

**Your music. Your machine. No distractions.**

WAV is a minimalist local music player for Linux. It plays your music files. Nothing else.

## Features

- **Format support**: MP3, FLAC, WAV, OGG Vorbis, AIFF
- **Audio engine**: CPAL output, Rubato high-quality resampling, Symphonia decoding, gapless playback
- **Terminal UI**: Ratatui-powered TUI with keyboard and mouse scroll navigation
- **Real-time feedback**: Elapsed/duration counter and visual progress bar
- **Live Search**: Instant library filtering as you type
- **Session Resume**: Remembers your last playing track and playback position
- **Music library**: Fast SQLite metadata cache with automatic incremental sync
- **Playlists**: In-TUI playlist management and M3U/M3U8 import/export
- **XDG-compliant**: Config (`~/.config/wav/`), data (`~/.local/share/wav/`), state (`~/.local/state/wav/`)
- **Zero bloat**: No accounts, no internet access, no telemetry, no background daemon

## Installation

### Prerequisites

- Rust (stable toolchain)
- ALSA development libraries:
  - Ubuntu/Debian: `sudo apt install libasound2-dev pkg-config`
  - Arch Linux: `sudo pacman -S alsa-lib pkg-config`
  - Fedora: `sudo dnf install alsa-lib-devel pkgconf-pkg-config`

### Build & Install

```sh
# Clone and build
git clone https://github.com/yowayui/wav.git
cd wav
cargo build --release

# Install to ~/.local/bin
cp target/release/wav ~/.local/bin/
```

Or run the included `./install.sh`:
```sh
./install.sh
```

## Usage

```sh
wav                        # launch interactive TUI
wav play FILE              # play an audio file directly
wav scan DIR               # scan directory into library
wav search QUERY           # search library via CLI
wav info FILE              # show file metadata
wav library                # list library tracks
wav playlist list          # list playlists
wav playlist import FILE   # import M3U playlist
wav playlist export NAME   # export playlist to M3U
wav devices                # list available audio output devices
wav --version
```

## Keybindings

| Key | Action |
|-----|--------|
| `Space` | Play selected / Pause |
| `Enter` | Play selected track |
| `c` | Toggle Pause / Resume |
| `x` / `v` | Stop playback |
| `n` | Next track |
| `p` | Previous track |
| `j` / `k` / `↑` / `↓` | Navigate list (mouse scroll supported) |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `PgUp` / `PgDn` | Scroll page up / down |
| `h` / `l` / `←` / `→` | Seek -10s / +10s |
| `+` / `-` | Volume up / down |
| `s` | Toggle shuffle |
| `r` | Cycle repeat mode (`off` / `all` / `one`) |
| `R` / `F5` | Rescan and synchronize music directory |
| `a` | Add selected track to playlist (in Library) |
| `d` | Delete selected playlist (in Playlists) |
| `/` | Live search (Esc to cancel) |
| `1` – `4` | Switch screen (Library, Playlists, Queue, Settings) |
| `?` | Toggle help popup |
| `q` | Save state and quit |

Keybindings can be customized in `~/.config/wav/keybindings.toml`.

## Configuration

Configuration is located at `~/.config/wav/config.toml` (auto-generated on first run):

```toml
[general]
music_directory = "~/Music"
theme = "monochrome"

[audio]
backend = "auto"
volume = 75
replaygain = "off"
gapless = false

[playback]
resume = true
shuffle = false
repeat = "off"

[ui]
show_album_art = true
show_spectrum = false
```

## License

MIT OR Apache-2.0

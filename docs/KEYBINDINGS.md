# Keybindings

Default keybindings. Configurable in `~/.config/wav/keybindings.toml`.

| Key | Action |
|-----|--------|
| `Space` | Play selected / Pause |
| `Enter` | Play selected track |
| `c` | Toggle Pause / Resume |
| `x` / `v` | Stop playback |
| `n` | Next track |
| `p` | Previous track |
| `j` / `k` / `↓` / `↑` | Navigate down / up |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `PgUp` / `PgDn` | Scroll page up / down |
| `h` / `l` / `←` / `→` | Seek -10s / +10s |
| `+` / `-` | Volume up / down |
| `s` | Toggle shuffle |
| `r` | Cycle repeat mode |
| `R` / `F5` | Rescan / sync library |
| `a` | Add selected track to playlist (in Library) |
| `d` | Delete selected playlist (in Playlists) |
| `/` | Search library |
| `1`–`4` | Switch screen |
| `?` | Toggle help popup |
| `q` | Save state and quit |

## Custom keybindings

Keybindings are configured in `~/.config/wav/keybindings.toml`:

```toml
Space = "PlayPause"
c = "TogglePause"
x = "Stop"
n = "Next"
p = "Previous"
```

Available actions:
`PlayPause`, `TogglePause`, `Stop`, `Next`, `Previous`, `SeekForward`, `SeekBackward`,
`VolumeUp`, `VolumeDown`, `ToggleShuffle`, `ToggleRepeat`, `Rescan`, `Search`,
`NavigateDown`, `NavigateUp`, `NavigateTop`, `NavigateBottom`, `PageUp`, `PageDown`,
`Confirm`, `AddToPlaylist`, `Delete`, `Help`, `Quit`.

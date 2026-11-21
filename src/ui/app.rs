use crate::audio::engine::{Engine, EngineEvent};
use crate::config::Config;
use crate::error::Result;
use crate::library::metadata::read_metadata;
use crate::library::scanner;
use crate::playlist::queue::Queue;
use crate::storage::database::Database;
use crate::ui::{
    event::{Event, EventLoop},
    keybindings::{Action, Keybindings},
    layout::vertical_split,
    screens::{
        library::LibraryScreen, playlists::PlaylistsScreen, queue::QueueScreen,
        settings::SettingsScreen,
    },
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Terminal,
};
use std::{
    io::{self, Stdout},
    path::PathBuf,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Library = 0,
    Playlists = 1,
    Queue = 2,
    Settings = 3,
}

impl Screen {
    fn from_u8(n: u8) -> Self {
        match n {
            1 => Screen::Library,
            2 => Screen::Playlists,
            3 => Screen::Queue,
            4 => Screen::Settings,
            _ => Screen::Library,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::All => "all",
            Self::One => "one",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct PlaybackState {
    last_track: Option<String>,
    position_secs: f64,
}

pub struct App {
    config: Config,
    db: Database,
    engine: Engine,
    keybindings: Keybindings,
    queue: Queue,
    screen: Screen,
    lib_screen: LibraryScreen,
    pl_screen: PlaylistsScreen,
    queue_screen: QueueScreen,
    now_playing: Option<PathBuf>,
    paused: bool,
    volume: u8,
    shuffle: bool,
    repeat: RepeatMode,
    show_help: bool,
    search_mode: bool,
    search_query: String,
    status_msg: Option<String>,
    position_secs: f64,
    duration_secs: Option<f64>,
}

impl App {
    pub fn new(config: Config, db: Database) -> Self {
        let volume = config.audio.volume;
        let shuffle = config.playback.shuffle;
        let repeat = match config.playback.repeat.as_str() {
            "one" => RepeatMode::One,
            "all" => RepeatMode::All,
            _ => RepeatMode::Off,
        };
        let engine = Engine::start(volume, config.audio.gapless);
        let dirs = crate::config::Dirs::new();
        let keybindings = Keybindings::load(&dirs.keybindings_file());

        Self {
            config,
            db,
            engine,
            keybindings,
            queue: Queue::new(),
            screen: Screen::Library,
            lib_screen: LibraryScreen::new(),
            pl_screen: PlaylistsScreen::new(),
            queue_screen: QueueScreen::new(),
            now_playing: None,
            paused: false,
            volume,
            shuffle,
            repeat,
            show_help: false,
            search_mode: false,
            search_query: String::new(),
            status_msg: None,
            position_secs: 0.0,
            duration_secs: None,
        }
    }

    pub fn run(mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;
        let result = self.event_loop(&mut terminal);
        restore_terminal(&mut terminal)?;
        result
    }

    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        // Auto-sync music_directory on startup (incremental scan + prune deleted files)
        let dir = PathBuf::from(shellexpand::tilde(&self.config.general.music_directory).as_ref());
        if dir.exists() {
            self.do_scan_blocking(&dir);
        }
        self.refresh_library();
        self.refresh_playlists();
        self.rebuild_queue_from_library();

        if self.config.playback.resume {
            self.load_and_resume_state();
        }

        let events = EventLoop::new(100);

        loop {
            terminal.draw(|f| self.render(f))?;

            // Drain audio events (non-blocking)
            while let Ok(ae) = self.engine.event_rx.try_recv() {
                self.handle_audio_event(ae);
            }

            match events.rx.recv() {
                Ok(Event::Key(k)) => {
                    if self.search_mode {
                        if self.handle_search_key(k) {
                            return Ok(());
                        }
                    } else if let Some(action) = self.keybindings.lookup(&k).cloned() {
                        if self.handle_action(action) {
                            return Ok(());
                        }
                    }
                }
                Ok(Event::Mouse(m)) => match m.kind {
                    MouseEventKind::ScrollDown => self.navigate_down(),
                    MouseEventKind::ScrollUp => self.navigate_up(),
                    _ => {}
                },
                Ok(Event::Tick) => {}
                Ok(_) => {}
                Err(_) => return Ok(()),
            }
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        self.status_msg = None;
        match action {
            Action::Quit => {
                self.save_state();
                self.engine.quit();
                self.save_config();
                return true;
            }
            Action::Help => self.show_help = !self.show_help,
            Action::SwitchScreen(n) => self.screen = Screen::from_u8(n),
            Action::Search => {
                self.search_mode = true;
                self.search_query.clear();
            }
            Action::PlayPause => {
                let sel = self.selected_path();
                if let (Some(curr), Some(ref sel_path)) = (&self.now_playing, &sel) {
                    if curr == sel_path {
                        // Same track: toggle pause / resume
                        if self.paused {
                            self.engine.resume();
                            self.paused = false;
                        } else {
                            self.engine.pause();
                            self.paused = true;
                        }
                        return false;
                    }
                }
                // Different track selected or nothing playing: play selected
                if self.screen == Screen::Settings && self.now_playing.is_some() {
                    if self.paused {
                        self.engine.resume();
                        self.paused = false;
                    } else {
                        self.engine.pause();
                        self.paused = true;
                    }
                } else {
                    self.play_selected();
                }
            }
            Action::TogglePause => {
                if self.now_playing.is_some() {
                    if self.paused {
                        self.engine.resume();
                        self.paused = false;
                    } else {
                        self.engine.pause();
                        self.paused = true;
                    }
                }
            }
            Action::Stop => {
                self.engine.stop();
                self.now_playing = None;
                self.paused = false;
                self.position_secs = 0.0;
                self.duration_secs = None;
            }
            Action::Confirm => self.play_selected(),
            Action::Next => self.play_next(),
            Action::Previous => self.play_prev(),
            Action::SeekForward => self.engine.seek(10.0),
            Action::SeekBackward => self.engine.seek(-10.0),
            Action::VolumeUp => {
                self.volume = self.volume.saturating_add(5).min(100);
                self.engine.set_volume(self.volume);
                self.config.audio.volume = self.volume;
                self.save_config();
            }
            Action::VolumeDown => {
                self.volume = self.volume.saturating_sub(5);
                self.engine.set_volume(self.volume);
                self.config.audio.volume = self.volume;
                self.save_config();
            }
            Action::ToggleShuffle => {
                self.shuffle = !self.shuffle;
                self.config.playback.shuffle = self.shuffle;
                self.save_config();
                if self.shuffle {
                    self.queue.shuffle();
                    self.sync_queue_screen();
                }
            }
            Action::ToggleRepeat => {
                self.repeat = self.repeat.next();
                self.config.playback.repeat = self.repeat.label().to_string();
                self.save_config();
            }
            Action::Rescan => {
                let dir = PathBuf::from(
                    shellexpand::tilde(&self.config.general.music_directory).as_ref(),
                );
                if dir.exists() {
                    self.do_scan_blocking(&dir);
                    self.refresh_library();
                    self.rebuild_queue_from_library();
                }
            }
            Action::NavigateDown => self.navigate_down(),
            Action::NavigateUp => self.navigate_up(),
            Action::NavigateTop => self.navigate_top(),
            Action::NavigateBottom => self.navigate_bottom(),
            Action::PageUp => self.navigate_page_up(),
            Action::PageDown => self.navigate_page_down(),
            Action::AddToPlaylist => {
                if self.screen == Screen::Library {
                    if let Some(track) = self.lib_screen.selected_track() {
                        let pls = self.db.list_playlists().unwrap_or_default();
                        let (pl_id, pl_name) = if pls.is_empty() {
                            let id = self.db.create_playlist("Favorites").unwrap_or(1);
                            (id, "Favorites".to_string())
                        } else {
                            (pls[0].id, pls[0].name.clone())
                        };
                        let pos = self
                            .db
                            .get_playlist_tracks(pl_id)
                            .map(|t| t.len() as i64)
                            .unwrap_or(0);
                        let _ = self.db.add_track_to_playlist(pl_id, track.id, pos);
                        self.refresh_playlists();
                        self.status_msg = Some(format!("Added to '{}'", pl_name));
                    }
                }
            }
            Action::Delete => {
                if self.screen == Screen::Playlists {
                    if let Some(pl) = self.pl_screen.selected().cloned() {
                        let _ = self.db.delete_playlist(pl.id);
                        self.refresh_playlists();
                        self.status_msg = Some(format!("Deleted playlist '{}'", pl.name));
                    }
                }
            }
        }
        false
    }

    fn handle_search_key(&mut self, k: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        match k.code {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query.clear();
                self.refresh_library();
            }
            KeyCode::Enter => {
                self.search_mode = false;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.do_search();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.do_search();
            }
            _ => {}
        }
        false
    }

    fn handle_audio_event(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::Playing {
                path,
                duration_secs,
            } => {
                self.now_playing = Some(path.clone());
                self.paused = false;
                self.position_secs = 0.0;
                let path_str = path.to_string_lossy();
                self.duration_secs = duration_secs.or_else(|| {
                    self.db
                        .get_all_tracks()
                        .ok()?
                        .into_iter()
                        .find(|t| t.path == path_str.as_ref())
                        .and_then(|t| t.duration)
                });
            }
            EngineEvent::Position {
                position_secs,
                duration_secs,
            } => {
                self.position_secs = position_secs;
                if self.duration_secs.is_none() {
                    self.duration_secs = duration_secs;
                }
            }
            EngineEvent::Paused => self.paused = true,
            EngineEvent::Resumed => self.paused = false,
            EngineEvent::Stopped => {
                self.now_playing = None;
                self.paused = false;
                self.position_secs = 0.0;
                self.duration_secs = None;
            }
            EngineEvent::EndOfTrack => match self.repeat {
                RepeatMode::One => {
                    if let Some(p) = self.now_playing.clone() {
                        self.engine.play(p);
                    }
                }
                RepeatMode::All => {
                    self.play_next();
                }
                RepeatMode::Off => {
                    if let Some(path) = self.queue.advance().cloned() {
                        self.engine.play(path);
                        self.sync_queue_screen();
                    } else {
                        self.now_playing = None;
                        self.paused = false;
                        self.position_secs = 0.0;
                        self.duration_secs = None;
                    }
                }
            },
            EngineEvent::Error(e) => {
                tracing::error!("audio: {e}");
                self.status_msg = Some(format!("Error: {e}"));
            }
        }
    }

    fn selected_path(&self) -> Option<PathBuf> {
        match self.screen {
            Screen::Library => self
                .lib_screen
                .selected_track()
                .map(|t| PathBuf::from(&t.path)),
            Screen::Queue => self
                .queue_screen
                .state
                .selected()
                .and_then(|idx| self.queue.get(idx))
                .cloned(),
            Screen::Playlists | Screen::Settings => None,
        }
    }

    fn play_selected(&mut self) {
        match self.screen {
            Screen::Library => {
                if let Some(track) = self.lib_screen.selected_track() {
                    let path = PathBuf::from(&track.path);
                    self.queue.clear();
                    for t in self.lib_screen.tracks() {
                        self.queue.push(PathBuf::from(&t.path));
                    }
                    self.queue.set_to_path(&path);
                    self.engine.play(path);
                    self.sync_queue_screen();
                }
            }
            Screen::Queue => {
                if let Some(idx) = self.queue_screen.state.selected() {
                    if let Some(path) = self.queue.get(idx).cloned() {
                        self.queue.set_index(idx);
                        self.engine.play(path);
                        self.sync_queue_screen();
                    }
                }
            }
            Screen::Playlists => {
                if let Some(pl) = self.pl_screen.selected() {
                    if let Ok(tracks) = self.db.get_playlist_tracks(pl.id) {
                        self.queue.clear();
                        for t in &tracks {
                            self.queue.push(PathBuf::from(&t.path));
                        }
                        if let Some(path) = self.queue.advance().cloned() {
                            self.engine.play(path);
                        }
                        self.sync_queue_screen();
                        self.screen = Screen::Queue;
                    }
                }
            }
            Screen::Settings => {}
        }
    }

    fn play_next(&mut self) {
        if let Some(path) = self.queue.advance().cloned() {
            self.engine.play(path);
            self.sync_queue_screen();
        } else if !self.queue.is_empty() {
            self.queue.reset();
            if let Some(path) = self.queue.advance().cloned() {
                self.engine.play(path);
                self.sync_queue_screen();
            }
        }
    }

    fn play_prev(&mut self) {
        if let Some(path) = self.queue.retreat().cloned() {
            self.engine.play(path);
            self.sync_queue_screen();
        }
    }

    fn navigate_down(&mut self) {
        match self.screen {
            Screen::Library => self.lib_screen.next(),
            Screen::Playlists => self.pl_screen.next(),
            Screen::Queue => self.queue_screen.next(),
            Screen::Settings => {}
        }
    }

    fn navigate_up(&mut self) {
        match self.screen {
            Screen::Library => self.lib_screen.prev(),
            Screen::Playlists => self.pl_screen.prev(),
            Screen::Queue => self.queue_screen.prev(),
            Screen::Settings => {}
        }
    }

    fn navigate_top(&mut self) {
        match self.screen {
            Screen::Library => self.lib_screen.top(),
            Screen::Playlists => self.pl_screen.top(),
            Screen::Queue => self.queue_screen.top(),
            Screen::Settings => {}
        }
    }

    fn navigate_bottom(&mut self) {
        match self.screen {
            Screen::Library => self.lib_screen.bottom(),
            Screen::Playlists => self.pl_screen.bottom(),
            Screen::Queue => self.queue_screen.bottom(),
            Screen::Settings => {}
        }
    }

    fn navigate_page_up(&mut self) {
        match self.screen {
            Screen::Library => self.lib_screen.page_up(15),
            Screen::Playlists => self.pl_screen.page_up(15),
            Screen::Queue => self.queue_screen.page_up(15),
            Screen::Settings => {}
        }
    }

    fn navigate_page_down(&mut self) {
        match self.screen {
            Screen::Library => self.lib_screen.page_down(15),
            Screen::Playlists => self.pl_screen.page_down(15),
            Screen::Queue => self.queue_screen.page_down(15),
            Screen::Settings => {}
        }
    }

    fn save_config(&self) {
        let dirs = crate::config::Dirs::new();
        let _ = self.config.save(&dirs.config_file());
    }

    fn save_state(&self) {
        let dirs = crate::config::Dirs::new();
        let state_path = dirs.state.join("state.toml");
        let state = PlaybackState {
            last_track: self
                .now_playing
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            position_secs: self.position_secs,
        };
        if let Ok(s) = toml::to_string(&state) {
            let _ = std::fs::write(state_path, s);
        }
    }

    fn load_and_resume_state(&mut self) {
        let dirs = crate::config::Dirs::new();
        let state_path = dirs.state.join("state.toml");
        if let Ok(text) = std::fs::read_to_string(&state_path) {
            if let Ok(state) = toml::from_str::<PlaybackState>(&text) {
                if let Some(ref path_str) = state.last_track {
                    let p = PathBuf::from(path_str);
                    if p.exists() {
                        self.queue.set_to_path(&p);
                        self.engine.play(p.clone());
                        if state.position_secs > 1.0 {
                            self.engine.seek(state.position_secs);
                        }
                        self.sync_queue_screen();
                        let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("track");
                        self.status_msg = Some(format!("Resumed {}", name));
                    }
                }
            }
        }
    }

    fn do_search(&mut self) {
        if self.search_query.is_empty() {
            self.refresh_library();
            self.status_msg = None;
            return;
        }
        let pattern = format!("%{}%", self.search_query.to_lowercase());
        if let Ok(ids) = self.db.search_tracks(&pattern) {
            let tracks: Vec<_> = ids
                .into_iter()
                .filter_map(|id| self.db.get_track_by_id(id).ok().flatten())
                .collect();
            let n = tracks.len();
            self.lib_screen.set_tracks(tracks);
            self.screen = Screen::Library;
            self.status_msg = Some(format!("{n} result(s) for \"{}\"", self.search_query));
        }
    }

    fn do_scan_blocking(&mut self, dir: &PathBuf) {
        let mut added = 0usize;
        for path in scanner::scan_audio_files(dir) {
            let path_str = path.to_string_lossy().to_string();
            if let Some((sz, mt)) = scanner::file_fingerprint(&path) {
                if self
                    .db
                    .track_needs_update(&path_str, sz, mt)
                    .unwrap_or(true)
                {
                    let meta = read_metadata(&path).unwrap_or_default();
                    let _ = self.db.upsert_track(&path_str, &meta, sz, mt);
                    added += 1;
                }
            }
        }
        let removed = self.db.prune_missing_tracks().unwrap_or(0);
        if added > 0 || removed > 0 {
            self.status_msg = Some(format!("Synced: +{added} -{removed} track(s)"));
        }
    }

    fn refresh_library(&mut self) {
        if let Ok(tracks) = self.db.get_all_tracks() {
            self.lib_screen.set_tracks(tracks);
        }
    }

    fn refresh_playlists(&mut self) {
        if let Ok(pls) = self.db.list_playlists() {
            self.pl_screen.set_playlists(pls);
        }
    }

    fn rebuild_queue_from_library(&mut self) {
        self.queue.clear();
        for track in self.lib_screen.tracks() {
            self.queue.push(PathBuf::from(&track.path));
        }
        self.sync_queue_screen();
    }

    fn sync_queue_screen(&mut self) {
        let tracks: Vec<PathBuf> = self.queue.all().to_vec();
        let current = self.queue.current_index();
        self.queue_screen.set_tracks(tracks, current);
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = f.size();
        let (tab_area, content_area, status_area) = vertical_split(area, 1, 4);
        self.render_tabs(f, tab_area);
        self.render_content(f, content_area);
        self.render_status(f, status_area);
        if self.show_help {
            self.render_help(f, area);
        }
        if self.search_mode {
            self.render_search_bar(f, area);
        }
    }

    fn render_tabs(&self, f: &mut ratatui::Frame, area: Rect) {
        let titles = vec![
            Line::from("1:Library"),
            Line::from("2:Playlists"),
            Line::from("3:Queue"),
            Line::from("4:Settings"),
        ];
        let tabs = Tabs::new(titles)
            .select(self.screen as usize)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        f.render_widget(tabs, area);
    }

    fn render_content(&mut self, f: &mut ratatui::Frame, area: Rect) {
        match self.screen {
            Screen::Library => self.lib_screen.render(f, area),
            Screen::Playlists => self.pl_screen.render(f, area),
            Screen::Queue => self.queue_screen.render(f, area),
            Screen::Settings => SettingsScreen::render(f, area, &self.config),
        }
    }

    fn render_status(&self, f: &mut ratatui::Frame, area: Rect) {
        let track_info = if let Some(ref msg) = self.status_msg {
            msg.as_str().to_string()
        } else {
            self.now_playing
                .as_ref()
                .and_then(|p| p.file_stem())
                .and_then(|s| s.to_str())
                .unwrap_or("stopped")
                .to_string()
        };

        let icon = if self.now_playing.is_some() && !self.paused {
            "▶"
        } else if self.paused {
            "⏸"
        } else {
            "⏹"
        };
        let shuf = if self.shuffle { "S" } else { "-" };

        let pos_str = fmt_time(self.position_secs);
        let dur_str = self
            .duration_secs
            .map(fmt_time)
            .unwrap_or_else(|| "--:--".into());

        let line1 = Line::from(vec![
            Span::raw(format!("{icon} ")),
            Span::styled(track_info, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "  vol:{}%  shuf:{}  repeat:{}  [?]help  [q]quit",
                self.volume,
                shuf,
                self.repeat.label()
            )),
        ]);

        let line2 = if self.now_playing.is_some() {
            let bar_width = (area.width.saturating_sub(20)).clamp(10, 50) as usize;
            let ratio = if let Some(dur) = self.duration_secs {
                if dur > 0.0 {
                    (self.position_secs / dur).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            let bar = progress_bar(ratio, bar_width);
            Line::from(vec![
                Span::raw(format!("{pos_str} ")),
                Span::styled(bar, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!(" {dur_str}")),
            ])
        } else {
            Line::from(Span::raw(""))
        };

        let para = Paragraph::new(vec![line1, line2]).block(Block::default().borders(Borders::TOP));
        f.render_widget(para, area);
    }

    fn render_help(&self, f: &mut ratatui::Frame, area: Rect) {
        use ratatui::widgets::Clear;
        let popup = centered_rect(65, 85, area);
        f.render_widget(Clear, popup);
        let lines = vec![
            Line::from("  WAV — Keybindings"),
            Line::from(""),
            Line::from("  Space          Play selected / Pause"),
            Line::from("  Enter          Play selected"),
            Line::from("  c              Toggle Pause"),
            Line::from("  x / v          Stop"),
            Line::from("  n              Next track"),
            Line::from("  p              Previous track"),
            Line::from("  j / k / ↑ / ↓  Navigate list"),
            Line::from("  g / Home       Jump to top"),
            Line::from("  G / End        Jump to bottom"),
            Line::from("  PgUp / PgDn    Scroll page"),
            Line::from("  h / l / ← / →  Seek -10s / +10s"),
            Line::from("  + / -          Volume up / down"),
            Line::from("  s              Toggle shuffle"),
            Line::from("  r              Cycle repeat (off/all/one)"),
            Line::from("  R / F5         Rescan / sync library"),
            Line::from("  a              Add track to playlist"),
            Line::from("  d              Delete playlist (in Playlists)"),
            Line::from("  /              Search library (Esc to cancel)"),
            Line::from("  1-4            Switch screen"),
            Line::from("  ?              Toggle this help"),
            Line::from("  q              Quit"),
        ];
        let para =
            Paragraph::new(lines).block(Block::default().title("Help").borders(Borders::ALL));
        f.render_widget(para, popup);
    }

    fn render_search_bar(&self, f: &mut ratatui::Frame, area: Rect) {
        use ratatui::widgets::Clear;
        let search_area = Rect {
            x: area.x + 2,
            y: area.height.saturating_sub(3),
            width: area.width.saturating_sub(4),
            height: 3,
        };
        f.render_widget(Clear, search_area);
        let para = Paragraph::new(Line::from(format!("/{}_", self.search_query))).block(
            Block::default()
                .title("Search  [Enter] confirm  [Esc] cancel")
                .borders(Borders::ALL),
        );
        f.render_widget(para, search_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let vchunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vchunks[1])[1]
}

fn fmt_time(secs: f64) -> String {
    let s = secs as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn progress_bar(ratio: f64, width: usize) -> String {
    let filled = ((width as f64) * ratio).round() as usize;
    let filled = filled.min(width);
    let mut s = String::with_capacity(width + 2);
    s.push('[');
    for i in 0..width {
        if i < filled {
            s.push('━');
        } else if i == filled && filled < width {
            s.push('●');
        } else {
            s.push('─');
        }
    }
    s.push(']');
    s
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

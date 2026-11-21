use crate::storage::database::Track;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct LibraryScreen {
    pub tracks: Vec<Track>,
    pub state: ListState,
}

impl LibraryScreen {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            state: ListState::default(),
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.tracks = tracks;
        // Keep selection valid; default to 0 if currently none
        if self.state.selected().is_none() && !self.tracks.is_empty() {
            self.state.select(Some(0));
        }
        // Clamp selection if list shrank
        if let Some(i) = self.state.selected() {
            if i >= self.tracks.len() {
                self.state.select(self.tracks.len().checked_sub(1));
            }
        }
    }

    pub fn selected_track(&self) -> Option<&Track> {
        self.state.selected().and_then(|i| self.tracks.get(i))
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn next(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        let i = self
            .state
            .selected()
            .map(|i| (i + 1).min(self.tracks.len() - 1))
            .unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        let i = self
            .state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn top(&mut self) {
        if !self.tracks.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn bottom(&mut self) {
        if !self.tracks.is_empty() {
            self.state.select(Some(self.tracks.len() - 1));
        }
    }

    pub fn page_down(&mut self, step: usize) {
        if self.tracks.is_empty() {
            return;
        }
        let i = self
            .state
            .selected()
            .map(|i| (i + step).min(self.tracks.len() - 1))
            .unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn page_up(&mut self, step: usize) {
        if self.tracks.is_empty() {
            return;
        }
        let i = self
            .state
            .selected()
            .map(|i| i.saturating_sub(step))
            .unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .tracks
            .iter()
            .map(|t| {
                let artist = t.artist.as_deref().unwrap_or("Unknown Artist");
                let title = t.title.as_deref().unwrap_or_else(|| {
                    std::path::Path::new(&t.path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?")
                });
                let dur = t.duration.map(fmt_duration).unwrap_or_default();
                // Truncate display line safely by characters to avoid UTF-8 panic and wrapping
                let info = format!("{} – {}", artist, title);
                let truncated = truncate_str(&info, 60);
                let line = Line::from(vec![
                    Span::raw(truncated),
                    Span::raw(format!("  {:>5}", dur)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let title = format!("Library  ({} tracks)", self.tracks.len());
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.state);
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count > max_chars {
        let prefix: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{prefix}…")
    } else {
        s.to_string()
    }
}

fn fmt_duration(secs: f64) -> String {
    let s = secs as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

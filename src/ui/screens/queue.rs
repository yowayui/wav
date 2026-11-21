use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::path::PathBuf;

pub struct QueueScreen {
    pub tracks: Vec<PathBuf>,
    pub current: Option<usize>,
    pub state: ListState,
}

impl QueueScreen {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            current: None,
            state: ListState::default(),
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<PathBuf>, current: Option<usize>) {
        self.tracks = tracks;
        self.current = current;
        // Auto-scroll to current
        if let Some(c) = current {
            self.state.select(Some(c));
        }
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
            .enumerate()
            .map(|(i, p)| {
                let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                let prefix = if Some(i) == self.current {
                    "▶ "
                } else {
                    "  "
                };
                ListItem::new(Line::from(format!("{}{}", prefix, name)))
            })
            .collect();

        let title = match (self.current, self.tracks.len()) {
            (Some(curr), len) if len > 0 => {
                format!("Queue  ({} tracks — playing #{}/{})", len, curr + 1, len)
            }
            (_, len) => format!("Queue  ({} tracks)", len),
        };

        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_stateful_widget(list, area, &mut self.state);
    }
}

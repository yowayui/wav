use crate::storage::database::Playlist;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct PlaylistsScreen {
    pub playlists: Vec<Playlist>,
    pub state: ListState,
}

impl PlaylistsScreen {
    pub fn new() -> Self {
        Self {
            playlists: Vec::new(),
            state: ListState::default(),
        }
    }

    pub fn set_playlists(&mut self, playlists: Vec<Playlist>) {
        self.playlists = playlists;
        if self.state.selected().is_none() && !self.playlists.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn selected(&self) -> Option<&Playlist> {
        self.state.selected().and_then(|i| self.playlists.get(i))
    }

    pub fn next(&mut self) {
        if self.playlists.is_empty() {
            return;
        }
        let i = self
            .state
            .selected()
            .map(|i| (i + 1).min(self.playlists.len() - 1))
            .unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn prev(&mut self) {
        if self.playlists.is_empty() {
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
        if !self.playlists.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn bottom(&mut self) {
        if !self.playlists.is_empty() {
            self.state.select(Some(self.playlists.len() - 1));
        }
    }

    pub fn page_down(&mut self, step: usize) {
        if self.playlists.is_empty() {
            return;
        }
        let i = self
            .state
            .selected()
            .map(|i| (i + step).min(self.playlists.len() - 1))
            .unwrap_or(0);
        self.state.select(Some(i));
    }

    pub fn page_up(&mut self, step: usize) {
        if self.playlists.is_empty() {
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
            .playlists
            .iter()
            .map(|p| ListItem::new(Line::from(p.name.as_str())))
            .collect();

        let title = format!("Playlists ({})", self.playlists.len());
        let list = List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_stateful_widget(list, area, &mut self.state);
    }
}

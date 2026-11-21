use crate::config::Config;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct SettingsScreen;

impl SettingsScreen {
    pub fn render(f: &mut Frame, area: Rect, config: &Config) {
        let lines = vec![
            Line::from(Span::raw(format!(
                "music_directory : {}",
                config.general.music_directory
            ))),
            Line::from(Span::raw(format!(
                "volume          : {}%  [+/- to change]",
                config.audio.volume
            ))),
            Line::from(Span::raw(format!(
                "gapless         : {}",
                config.audio.gapless
            ))),
            Line::from(Span::raw(format!(
                "resume          : {}",
                config.playback.resume
            ))),
            Line::from(Span::raw(format!(
                "shuffle         : {}  [s to toggle]",
                config.playback.shuffle
            ))),
            Line::from(Span::raw(format!(
                "repeat          : {}  [r to cycle]",
                config.playback.repeat
            ))),
            Line::from(Span::raw("")),
            Line::from(Span::raw("Configuration files:")),
            Line::from(Span::raw("  Config      : ~/.config/wav/config.toml")),
            Line::from(Span::raw("  Keybindings : ~/.config/wav/keybindings.toml")),
            Line::from(Span::raw("  Database    : ~/.local/share/wav/library.db")),
            Line::from(Span::raw("")),
            Line::from(Span::raw(
                "Tip: Edit ~/.config/wav/config.toml to customize options.",
            )),
        ];

        let para =
            Paragraph::new(lines).block(Block::default().title("Settings").borders(Borders::ALL));
        f.render_widget(para, area);
    }
}

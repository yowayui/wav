use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    PlayPause,
    TogglePause,
    Stop,
    Next,
    Previous,
    SeekForward,
    SeekBackward,
    VolumeUp,
    VolumeDown,
    ToggleShuffle,
    ToggleRepeat,
    Search,
    Rescan,
    SwitchScreen(u8),
    Help,
    Quit,
    NavigateDown,
    NavigateUp,
    NavigateTop,
    NavigateBottom,
    PageUp,
    PageDown,
    Confirm,
    AddToPlaylist,
    Delete,
}

/// Maps key strings (e.g. "Space", "n") to actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings(HashMap<String, Action>);

impl Default for Keybindings {
    fn default() -> Self {
        let mut m = HashMap::new();
        m.insert("Space".into(), Action::PlayPause);
        m.insert("c".into(), Action::TogglePause);
        m.insert("x".into(), Action::Stop);
        m.insert("v".into(), Action::Stop);
        m.insert("n".into(), Action::Next);
        m.insert("p".into(), Action::Previous);
        m.insert("l".into(), Action::SeekForward);
        m.insert("h".into(), Action::SeekBackward);
        m.insert("+".into(), Action::VolumeUp);
        m.insert("-".into(), Action::VolumeDown);
        m.insert("s".into(), Action::ToggleShuffle);
        m.insert("r".into(), Action::ToggleRepeat);
        m.insert("R".into(), Action::Rescan);
        m.insert("F5".into(), Action::Rescan);
        m.insert("Ctrl-r".into(), Action::Rescan);
        m.insert("/".into(), Action::Search);
        m.insert("a".into(), Action::AddToPlaylist);
        m.insert("d".into(), Action::Delete);
        m.insert("1".into(), Action::SwitchScreen(1));
        m.insert("2".into(), Action::SwitchScreen(2));
        m.insert("3".into(), Action::SwitchScreen(3));
        m.insert("4".into(), Action::SwitchScreen(4));
        m.insert("?".into(), Action::Help);
        m.insert("q".into(), Action::Quit);
        m.insert("j".into(), Action::NavigateDown);
        m.insert("k".into(), Action::NavigateUp);
        m.insert("Down".into(), Action::NavigateDown);
        m.insert("Up".into(), Action::NavigateUp);
        m.insert("Left".into(), Action::SeekBackward);
        m.insert("Right".into(), Action::SeekForward);
        m.insert("g".into(), Action::NavigateTop);
        m.insert("G".into(), Action::NavigateBottom);
        m.insert("Home".into(), Action::NavigateTop);
        m.insert("End".into(), Action::NavigateBottom);
        m.insert("PageUp".into(), Action::PageUp);
        m.insert("PageDown".into(), Action::PageDown);
        m.insert("Enter".into(), Action::Confirm);
        Self(m)
    }
}

impl Keybindings {
    pub fn default_template() -> &'static str {
        r#"# WAV Keybindings Configuration (~/.config/wav/keybindings.toml)
# Format: "Key" = "Action"
#
# Available actions:
#   PlayPause, TogglePause, Stop, Next, Previous
#   SeekForward, SeekBackward, VolumeUp, VolumeDown
#   ToggleShuffle, ToggleRepeat, Rescan, Search
#   NavigateDown, NavigateUp, NavigateTop, NavigateBottom
#   PageUp, PageDown, SwitchScreen, Help, Quit, Confirm
#   AddToPlaylist, Delete

Space = "PlayPause"
c = "TogglePause"
x = "Stop"
v = "Stop"
Enter = "Confirm"
a = "AddToPlaylist"
d = "Delete"
n = "Next"
p = "Previous"
j = "NavigateDown"
k = "NavigateUp"
Down = "NavigateDown"
Up = "NavigateUp"
g = "NavigateTop"
G = "NavigateBottom"
Home = "NavigateTop"
End = "NavigateBottom"
PageDown = "PageDown"
PageUp = "PageUp"
l = "SeekForward"
h = "SeekBackward"
Right = "SeekForward"
Left = "SeekBackward"
"+" = "VolumeUp"
"-" = "VolumeDown"
s = "ToggleShuffle"
r = "ToggleRepeat"
R = "Rescan"
F5 = "Rescan"
"/" = "Search"
"?" = "Help"
q = "Quit"
"#
    }

    pub fn load(path: &std::path::Path) -> Self {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, Self::default_template());
            return Self::default();
        }
        if let Ok(text) = std::fs::read_to_string(path) {
            match toml::from_str::<HashMap<String, Action>>(&text) {
                Ok(custom) => {
                    let mut default = Self::default();
                    default.0.extend(custom);
                    return default;
                }
                Err(e) => {
                    tracing::warn!("failed to parse keybindings {}: {e}", path.display());
                }
            }
        }
        Self::default()
    }

    pub fn lookup(&self, event: &KeyEvent) -> Option<&Action> {
        let key_str = key_to_str(event);
        self.0.get(&key_str)
    }

    #[allow(dead_code)]
    pub fn inner(&self) -> &HashMap<String, Action> {
        &self.0
    }
}

fn key_to_str(event: &KeyEvent) -> String {
    let modifier_prefix = if event.modifiers.contains(KeyModifiers::CONTROL) {
        "Ctrl-"
    } else if event.modifiers.contains(KeyModifiers::ALT) {
        "Alt-"
    } else {
        ""
    };

    let key = match event.code {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::BackTab => "BackTab".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::Up => "Up".into(),
        KeyCode::Down => "Down".into(),
        KeyCode::Left => "Left".into(),
        KeyCode::Right => "Right".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PageUp".into(),
        KeyCode::PageDown => "PageDown".into(),
        KeyCode::F(n) => format!("F{n}"),
        _ => return String::new(),
    };
    format!("{}{}", modifier_prefix, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn default_keybindings_cover_required_keys() {
        let kb = Keybindings::default();
        assert_eq!(
            kb.lookup(&key(KeyCode::Char(' '))),
            Some(&Action::PlayPause)
        );
        assert_eq!(kb.lookup(&key(KeyCode::Char('q'))), Some(&Action::Quit));
        assert_eq!(
            kb.lookup(&key(KeyCode::Char('j'))),
            Some(&Action::NavigateDown)
        );
        assert_eq!(
            kb.lookup(&key(KeyCode::Char('k'))),
            Some(&Action::NavigateUp)
        );
        assert_eq!(kb.lookup(&key(KeyCode::Up)), Some(&Action::NavigateUp));
        assert_eq!(kb.lookup(&key(KeyCode::Down)), Some(&Action::NavigateDown));
        assert_eq!(kb.lookup(&key(KeyCode::Left)), Some(&Action::SeekBackward));
        assert_eq!(kb.lookup(&key(KeyCode::Right)), Some(&Action::SeekForward));
        assert_eq!(
            kb.lookup(&key(KeyCode::Char('1'))),
            Some(&Action::SwitchScreen(1))
        );
    }
}

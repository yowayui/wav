use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Split vertically: [top_height lines, fill, bottom_height lines]
pub fn vertical_split(area: Rect, top: u16, bottom: u16) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top),
            Constraint::Min(0),
            Constraint::Length(bottom),
        ])
        .split(area);
    (chunks[0], chunks[1], chunks[2])
}

/// Split horizontally into two halves by percentage.
#[allow(dead_code)]
pub fn horizontal_split_pct(area: Rect, left_pct: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(100 - left_pct),
        ])
        .split(area);
    (chunks[0], chunks[1])
}

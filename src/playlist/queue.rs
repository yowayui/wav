use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Queue {
    tracks: Vec<PathBuf>,
    current: Option<usize>,
}

impl Queue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, path: PathBuf) {
        self.tracks.push(path);
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current = None;
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn current(&self) -> Option<&PathBuf> {
        self.current.and_then(|i| self.tracks.get(i))
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    pub fn get(&self, idx: usize) -> Option<&PathBuf> {
        self.tracks.get(idx)
    }

    pub fn all(&self) -> &[PathBuf] {
        &self.tracks
    }

    /// Set current index directly (e.g. user clicks item in queue screen).
    pub fn set_index(&mut self, idx: usize) {
        if idx < self.tracks.len() {
            self.current = Some(idx);
        }
    }

    /// Find first occurrence of path and set as current.
    pub fn set_to_path(&mut self, path: &PathBuf) {
        if let Some(idx) = self.tracks.iter().position(|p| p == path) {
            self.current = Some(idx);
        }
    }

    /// Move to next track. Returns the new current path.
    pub fn advance(&mut self) -> Option<&PathBuf> {
        match self.current {
            None if !self.tracks.is_empty() => {
                self.current = Some(0);
            }
            Some(i) if i + 1 < self.tracks.len() => {
                self.current = Some(i + 1);
            }
            _ => return None,
        }
        self.current()
    }

    /// Move to previous track. Returns the new current path.
    pub fn retreat(&mut self) -> Option<&PathBuf> {
        match self.current {
            Some(i) if i > 0 => {
                self.current = Some(i - 1);
            }
            _ => return None,
        }
        self.current()
    }

    /// Reset to before-first position (for repeat-all).
    pub fn reset(&mut self) {
        self.current = None;
    }

    /// Fisher-Yates shuffle using xorshift, keeping current track at front.
    pub fn shuffle(&mut self) {
        let current_path = self.current().cloned();
        let n = self.tracks.len();
        if n <= 1 {
            return;
        }
        // xorshift32 — no rand dep needed
        let mut state: u32 = 0xdeadbeef;
        for i in (1..n).rev() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            let j = (state as usize) % (i + 1);
            self.tracks.swap(i, j);
        }
        // Re-anchor current
        if let Some(ref path) = current_path {
            self.current = self.tracks.iter().position(|p| p == path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn queue_advance_and_retreat() {
        let mut q = Queue::new();
        q.push(p("a.mp3"));
        q.push(p("b.mp3"));
        q.push(p("c.mp3"));

        assert!(q.current().is_none());
        assert_eq!(q.advance(), Some(&p("a.mp3")));
        assert_eq!(q.advance(), Some(&p("b.mp3")));
        assert_eq!(q.advance(), Some(&p("c.mp3")));
        assert_eq!(q.advance(), None);
        assert_eq!(q.retreat(), Some(&p("b.mp3")));
    }

    #[test]
    fn queue_clear_resets() {
        let mut q = Queue::new();
        q.push(p("a.mp3"));
        q.advance();
        q.clear();
        assert!(q.is_empty());
        assert!(q.current().is_none());
    }

    #[test]
    fn queue_set_index() {
        let mut q = Queue::new();
        q.push(p("a.mp3"));
        q.push(p("b.mp3"));
        q.push(p("c.mp3"));
        q.set_index(2);
        assert_eq!(q.current(), Some(&p("c.mp3")));
    }

    #[test]
    fn queue_set_to_path() {
        let mut q = Queue::new();
        q.push(p("a.mp3"));
        q.push(p("b.mp3"));
        q.set_to_path(&p("b.mp3"));
        assert_eq!(q.current_index(), Some(1));
    }

    #[test]
    fn queue_reset() {
        let mut q = Queue::new();
        q.push(p("a.mp3"));
        q.advance();
        q.reset();
        assert!(q.current().is_none());
        assert_eq!(q.advance(), Some(&p("a.mp3")));
    }

    #[test]
    fn queue_shuffle_keeps_all_tracks() {
        let mut q = Queue::new();
        for i in 0..10 {
            q.push(PathBuf::from(format!("{i}.mp3")));
        }
        q.advance();
        let before: std::collections::HashSet<_> = q.all().iter().cloned().collect();
        q.shuffle();
        let after: std::collections::HashSet<_> = q.all().iter().cloned().collect();
        assert_eq!(before, after);
        assert_eq!(q.len(), 10);
    }
}

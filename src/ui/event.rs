use crossbeam_channel::{bounded, Receiver, Sender};
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    AudioEvent(crate::audio::engine::EngineEvent),
    ScanComplete(usize), // number of tracks added/updated
}

pub struct EventLoop {
    pub rx: Receiver<Event>,
    _tx: Sender<Event>, // keep alive
}

impl EventLoop {
    pub fn new(tick_ms: u64) -> Self {
        let (tx, rx) = bounded::<Event>(256);
        let tx2 = tx.clone();

        thread::Builder::new()
            .name("wav-events".into())
            .spawn(move || {
                let tick = Duration::from_millis(tick_ms);
                loop {
                    if event::poll(tick).unwrap_or(false) {
                        match event::read() {
                            Ok(CrosstermEvent::Key(k)) => {
                                if k.kind != crossterm::event::KeyEventKind::Release {
                                    if tx2.send(Event::Key(k)).is_err() {
                                        break;
                                    }
                                }
                            }
                            Ok(CrosstermEvent::Mouse(m)) => {
                                let _ = tx2.send(Event::Mouse(m));
                            }
                            Ok(CrosstermEvent::Resize(_, _)) => {
                                // Terminal will re-render on next tick
                            }
                            _ => {}
                        }
                    }
                    if tx2.send(Event::Tick).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn event thread");

        Self { rx, _tx: tx }
    }
}

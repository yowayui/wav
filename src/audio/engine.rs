use crate::audio::{
    decoder::Decoder, mixer::adapt_channels, output::AudioOutput, resampler::StreamResampler,
};
use cpal::traits::HostTrait;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::PathBuf;
use std::thread;

#[derive(Debug)]
pub enum EngineCmd {
    Play(PathBuf),
    Pause,
    Resume,
    Stop,
    SeekRelative(f64),
    Volume(u8),
    Quit,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    Playing {
        path: PathBuf,
        duration_secs: Option<f64>,
    },
    Position {
        position_secs: f64,
        duration_secs: Option<f64>,
    },
    Paused,
    Resumed,
    Stopped,
    EndOfTrack,
    Error(String),
}

pub struct Engine {
    pub cmd_tx: Sender<EngineCmd>,
    pub event_rx: Receiver<EngineEvent>,
}

impl Engine {
    pub fn start(initial_volume: u8, gapless: bool) -> Self {
        let (cmd_tx, cmd_rx) = bounded::<EngineCmd>(32);
        let (event_tx, event_rx) = bounded::<EngineEvent>(64);

        thread::Builder::new()
            .name("wav-audio".into())
            .spawn(move || engine_loop(cmd_rx, event_tx, initial_volume, gapless))
            .expect("spawn audio thread");

        Self { cmd_tx, event_rx }
    }

    pub fn play(&self, path: PathBuf) {
        let _ = self.cmd_tx.send(EngineCmd::Play(path));
    }
    pub fn pause(&self) {
        let _ = self.cmd_tx.send(EngineCmd::Pause);
    }
    pub fn resume(&self) {
        let _ = self.cmd_tx.send(EngineCmd::Resume);
    }
    pub fn stop(&self) {
        let _ = self.cmd_tx.send(EngineCmd::Stop);
    }
    pub fn seek(&self, delta: f64) {
        let _ = self.cmd_tx.send(EngineCmd::SeekRelative(delta));
    }
    pub fn set_volume(&self, percent: u8) {
        let _ = self.cmd_tx.send(EngineCmd::Volume(percent));
    }
    pub fn quit(&self) {
        let _ = self.cmd_tx.send(EngineCmd::Quit);
    }
}

fn engine_loop(
    cmd_rx: Receiver<EngineCmd>,
    event_tx: Sender<EngineEvent>,
    initial_volume: u8,
    gapless: bool,
) {
    let device = match cpal::default_host().default_output_device() {
        Some(d) => d,
        None => {
            let _ = event_tx.send(EngineEvent::Error("no audio output device".into()));
            return;
        }
    };

    let mut output: Option<AudioOutput> = None;
    let mut volume = initial_volume;

    loop {
        let cmd = match cmd_rx.recv() {
            Ok(c) => c,
            Err(_) => return,
        };

        match cmd {
            EngineCmd::Quit => return,
            EngineCmd::Volume(v) => {
                volume = v;
                if let Some(ref o) = output {
                    o.set_volume(v);
                }
            }
            EngineCmd::Pause => {
                if let Some(ref o) = output {
                    o.set_paused(true);
                }
                let _ = event_tx.send(EngineEvent::Paused);
            }
            EngineCmd::Resume => {
                if let Some(ref o) = output {
                    o.set_paused(false);
                }
                let _ = event_tx.send(EngineEvent::Resumed);
            }
            EngineCmd::Stop => {
                if let Some(ref o) = output {
                    o.set_paused(true);
                    o.reset_buffer();
                }
                let _ = event_tx.send(EngineEvent::Stopped);
            }
            EngineCmd::SeekRelative(_) => {}

            EngineCmd::Play(mut current_path) => {
                if output.is_none() {
                    match AudioOutput::open(&device, volume) {
                        Ok(o) => output = Some(o),
                        Err(e) => {
                            let _ = event_tx.send(EngineEvent::Error(e.to_string()));
                            continue;
                        }
                    }
                }

                let out = output.as_mut().unwrap();
                let (sample_rate, channels) = (out.sample_rate, out.channels);

                loop {
                    out.reset_buffer();
                    out.set_paused(false);

                    let result = play_one(
                        &current_path,
                        out,
                        sample_rate,
                        channels,
                        &cmd_rx,
                        &event_tx,
                        &mut volume,
                        gapless,
                    );

                    match result {
                        PlayResult::Quit => return,
                        PlayResult::Stop | PlayResult::Done => break,
                        PlayResult::Next(new_path) => {
                            current_path = new_path;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum PlayResult {
    Done,
    Stop,
    Quit,
    Next(PathBuf),
}

fn handle_engine_cmd(
    cmd: EngineCmd,
    out: &mut AudioOutput,
    decoder: &mut Decoder,
    approx_pos: &mut f64,
    volume: &mut u8,
    cmd_rx: &Receiver<EngineCmd>,
    event_tx: &Sender<EngineEvent>,
) -> Option<PlayResult> {
    match cmd {
        EngineCmd::Quit => Some(PlayResult::Quit),
        EngineCmd::Stop => {
            out.set_paused(true);
            out.reset_buffer();
            let _ = event_tx.send(EngineEvent::Stopped);
            Some(PlayResult::Stop)
        }
        EngineCmd::Play(new_path) => Some(PlayResult::Next(new_path)),
        EngineCmd::Pause => {
            out.set_paused(true);
            let _ = event_tx.send(EngineEvent::Paused);
            loop {
                match cmd_rx.recv() {
                    Ok(EngineCmd::Resume) => {
                        out.set_paused(false);
                        let _ = event_tx.send(EngineEvent::Resumed);
                        return None;
                    }
                    Ok(EngineCmd::Stop) => {
                        out.set_paused(true);
                        out.reset_buffer();
                        let _ = event_tx.send(EngineEvent::Stopped);
                        return Some(PlayResult::Stop);
                    }
                    Ok(EngineCmd::Quit) => return Some(PlayResult::Quit),
                    Ok(EngineCmd::Play(p)) => return Some(PlayResult::Next(p)),
                    Ok(EngineCmd::Volume(v)) => {
                        *volume = v;
                        out.set_volume(v);
                    }
                    Ok(EngineCmd::SeekRelative(s)) => {
                        let t = (*approx_pos + s).max(0.0);
                        let _ = decoder.seek(t);
                        *approx_pos = t;
                        out.reset_buffer();
                        let _ = event_tx.send(EngineEvent::Position {
                            position_secs: *approx_pos,
                            duration_secs: decoder.info.duration_secs,
                        });
                    }
                    Ok(EngineCmd::Pause) => {} // already paused
                    Err(_) => return Some(PlayResult::Quit),
                }
            }
        }
        EngineCmd::Resume => {
            out.set_paused(false);
            let _ = event_tx.send(EngineEvent::Resumed);
            None
        }
        EngineCmd::Volume(v) => {
            *volume = v;
            out.set_volume(v);
            None
        }
        EngineCmd::SeekRelative(s) => {
            let t = (*approx_pos + s).max(0.0);
            let _ = decoder.seek(t);
            *approx_pos = t;
            out.reset_buffer();
            let _ = event_tx.send(EngineEvent::Position {
                position_secs: *approx_pos,
                duration_secs: decoder.info.duration_secs,
            });
            None
        }
    }
}

fn play_one(
    path: &PathBuf,
    out: &mut AudioOutput,
    dst_rate: u32,
    dst_ch: usize,
    cmd_rx: &Receiver<EngineCmd>,
    event_tx: &Sender<EngineEvent>,
    volume: &mut u8,
    gapless: bool,
) -> PlayResult {
    let mut decoder = match Decoder::open(path, gapless) {
        Ok(d) => d,
        Err(e) => {
            let _ = event_tx.send(EngineEvent::Error(e.to_string()));
            return PlayResult::Stop;
        }
    };

    let src_rate = decoder.info.sample_rate;
    let src_ch = decoder.info.channels as usize;

    let mut resampler = if src_rate != dst_rate {
        match StreamResampler::new(src_rate, dst_rate, src_ch) {
            Ok(r) => Some(r),
            Err(e) => {
                let _ = event_tx.send(EngineEvent::Error(e.to_string()));
                return PlayResult::Stop;
            }
        }
    } else {
        None
    };

    let duration_secs = decoder.info.duration_secs;
    let _ = event_tx.send(EngineEvent::Playing {
        path: path.clone(),
        duration_secs,
    });
    let mut approx_pos: f64 = 0.0;
    let mut last_report = std::time::Instant::now();

    loop {
        // Poll commands before next packet
        while let Ok(cmd) = cmd_rx.try_recv() {
            if let Some(res) = handle_engine_cmd(
                cmd,
                out,
                &mut decoder,
                &mut approx_pos,
                volume,
                cmd_rx,
                event_tx,
            ) {
                return res;
            }
        }

        // Decode next packet
        match decoder.next_packet_f32() {
            Ok(Some(mut samples)) => {
                approx_pos += samples.len() as f64 / (src_rate as f64 * src_ch as f64);

                if src_ch != dst_ch {
                    samples = adapt_channels(samples, src_ch, dst_ch);
                }
                if let Some(ref mut rs) = resampler {
                    match rs.process(&samples) {
                        Ok(r) => samples = r,
                        Err(e) => tracing::warn!("resample: {e}"),
                    }
                }

                // Push samples into ring buffer
                let mut offset = 0;
                while offset < samples.len() {
                    let pushed = out.producer.push_slice(&samples[offset..]);
                    offset += pushed;
                    if offset < samples.len() {
                        // Buffer full — yield 2ms and check commands!
                        std::thread::sleep(std::time::Duration::from_millis(2));
                        while let Ok(cmd) = cmd_rx.try_recv() {
                            if let Some(res) = handle_engine_cmd(
                                cmd,
                                out,
                                &mut decoder,
                                &mut approx_pos,
                                volume,
                                cmd_rx,
                                event_tx,
                            ) {
                                return res;
                            }
                        }
                    }
                }

                if last_report.elapsed() >= std::time::Duration::from_millis(200) {
                    let _ = event_tx.send(EngineEvent::Position {
                        position_secs: approx_pos,
                        duration_secs,
                    });
                    last_report = std::time::Instant::now();
                }
            }
            Ok(None) => {
                // End of track — flush resampler
                if let Some(ref mut rs) = resampler {
                    if let Ok(tail) = rs.flush() {
                        let mut offset = 0;
                        while offset < tail.len() {
                            let pushed = out.producer.push_slice(&tail[offset..]);
                            offset += pushed;
                            if offset < tail.len() {
                                std::thread::sleep(std::time::Duration::from_millis(2));
                                while let Ok(cmd) = cmd_rx.try_recv() {
                                    if let Some(res) = handle_engine_cmd(
                                        cmd,
                                        out,
                                        &mut decoder,
                                        &mut approx_pos,
                                        volume,
                                        cmd_rx,
                                        event_tx,
                                    ) {
                                        return res;
                                    }
                                }
                            }
                        }
                    }
                }
                let _ = event_tx.send(EngineEvent::EndOfTrack);
                return PlayResult::Done;
            }
            Err(e) => {
                let _ = event_tx.send(EngineEvent::Error(e.to_string()));
                return PlayResult::Stop;
            }
        }
    }
}

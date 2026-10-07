use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::Client;
use rodio::decoder::DecoderBuilder;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde::Serialize;

use super::stream::{self, MediaSource, StreamBuffer, StreamReader};
use crate::error::{Error, Result};

const TICK: Duration = Duration::from_millis(50);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Enough data for the decoder to probe the container and play the first seconds smoothly.
const PREBUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PlayerEvent {
    Playing { position_ms: u64 },
    Paused { position_ms: u64 },
    Progress { position_ms: u64 },
    Ended,
    Error { message: String },
}

pub struct LoadRequest {
    pub source: MediaSource,
    /// Container hint for the decoder, e.g. `mp3` or `m4a`.
    pub format_hint: &'static str,
    /// Loudness normalization factor (1.0 = unchanged).
    pub gain: f32,
}

enum Command {
    Start {
        generation: u64,
        decoder: Decoder<StreamReader>,
        gain: f32,
    },
    Pause,
    Resume,
    Seek(Duration),
    Volume(f32),
    Stop,
}

pub struct AudioEngine {
    commands: Sender<Command>,
    generation: Arc<AtomicU64>,
    current: Mutex<Option<Arc<StreamBuffer>>>,
    http: Client,
}

impl AudioEngine {
    pub fn spawn(http: Client, emit: impl Fn(PlayerEvent) + Send + 'static) -> Self {
        let (commands, receiver) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let latest = generation.clone();

        thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || EngineThread::new(emit, latest).run(receiver))
            .expect("failed to spawn audio thread");

        Self {
            commands,
            generation,
            current: Mutex::new(None),
            http,
        }
    }

    /// Starts downloading and decoding a new track, replacing the current one.
    pub async fn load(&self, request: LoadRequest) -> Result<()> {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let buffer = StreamBuffer::new();
        if let Some(previous) = self.lock_current().replace(buffer.clone()) {
            previous.cancel();
        }
        self.send(Command::Stop);

        tauri::async_runtime::spawn(stream::download(
            self.http.clone(),
            request.source,
            buffer.clone(),
        ));

        let hint = request.format_hint;
        let decoder = tauri::async_runtime::spawn_blocking(move || build_decoder(buffer, hint))
            .await
            .map_err(|e| Error::Decode(e.to_string()))??;

        self.send(Command::Start {
            generation,
            decoder,
            gain: request.gain,
        });
        Ok(())
    }

    pub fn pause(&self) {
        self.send(Command::Pause);
    }

    pub fn resume(&self) {
        self.send(Command::Resume);
    }

    pub fn seek(&self, position: Duration) {
        self.send(Command::Seek(position));
    }

    pub fn set_volume(&self, volume: f32) {
        self.send(Command::Volume(volume.clamp(0.0, 1.0)));
    }

    pub fn stop(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Some(previous) = self.lock_current().take() {
            previous.cancel();
        }
        self.send(Command::Stop);
    }

    fn lock_current(&self) -> std::sync::MutexGuard<'_, Option<Arc<StreamBuffer>>> {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn send(&self, command: Command) {
        // The engine thread only exits when the engine itself is dropped.
        let _ = self.commands.send(command);
    }
}

fn build_decoder(buffer: Arc<StreamBuffer>, hint: &str) -> Result<Decoder<StreamReader>> {
    buffer.wait_for(PREBUFFER_BYTES).map_err(Error::Unavailable)?;
    if buffer.is_cancelled() {
        return Err(Error::Unavailable("cancelled".into()));
    }

    let mut builder = DecoderBuilder::new()
        .with_data(StreamReader::new(buffer.clone()))
        .with_hint(hint)
        .with_seekable(true)
        .with_gapless(true);
    if let Some(len) = buffer.total_len() {
        builder = builder.with_byte_len(len);
    }
    builder.build().map_err(|e| Error::Decode(e.to_string()))
}

struct EngineThread<F> {
    emit: F,
    latest_generation: Arc<AtomicU64>,
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
    volume: f32,
    gain: f32,
    last_progress: Instant,
}

impl<F: Fn(PlayerEvent)> EngineThread<F> {
    fn new(emit: F, latest_generation: Arc<AtomicU64>) -> Self {
        Self {
            emit,
            latest_generation,
            sink: None,
            player: None,
            volume: 1.0,
            gain: 1.0,
            last_progress: Instant::now(),
        }
    }

    fn run(mut self, receiver: mpsc::Receiver<Command>) {
        loop {
            match receiver.recv_timeout(TICK) {
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.tick();
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Start {
                generation,
                decoder,
                gain,
            } => {
                if generation == self.latest_generation.load(Ordering::Acquire) {
                    self.start(decoder, gain);
                }
            }
            Command::Pause => {
                if let Some(player) = &self.player {
                    player.pause();
                    (self.emit)(PlayerEvent::Paused {
                        position_ms: position_ms(player),
                    });
                }
            }
            Command::Resume => {
                if let Some(player) = &self.player {
                    player.play();
                    (self.emit)(PlayerEvent::Playing {
                        position_ms: position_ms(player),
                    });
                }
            }
            Command::Seek(position) => {
                if let Some(player) = &self.player {
                    if let Err(err) = player.try_seek(position) {
                        (self.emit)(PlayerEvent::Error {
                            message: err.to_string(),
                        });
                    }
                    (self.emit)(PlayerEvent::Progress {
                        position_ms: position_ms(player),
                    });
                }
            }
            Command::Volume(volume) => {
                self.volume = volume;
                self.apply_volume();
            }
            Command::Stop => self.player = None,
        }
    }

    fn start(&mut self, decoder: Decoder<StreamReader>, gain: f32) {
        if self.sink.is_none() {
            match DeviceSinkBuilder::open_default_sink() {
                Ok(mut sink) => {
                    sink.log_on_drop(false);
                    self.sink = Some(sink);
                }
                Err(err) => {
                    (self.emit)(PlayerEvent::Error {
                        message: Error::Output(err.to_string()).to_string(),
                    });
                    return;
                }
            }
        }
        let Some(sink) = &self.sink else { return };

        let player = Player::connect_new(sink.mixer());
        player.append(decoder);
        self.player = Some(player);
        self.gain = gain;
        self.apply_volume();
        self.last_progress = Instant::now();
        (self.emit)(PlayerEvent::Playing { position_ms: 0 });
    }

    fn apply_volume(&self) {
        if let Some(player) = &self.player {
            // Squared slider value tracks perceived loudness far better than a linear one.
            player.set_volume(self.volume * self.volume * self.gain);
        }
    }

    fn tick(&mut self) {
        let Some(player) = &self.player else { return };

        if player.empty() {
            self.player = None;
            (self.emit)(PlayerEvent::Ended);
            return;
        }
        if !player.is_paused() && self.last_progress.elapsed() >= PROGRESS_INTERVAL {
            self.last_progress = Instant::now();
            (self.emit)(PlayerEvent::Progress {
                position_ms: position_ms(player),
            });
        }
    }
}

fn position_ms(player: &Player) -> u64 {
    player.get_pos().as_millis() as u64
}

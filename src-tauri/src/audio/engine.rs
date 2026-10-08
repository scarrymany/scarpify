use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::Client;
use rodio::decoder::DecoderBuilder;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde::Serialize;
use tokio::sync::oneshot;

use super::devices;
use super::stream::{self, MediaSource, StreamBuffer, StreamReader};
use crate::error::{Error, Result};

const TICK: Duration = Duration::from_millis(50);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
/// Enough data for the decoder to probe the container and play the first seconds smoothly.
const PREBUFFER_BYTES: usize = 64 * 1024;
/// How often the system default output (headphones plugged in, etc.) is checked.
const DEVICE_WATCH_INTERVAL: Duration = Duration::from_secs(2);

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

/// Where playback stood when the output device was swapped.
struct Resume {
    position: Duration,
    paused: bool,
}

enum Command {
    Start {
        generation: u64,
        decoder: Decoder<StreamReader>,
        gain: f32,
        start_at: Duration,
        paused: bool,
    },
    Pause,
    Resume,
    Seek(Duration),
    Volume(f32),
    Stop,
    /// Closes the current output and remembers the device to open next.
    SwitchDevice {
        device: Option<String>,
        reply: oneshot::Sender<Option<Resume>>,
    },
}

/// The track being played: kept so it can be re-decoded from memory on a device switch.
#[derive(Clone)]
struct Current {
    buffer: Arc<StreamBuffer>,
    hint: &'static str,
    gain: f32,
}

pub struct AudioEngine {
    commands: Sender<Command>,
    generation: Arc<AtomicU64>,
    current: Mutex<Option<Current>>,
    /// Output chosen in settings; `None` follows the system default.
    preferred_device: Mutex<Option<String>>,
    /// Device the engine thread actually opened.
    active_device: Arc<Mutex<Option<String>>>,
    http: Client,
}

impl AudioEngine {
    pub fn spawn(http: Client, emit: impl Fn(PlayerEvent) + Send + 'static) -> Arc<Self> {
        let (commands, receiver) = mpsc::channel();
        let generation = Arc::new(AtomicU64::new(0));
        let active_device = Arc::new(Mutex::new(None));

        let thread = EngineThread::new(emit, generation.clone(), active_device.clone());
        thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || thread.run(receiver))
            .expect("failed to spawn audio thread");

        let engine = Arc::new(Self {
            commands,
            generation,
            current: Mutex::new(None),
            preferred_device: Mutex::new(None),
            active_device,
            http,
        });
        tauri::async_runtime::spawn(watch_devices(Arc::downgrade(&engine)));
        engine
    }

    /// Starts downloading and decoding a new track, replacing the current one.
    pub async fn load(&self, request: LoadRequest) -> Result<()> {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let current = Current {
            buffer: StreamBuffer::new(),
            hint: request.format_hint,
            gain: request.gain,
        };
        if let Some(previous) = lock(&self.current).replace(current.clone()) {
            previous.buffer.cancel();
        }
        self.send(Command::Stop);

        tauri::async_runtime::spawn(stream::download(
            self.http.clone(),
            request.source,
            current.buffer.clone(),
        ));

        let decoder = decode(&current).await?;
        self.send(Command::Start {
            generation,
            decoder,
            gain: current.gain,
            start_at: Duration::ZERO,
            paused: false,
        });
        Ok(())
    }

    /// Moves playback to another output without losing the position.
    pub async fn set_output_device(&self, device: Option<String>) -> Result<()> {
        *lock(&self.preferred_device) = device;
        self.reopen_output().await
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
        if let Some(previous) = lock(&self.current).take() {
            previous.buffer.cancel();
        }
        self.send(Command::Stop);
    }

    async fn reopen_output(&self) -> Result<()> {
        let generation = self.generation.load(Ordering::Acquire);
        let (reply, answer) = oneshot::channel();
        self.send(Command::SwitchDevice {
            device: lock(&self.preferred_device).clone(),
            reply,
        });
        let resume = answer.await.ok().flatten();

        // The track is already in memory, so it is decoded again instead of re-downloaded.
        let current = lock(&self.current).clone();
        if let (Some(resume), Some(current)) = (resume, current) {
            let decoder = decode(&current).await?;
            self.send(Command::Start {
                generation,
                decoder,
                gain: current.gain,
                start_at: resume.position,
                paused: resume.paused,
            });
        }
        Ok(())
    }

    /// Whether the open output no longer matches what the settings ask for.
    fn output_is_stale(&self) -> bool {
        let Some(active) = lock(&self.active_device).clone() else { return false };
        match lock(&self.preferred_device).clone() {
            None => devices::default_id().is_some_and(|default| default != active),
            Some(preferred) if preferred == active => devices::find(&preferred).is_none(),
            // Fell back to the default earlier; switch back once the chosen device returns.
            Some(preferred) => devices::find(&preferred).is_some(),
        }
    }

    fn send(&self, command: Command) {
        // The engine thread only exits when the engine itself is dropped.
        let _ = self.commands.send(command);
    }
}

async fn watch_devices(engine: Weak<AudioEngine>) {
    loop {
        tokio::time::sleep(DEVICE_WATCH_INTERVAL).await;
        let Some(engine) = engine.upgrade() else { return };
        if engine.output_is_stale() {
            let _ = engine.reopen_output().await;
        }
    }
}

async fn decode(current: &Current) -> Result<Decoder<StreamReader>> {
    let buffer = current.buffer.clone();
    let hint = current.hint;
    tauri::async_runtime::spawn_blocking(move || build_decoder(buffer, hint))
        .await
        .map_err(|e| Error::Decode(e.to_string()))?
}

fn build_decoder(buffer: Arc<StreamBuffer>, hint: &str) -> Result<Decoder<StreamReader>> {
    buffer.wait_for(PREBUFFER_BYTES).map_err(Error::Unavailable)?;
    if buffer.is_cancelled() {
        return Err(Error::Cancelled);
    }

    let mut builder = DecoderBuilder::new()
        .with_data(StreamReader::new(buffer.clone()))
        .with_hint(hint)
        .with_seekable(true)
        .with_gapless(true);
    if let Some(len) = buffer.total_len() {
        builder = builder.with_byte_len(len);
    }
    builder.build().map_err(|e| {
        // A reader cut off by cancellation fails to probe; that is not a broken stream.
        if buffer.is_cancelled() {
            Error::Cancelled
        } else {
            Error::Decode(e.to_string())
        }
    })
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EngineThread<F> {
    emit: F,
    latest_generation: Arc<AtomicU64>,
    active_device: Arc<Mutex<Option<String>>>,
    device: Option<String>,
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
    volume: f32,
    gain: f32,
    last_progress: Instant,
}

impl<F: Fn(PlayerEvent)> EngineThread<F> {
    fn new(emit: F, latest_generation: Arc<AtomicU64>, active_device: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            emit,
            latest_generation,
            active_device,
            device: None,
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
                start_at,
                paused,
            } => {
                if generation == self.latest_generation.load(Ordering::Acquire) {
                    self.start(decoder, gain, start_at, paused);
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
            Command::SwitchDevice { device, reply } => {
                let resume = self.player.take().map(|player| Resume {
                    position: player.get_pos(),
                    paused: player.is_paused(),
                });
                self.sink = None;
                *lock(&self.active_device) = None;
                self.device = device;
                let _ = reply.send(resume);
            }
        }
    }

    fn start(&mut self, decoder: Decoder<StreamReader>, gain: f32, start_at: Duration, paused: bool) {
        if self.sink.is_none() {
            match self.open_sink() {
                Ok(sink) => self.sink = Some(sink),
                Err(message) => {
                    (self.emit)(PlayerEvent::Error {
                        message: Error::Output(message).to_string(),
                    });
                    return;
                }
            }
        }
        let Some(sink) = &self.sink else { return };

        let player = Player::connect_new(sink.mixer());
        player.append(decoder);
        if paused {
            player.pause();
        }
        if !start_at.is_zero() {
            let _ = player.try_seek(start_at);
        }
        let position_ms = start_at.as_millis() as u64;
        self.player = Some(player);
        self.gain = gain;
        self.apply_volume();
        self.last_progress = Instant::now();
        (self.emit)(if paused {
            PlayerEvent::Paused { position_ms }
        } else {
            PlayerEvent::Playing { position_ms }
        });
    }

    /// Opens the chosen device, falling back to the system default when it is gone.
    fn open_sink(&self) -> std::result::Result<MixerDeviceSink, String> {
        let chosen = self.device.as_deref().and_then(devices::find);
        let (mut sink, id) = match chosen.and_then(|device| {
            let id = devices::device_id(&device);
            DeviceSinkBuilder::from_device(device)
                .and_then(|builder| builder.open_stream())
                .ok()
                .map(|sink| (sink, id))
        }) {
            Some(opened) => opened,
            None => (
                DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?,
                devices::default_id(),
            ),
        };
        sink.log_on_drop(false);
        *lock(&self.active_device) = id;
        Ok(sink)
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

#[cfg(test)]
mod tests {
    use super::PlayerEvent;

    /// The UI reads `positionMs`; a snake_case field silently froze the progress bar.
    #[test]
    fn events_serialize_in_camel_case() {
        let event = serde_json::to_value(PlayerEvent::Progress { position_ms: 1500 }).unwrap();
        assert_eq!(event, serde_json::json!({ "kind": "progress", "positionMs": 1500 }));

        let ended = serde_json::to_value(PlayerEvent::Ended).unwrap();
        assert_eq!(ended, serde_json::json!({ "kind": "ended" }));
    }
}

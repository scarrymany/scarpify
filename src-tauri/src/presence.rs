//! Discord Rich Presence: "Listening to SCARPIFY" with cover, title, artist and progress bar.
//!
//! Discord runs locally and may start, quit or restart at any time, and it rate-limits
//! activity updates. A dedicated thread therefore owns the IPC connection, coalesces bursts
//! (track changes, seeks) into the latest state and reconnects lazily.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::activity::{Activity, ActivityType, Assets, StatusDisplayType, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::Deserialize;

/// Application registered in the Discord Developer Portal; its name and icon are what
/// Discord shows as "Listening to SCARPIFY". Forks can point to their own application
/// with `SCARPIFY_DISCORD_APP_ID` at build time.
const DISCORD_APP_ID: &str = match option_env!("SCARPIFY_DISCORD_APP_ID") {
    Some(id) => id,
    None => "1557570425213423616",
};
/// Discord accepts five activity updates per 20 seconds.
const MIN_UPDATE_INTERVAL: Duration = Duration::from_secs(4);
const RECONNECT_INTERVAL: Duration = Duration::from_secs(15);
/// The `scarpify` art asset of the Discord application, referenced by its CDN URL: Discord
/// renders image URLs immediately, while asset keys can take a long time to propagate.
/// Drawn as a badge in the corner of the cover.
const BADGE_IMAGE: &str = "https://cdn.discordapp.com/app-assets/1557570425213423616/1557581447848525865.png";
const BADGE_TEXT: &str = "SCARPIFY";
/// Discord rejects activity strings outside 2..=128 characters.
const MAX_TEXT_CHARS: usize = 128;
const MIN_TEXT_CHARS: usize = 2;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub artwork: Option<String>,
    pub url: Option<String>,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
}

enum Command {
    Show(NowPlaying),
    Hide,
    Enable(bool),
}

pub struct Presence {
    commands: Sender<Command>,
}

impl Presence {
    pub fn spawn() -> Self {
        let (commands, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("discord-presence".into())
            .spawn(move || Worker::new(DISCORD_APP_ID).run(receiver))
            .expect("failed to spawn presence thread");
        Self { commands }
    }

    pub fn show(&self, now_playing: NowPlaying) {
        let _ = self.commands.send(Command::Show(now_playing));
    }

    pub fn hide(&self) {
        let _ = self.commands.send(Command::Hide);
    }

    pub fn set_enabled(&self, enabled: bool) {
        let _ = self.commands.send(Command::Enable(enabled));
    }
}

/// What Discord should display once the rate limit and connection allow it.
#[derive(Clone, PartialEq)]
enum Desired {
    Nothing,
    Track { now_playing: NowPlaying, started_at_ms: i64 },
}

struct Worker {
    app_id: &'static str,
    client: Option<DiscordIpcClient>,
    enabled: bool,
    desired: Desired,
    /// `None` until something was sent on the current connection.
    shown: Option<Desired>,
    last_sent: Option<Instant>,
    last_connect_attempt: Option<Instant>,
}

impl Worker {
    fn new(app_id: &'static str) -> Self {
        Self {
            app_id,
            client: None,
            enabled: true,
            desired: Desired::Nothing,
            shown: None,
            last_sent: None,
            last_connect_attempt: None,
        }
    }

    fn run(mut self, receiver: Receiver<Command>) {
        loop {
            match receiver.recv_timeout(self.wait_time()) {
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            self.sync();
        }
        self.disconnect();
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Show(now_playing) => {
                let started_at_ms = unix_ms() - now_playing.position_ms as i64;
                self.desired = Desired::Track { now_playing, started_at_ms };
            }
            Command::Hide => self.desired = Desired::Nothing,
            Command::Enable(enabled) => {
                self.enabled = enabled;
                if !enabled {
                    self.disconnect();
                }
            }
        }
    }

    /// Sleeps until the next send is allowed when an update is pending, otherwise idles.
    fn wait_time(&self) -> Duration {
        if self.is_synced() {
            return Duration::from_secs(3600);
        }
        let since_send = self.last_sent.map_or(MIN_UPDATE_INTERVAL, |at| at.elapsed());
        MIN_UPDATE_INTERVAL.saturating_sub(since_send).max(Duration::from_millis(50))
    }

    fn is_synced(&self) -> bool {
        !self.enabled || self.shown.as_ref() == Some(&self.desired)
    }

    fn sync(&mut self) {
        if self.is_synced() {
            return;
        }
        if self.last_sent.is_some_and(|at| at.elapsed() < MIN_UPDATE_INTERVAL) {
            return;
        }
        if self.client.is_none() && !self.connect() {
            return;
        }
        let Some(client) = self.client.as_mut() else { return };

        let result = match &self.desired {
            Desired::Nothing => client.clear_activity(),
            Desired::Track { now_playing, started_at_ms } => {
                client.set_activity(activity(now_playing, *started_at_ms))
            }
        };
        self.last_sent = Some(Instant::now());
        match result {
            Ok(()) => self.shown = Some(self.desired.clone()),
            // Discord was closed; the next sync reconnects and resends the current state.
            Err(_) => self.disconnect(),
        }
    }

    fn connect(&mut self) -> bool {
        if self.last_connect_attempt.is_some_and(|at| at.elapsed() < RECONNECT_INTERVAL) {
            return false;
        }
        self.last_connect_attempt = Some(Instant::now());

        let mut client = DiscordIpcClient::new(self.app_id);
        if client.connect().is_err() {
            return false;
        }
        self.client = Some(client);
        self.shown = None;
        true
    }

    fn disconnect(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
        self.shown = None;
    }
}

fn activity(now_playing: &NowPlaying, started_at_ms: i64) -> Activity<'_> {
    let mut timestamps = Timestamps::new().start(started_at_ms);
    if let Some(duration) = now_playing.duration_ms {
        timestamps = timestamps.end(started_at_ms + duration as i64);
    }

    let mut assets = Assets::new();
    if let Some(artwork) = now_playing.artwork.as_deref().filter(|a| a.starts_with("https://")) {
        // The badge needs a large image to sit on; without a cover Discord shows the app icon.
        assets = assets
            .large_image(artwork)
            .small_image(BADGE_IMAGE)
            .small_text(BADGE_TEXT);
        if let Some(album) = &now_playing.album {
            assets = assets.large_text(discord_text(album));
        }
    }

    let mut activity = Activity::new()
        .activity_type(ActivityType::Listening)
        .status_display_type(StatusDisplayType::Name)
        .details(discord_text(&now_playing.title))
        .state(discord_text(&now_playing.artists.join(", ")))
        .timestamps(timestamps)
        .assets(assets);
    if let Some(url) = now_playing.url.as_deref().filter(|u| u.starts_with("https://")) {
        activity = activity.details_url(url);
    }
    activity
}

/// Fits text into Discord's accepted length, padding one-character titles.
fn discord_text(text: &str) -> String {
    let mut text: String = text.trim().chars().take(MAX_TEXT_CHARS).collect();
    while text.chars().count() < MIN_TEXT_CHARS {
        text.push(' ');
    }
    text
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now_playing() -> NowPlaying {
        serde_json::from_value(serde_json::json!({
            "title": "Get Lucky",
            "artists": ["Daft Punk", "Pharrell Williams"],
            "album": "Random Access Memories",
            "artwork": "https://i1.sndcdn.com/artworks-x-t500x500.jpg",
            "url": "https://soundcloud.com/daftpunk/get-lucky",
            "positionMs": 22_000,
            "durationMs": 132_000
        }))
        .unwrap()
    }

    #[test]
    fn builds_a_listening_activity_with_progress() {
        let started = 1_000_000;
        let json = serde_json::to_value(activity(&now_playing(), started)).unwrap();

        assert_eq!(json["type"], 2);
        assert_eq!(json["status_display_type"], 0);
        assert_eq!(json["details"], "Get Lucky");
        assert_eq!(json["state"], "Daft Punk, Pharrell Williams");
        assert_eq!(json["timestamps"]["start"], started);
        assert_eq!(json["timestamps"]["end"], started + 132_000);
        assert_eq!(json["assets"]["large_image"], "https://i1.sndcdn.com/artworks-x-t500x500.jpg");
        assert_eq!(json["assets"]["small_image"], BADGE_IMAGE);
        assert_eq!(json["assets"]["small_text"], BADGE_TEXT);
        assert_eq!(json["details_url"], "https://soundcloud.com/daftpunk/get-lucky");
    }

    #[test]
    fn fits_text_into_discord_limits() {
        assert_eq!(discord_text("A"), "A ");
        assert_eq!(discord_text("  Song  "), "Song");
        assert_eq!(discord_text(&"x".repeat(300)).chars().count(), MAX_TEXT_CHARS);
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    #[test]
    #[ignore = "requires a running Discord client"]
    fn shows_activity_in_discord() {
        let mut client = DiscordIpcClient::new(DISCORD_APP_ID);
        client.connect().expect("Discord IPC is not reachable");
        let now_playing = NowPlaying {
            title: "Get Lucky".into(),
            artists: vec!["Daft Punk".into(), "Pharrell Williams".into()],
            album: Some("Random Access Memories".into()),
            artwork: Some("https://i1.sndcdn.com/artworks-000045765973-c2x8ko-t500x500.jpg".into()),
            url: Some("https://soundcloud.com/daftpunk".into()),
            position_ms: 22_000,
            duration_ms: Some(248_000),
        };
        client
            .set_activity(activity(&now_playing, unix_ms() - 22_000))
            .expect("Discord rejected the activity");
        println!("activity set, visible for 20 s");
        std::thread::sleep(Duration::from_secs(20));
        client.clear_activity().unwrap();
        client.close().unwrap();
    }
}

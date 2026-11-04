//! The Windows media session (System Media Transport Controls).
//!
//! Windows shows the track in its media flyout and routes the media keys here, and overlays
//! such as SCARP ISLAND read the same session to display the track and drive playback.
//! Button presses come back as `media-control` events, which the window handles like its
//! own buttons since it owns the player state. The interface sends an update only when
//! playback changes, so an uninterrupted track costs nothing.

use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use souvlaki::{
    Error, MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use tauri::{AppHandle, Emitter};

pub const CONTROL_EVENT: &str = "media-control";
const DISPLAY_NAME: &str = "SCARPIFY";
/// Unused on Windows, but part of the cross-platform config.
const DBUS_NAME: &str = "scarpify";
/// How far the session's fast-forward and rewind buttons move the track.
const SEEK_STEP_MS: i64 = 10_000;

/// What the session displays. Metadata is resent only when this changes, because Windows
/// reloads the cover every time it is set.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTrack {
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub artwork: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    #[serde(flatten)]
    pub track: SessionTrack,
    pub position_ms: u64,
    pub playing: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "action", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum Control {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Seek { position_ms: u64 },
    SeekBy { offset_ms: i64 },
}

impl Control {
    fn from_event(event: MediaControlEvent) -> Option<Self> {
        let step = |direction: SeekDirection, by: Duration| {
            let offset = by.as_millis().min(i64::MAX as u128) as i64;
            Self::SeekBy { offset_ms: if direction == SeekDirection::Forward { offset } else { -offset } }
        };
        Some(match event {
            MediaControlEvent::Play => Self::Play,
            // There is no separate stop: like other players, it pauses and keeps the track.
            MediaControlEvent::Pause | MediaControlEvent::Stop => Self::Pause,
            MediaControlEvent::Toggle => Self::Toggle,
            MediaControlEvent::Next => Self::Next,
            MediaControlEvent::Previous => Self::Previous,
            MediaControlEvent::SetPosition(MediaPosition(position)) => {
                Self::Seek { position_ms: position.as_millis().min(u64::MAX as u128) as u64 }
            }
            MediaControlEvent::Seek(direction) => step(direction, Duration::from_millis(SEEK_STEP_MS as u64)),
            MediaControlEvent::SeekBy(direction, by) => step(direction, by),
            _ => return None,
        })
    }
}

pub struct MediaSession {
    updates: Sender<Option<MediaState>>,
}

impl MediaSession {
    /// Binds the session to the main window; Windows ties media sessions to a window handle.
    pub fn spawn(app: AppHandle, hwnd: *mut c_void) -> Self {
        let (updates, receiver) = mpsc::channel();
        let hwnd = hwnd as isize;
        thread::Builder::new()
            .name("media-session".into())
            .spawn(move || run(app, hwnd, receiver))
            .expect("failed to spawn media session thread");
        Self { updates }
    }

    /// `None` when nothing is loaded.
    pub fn update(&self, state: Option<MediaState>) {
        let _ = self.updates.send(state);
    }
}

fn run(app: AppHandle, hwnd: isize, updates: Receiver<Option<MediaState>>) {
    let config = PlatformConfig { display_name: DISPLAY_NAME, dbus_name: DBUS_NAME, hwnd: Some(hwnd as *mut c_void) };
    let mut controls = match MediaControls::new(config) {
        Ok(controls) => controls,
        Err(err) => {
            eprintln!("media session unavailable: {err}");
            return;
        }
    };
    let attached = controls.attach(move |event| {
        if let Some(control) = Control::from_event(event) {
            let _ = app.emit(CONTROL_EVENT, control);
        }
    });
    if let Err(err) = attached {
        eprintln!("media session unavailable: {err}");
        return;
    }

    let mut shown: Option<SessionTrack> = None;
    for state in updates {
        if let Err(err) = apply(&mut controls, &mut shown, state) {
            eprintln!("media session update failed: {err}");
        }
    }
}

fn apply(controls: &mut MediaControls, shown: &mut Option<SessionTrack>, state: Option<MediaState>) -> Result<(), Error> {
    let Some(state) = state else {
        *shown = None;
        return controls.set_playback(MediaPlayback::Stopped);
    };

    if shown.as_ref() != Some(&state.track) {
        let track = &state.track;
        let artist = track.artists.join(", ");
        controls.set_metadata(MediaMetadata {
            title: Some(&track.title),
            artist: Some(&artist),
            // An empty album replaces the previous track's instead of keeping it.
            album: Some(track.album.as_deref().unwrap_or_default()),
            cover_url: track.artwork.as_deref().filter(|url| is_web_url(url)),
            duration: track.duration_ms.map(Duration::from_millis),
        })?;
        *shown = Some(state.track.clone());
    }

    let progress = Some(MediaPosition(Duration::from_millis(state.position_ms)));
    controls.set_playback(if state.playing {
        MediaPlayback::Playing { progress }
    } else {
        MediaPlayback::Paused { progress }
    })
}

/// Windows loads covers from web addresses itself; anything else would fail the update.
fn is_web_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_session_buttons_to_player_actions() {
        assert_eq!(Control::from_event(MediaControlEvent::Stop), Some(Control::Pause));
        assert_eq!(
            Control::from_event(MediaControlEvent::Seek(SeekDirection::Backward)),
            Some(Control::SeekBy { offset_ms: -SEEK_STEP_MS })
        );
        assert_eq!(
            Control::from_event(MediaControlEvent::SetPosition(MediaPosition(Duration::from_millis(1_500)))),
            Some(Control::Seek { position_ms: 1_500 })
        );
        assert_eq!(Control::from_event(MediaControlEvent::Raise), None);
    }

    #[test]
    fn serializes_controls_for_the_window() {
        let json = serde_json::to_value(Control::SeekBy { offset_ms: 10_000 }).unwrap();
        assert_eq!(json, serde_json::json!({ "action": "seekBy", "offsetMs": 10_000 }));
        let json = serde_json::to_value(Control::Next).unwrap();
        assert_eq!(json, serde_json::json!({ "action": "next" }));
    }

    #[test]
    fn reads_the_state_the_window_sends() {
        let state: MediaState = serde_json::from_value(serde_json::json!({
            "title": "Song", "artists": ["A", "B"], "album": null, "artwork": "https://cover",
            "durationMs": 90_000, "positionMs": 1_000, "playing": true
        }))
        .unwrap();
        assert_eq!(state.track.artists, ["A", "B"]);
        assert!(state.playing);
        assert!(is_web_url("https://cover") && !is_web_url("data:image/png;base64,AA"));
    }
}

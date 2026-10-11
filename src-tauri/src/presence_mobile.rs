//! Android has no desktop Discord IPC service. Keep the command contract compatible.
use serde::Deserialize;

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


pub struct Presence;

impl Presence {
    pub fn spawn() -> Self { Self }
    pub fn show(&self, _now_playing: NowPlaying) {}
    pub fn hide(&self) {}
    pub fn set_enabled(&self, _enabled: bool) {}
}

//! Opt-in, offline native E2E fixtures. Never enabled in distributable release builds.
use crate::model::{Collection, CollectionKind, Provider, Track};
pub const AUDIO: &[u8] = include_bytes!("../tests/fixtures/tone.mp3");
pub fn tracks() -> Vec<Track> {
    (1..=3).map(|n| Track {
        id: format!("scarpify-test-{n}"), provider: Provider::Youtube,
        title: format!("Offline test tone {n}"), artists: vec!["SCARPIFY test fixture".into()],
        album: Some("Offline playback tests".into()), duration_ms: Some(12_000),
        artwork: None, url: None,
    }).collect()
}
pub fn collection() -> Collection {
    Collection { kind: CollectionKind::Playlist, name: "Offline test playlist".into(),
        owner: None, artwork: None, provider: Provider::Youtube, tracks: tracks() }
}

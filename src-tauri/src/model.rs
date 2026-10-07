use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Youtube,
    Soundcloud,
    Spotify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Provider-specific identifier: YouTube video id, SoundCloud track id or Spotify URI.
    pub id: String,
    pub provider: Provider,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub artwork: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    Playlist,
    Album,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub kind: CollectionKind,
    pub name: String,
    pub owner: Option<String>,
    pub artwork: Option<String>,
    pub provider: Provider,
    pub tracks: Vec<Track>,
}

/// Each provider is searched separately so a slow or failing service never holds back the other.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderResult {
    pub tracks: Vec<Track>,
    pub error: Option<String>,
}

impl From<Result<Vec<Track>, crate::error::Error>> for ProviderResult {
    fn from(result: Result<Vec<Track>, crate::error::Error>) -> Self {
        match result {
            Ok(tracks) => Self { tracks, error: None },
            Err(err) => Self {
                tracks: Vec::new(),
                error: Some(err.to_string()),
            },
        }
    }
}

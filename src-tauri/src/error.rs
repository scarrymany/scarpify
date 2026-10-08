use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Http(reqwest::Error),
    #[error("YouTube Music: {0}")]
    Youtube(#[from] rustypipe::error::Error),
    #[error("invalid response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("library: {0}")]
    Library(#[from] rusqlite::Error),
    #[error("audio output: {0}")]
    Output(String),
    #[error("cannot decode stream: {0}")]
    Decode(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("unsupported link")]
    UnsupportedLink,
    /// Another track was started before this one finished loading.
    #[error("cancelled")]
    Cancelled,
}

/// Request URLs carry signatures and client ids that only clutter user-facing messages.
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err.without_url())
    }
}

/// Tauri commands return errors to the UI as plain messages.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

use std::collections::HashMap;
use std::sync::Mutex as StdMutex;

use reqwest::header::USER_AGENT;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::sync::Mutex;

use super::{ResolvedStream, BROWSER_USER_AGENT};
use crate::audio::stream::MediaSource;
use crate::error::{Error, Result};
use crate::model::{Collection, CollectionKind, Provider, Track};

const HOME_URL: &str = "https://soundcloud.com/";
const API_URL: &str = "https://api-v2.soundcloud.com";
const SEARCH_LIMIT: &str = "30";
/// The web app embeds its public client id in the hydration data of every page.
const CLIENT_ID_MARKER: &str = "\"hydratable\":\"apiClient\",\"data\":{\"id\":\"";
const CLIENT_ID_LEN: usize = 32;
/// Go+ tracks only expose a 30 second preview to anonymous clients.
const POLICY_SNIP: &str = "SNIP";
const ENCRYPTED_PROTOCOL: &str = "encrypted";
const HOST: &str = "soundcloud.com";
const SET_PATH: &str = "/sets/";
const KIND_PLAYLIST: &str = "playlist";
/// `/tracks?ids=` accepts at most this many ids per request.
const TRACKS_PER_REQUEST: usize = 50;
/// Search results are cached per session; the cap only guards against unbounded growth.
const PLAYBACK_CACHE_LIMIT: usize = 2000;
const ARTWORK_SMALL: &str = "-large.";
const ARTWORK_LARGE: &str = "-t500x500.";

pub struct SoundCloud {
    http: Client,
    client_id: Mutex<Option<String>>,
    playback: StdMutex<HashMap<String, Playback>>,
}

#[derive(Deserialize)]
struct SearchPage {
    collection: Vec<ApiTrack>,
}

#[derive(Deserialize)]
struct ApiTrack {
    id: u64,
    title: String,
    duration: Option<u64>,
    artwork_url: Option<String>,
    permalink_url: Option<String>,
    policy: Option<String>,
    track_authorization: Option<String>,
    user: ApiUser,
    publisher_metadata: Option<PublisherMetadata>,
    media: Option<Media>,
}

#[derive(Deserialize)]
struct ApiPlaylist {
    kind: String,
    /// Empty for user playlists, `album`, `ep` and similar for releases.
    #[serde(default)]
    set_type: Option<String>,
    title: String,
    artwork_url: Option<String>,
    user: ApiUser,
    /// Only the first few tracks are complete; the rest are `{ id, ... }` stubs.
    tracks: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct ApiUser {
    username: String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct PublisherMetadata {
    artist: Option<String>,
}

#[derive(Deserialize)]
struct Media {
    transcodings: Vec<Transcoding>,
}

#[derive(Clone, Deserialize)]
struct Transcoding {
    url: String,
    #[serde(default)]
    snipped: bool,
    format: Format,
}

#[derive(Clone, Deserialize)]
struct Format {
    protocol: String,
    mime_type: String,
}

#[derive(Deserialize)]
struct StreamLocation {
    url: String,
}

/// What is needed to start a track, kept from search results to skip a metadata request.
#[derive(Clone)]
struct Playback {
    transcodings: Vec<Transcoding>,
    authorization: String,
}

/// Preferred first: progressive MP3 is a single seekable file.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Progressive,
    HlsMp3,
    HlsAac,
}

impl Transcoding {
    fn kind(&self) -> Option<Kind> {
        if self.snipped {
            return None;
        }
        let mime = self.format.mime_type.as_str();
        match self.format.protocol.as_str() {
            "progressive" if mime == "audio/mpeg" => Some(Kind::Progressive),
            "hls" if mime == "audio/mpeg" => Some(Kind::HlsMp3),
            "hls" if mime.starts_with("audio/mp4") => Some(Kind::HlsAac),
            _ => None,
        }
    }

    /// Label releases are DRM-encrypted; their plain transcodings are listed but return 404.
    fn is_encrypted(&self) -> bool {
        self.format.protocol.contains(ENCRYPTED_PROTOCOL)
    }
}

impl Playback {
    fn is_playable(&self) -> bool {
        !self.transcodings.iter().any(Transcoding::is_encrypted)
            && self.transcodings.iter().any(|t| t.kind().is_some())
    }

    fn candidates(&self) -> Vec<(&Transcoding, Kind)> {
        let mut candidates: Vec<_> = self
            .transcodings
            .iter()
            .filter_map(|t| t.kind().map(|kind| (t, kind)))
            .collect();
        candidates.sort_by_key(|(_, kind)| *kind);
        candidates
    }
}

impl ApiTrack {
    fn into_parts(self) -> (Track, Option<Playback>) {
        let playback = (self.policy.as_deref() != Some(POLICY_SNIP))
            .then(|| Playback {
                transcodings: self.media.map(|m| m.transcodings).unwrap_or_default(),
                authorization: self.track_authorization.unwrap_or_default(),
            })
            .filter(Playback::is_playable);

        let artist = self
            .publisher_metadata
            .and_then(|m| m.artist)
            .filter(|a| !a.trim().is_empty())
            .unwrap_or(self.user.username);
        let artwork = self
            .artwork_url
            .or(self.user.avatar_url)
            .map(|url| url.replace(ARTWORK_SMALL, ARTWORK_LARGE));

        let track = Track {
            id: self.id.to_string(),
            provider: Provider::Soundcloud,
            title: self.title,
            artists: vec![artist],
            album: None,
            duration_ms: self.duration,
            artwork,
            url: self.permalink_url,
        };
        (track, playback)
    }
}

impl SoundCloud {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            client_id: Mutex::new(None),
            playback: StdMutex::default(),
        }
    }

    pub async fn warm_up(&self) -> Result<()> {
        self.client_id(false).await.map(drop)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Track>> {
        let url = format!("{API_URL}/search/tracks");
        let page: SearchPage = self
            .api_get(&url, &[("q", query), ("limit", SEARCH_LIMIT)])
            .await?;

        let mut cache = self.lock_playback();
        if cache.len() > PLAYBACK_CACHE_LIMIT {
            cache.clear();
        }
        Ok(page
            .collection
            .into_iter()
            .filter_map(|api_track| {
                let (track, playback) = api_track.into_parts();
                cache.insert(track.id.clone(), playback?);
                Some(track)
            })
            .collect())
    }

    pub async fn import(&self, link: &str) -> Result<Collection> {
        let link = link.trim();
        if !link.contains(HOST) || !link.contains(SET_PATH) {
            return Err(Error::UnsupportedLink);
        }
        let url = link.split(['?', '#']).next().unwrap_or(link);
        let playlist: ApiPlaylist = self
            .api_get(&format!("{API_URL}/resolve"), &[("url", url)])
            .await?;
        if playlist.kind != KIND_PLAYLIST {
            return Err(Error::UnsupportedLink);
        }

        let ids: Vec<u64> = playlist
            .tracks
            .iter()
            .filter_map(|t| t.get("id").and_then(serde_json::Value::as_u64))
            .collect();
        let mut complete: HashMap<u64, ApiTrack> = playlist
            .tracks
            .into_iter()
            .filter_map(|t| serde_json::from_value::<ApiTrack>(t).ok())
            .map(|t| (t.id, t))
            .collect();

        let missing: Vec<String> = ids
            .iter()
            .filter(|id| !complete.contains_key(id))
            .map(u64::to_string)
            .collect();
        for chunk in missing.chunks(TRACKS_PER_REQUEST) {
            let batch: Vec<ApiTrack> = self
                .api_get(&format!("{API_URL}/tracks"), &[("ids", chunk.join(",").as_str())])
                .await?;
            complete.extend(batch.into_iter().map(|t| (t.id, t)));
        }

        let mut cache = self.lock_playback();
        let tracks = ids
            .iter()
            .filter_map(|id| complete.remove(id))
            .filter_map(|api_track| {
                let (track, playback) = api_track.into_parts();
                cache.insert(track.id.clone(), playback?);
                Some(track)
            })
            .collect();

        let kind = match playlist.set_type.as_deref() {
            None | Some("") => CollectionKind::Playlist,
            Some(_) => CollectionKind::Album,
        };
        Ok(Collection {
            kind,
            name: playlist.title,
            owner: Some(playlist.user.username),
            artwork: playlist
                .artwork_url
                .or(playlist.user.avatar_url)
                .map(|url| url.replace(ARTWORK_SMALL, ARTWORK_LARGE)),
            provider: Provider::Soundcloud,
            tracks,
        })
    }

    pub async fn stream(&self, track_id: &str) -> Result<ResolvedStream> {
        let cached = self.lock_playback().get(track_id).cloned();
        let playback = match cached {
            Some(playback) => playback,
            None => {
                let track: ApiTrack = self
                    .api_get(&format!("{API_URL}/tracks/{track_id}"), &[])
                    .await?;
                track.into_parts().1.ok_or_else(unavailable)?
            }
        };

        // A listed transcoding can still be missing on the CDN, so fall through to the next.
        let mut last_error = unavailable();
        for (transcoding, kind) in playback.candidates() {
            let params = [("track_authorization", playback.authorization.as_str())];
            match self.api_get::<StreamLocation>(&transcoding.url, &params).await {
                Ok(location) => return Ok(resolved(location.url, kind)),
                Err(err) => last_error = err,
            }
        }
        Err(last_error)
    }

    fn lock_playback(&self) -> std::sync::MutexGuard<'_, HashMap<String, Playback>> {
        self.playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Calls the API, refreshing the scraped client id once if it has been rotated.
    async fn api_get<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T> {
        let mut refreshed = false;
        loop {
            let client_id = self.client_id(refreshed).await?;
            let response = self
                .http
                .get(url)
                .header(USER_AGENT, BROWSER_USER_AGENT)
                .query(params)
                .query(&[("client_id", client_id.as_str())])
                .send()
                .await?;

            let status = response.status();
            if !refreshed && (status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN) {
                refreshed = true;
                continue;
            }
            return Ok(response.error_for_status()?.json().await?);
        }
    }

    async fn client_id(&self, force_refresh: bool) -> Result<String> {
        let mut cached = self.client_id.lock().await;
        if let (Some(id), false) = (cached.as_ref(), force_refresh) {
            return Ok(id.clone());
        }

        let page = self
            .http
            .get(HOME_URL)
            .header(USER_AGENT, BROWSER_USER_AGENT)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let id = extract_client_id(&page)
            .ok_or_else(|| Error::Unavailable("SoundCloud client id not found".into()))?;
        *cached = Some(id.clone());
        Ok(id)
    }
}

fn unavailable() -> Error {
    Error::Unavailable("track is not available for streaming".into())
}

fn resolved(url: String, kind: Kind) -> ResolvedStream {
    let (source, format_hint) = match kind {
        Kind::Progressive => (MediaSource::Direct { url }, "mp3"),
        Kind::HlsMp3 => (MediaSource::Hls { playlist_url: url }, "mp3"),
        Kind::HlsAac => (MediaSource::Hls { playlist_url: url }, "mp4"),
    };
    ResolvedStream {
        source,
        format_hint,
        gain: 1.0,
    }
}

fn extract_client_id(page: &str) -> Option<String> {
    let start = page.find(CLIENT_ID_MARKER)? + CLIENT_ID_MARKER.len();
    let id = page.get(start..start + CLIENT_ID_LEN)?;
    id.chars()
        .all(|c| c.is_ascii_alphanumeric())
        .then(|| id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_client_id_from_hydration() {
        let page = r#"window.__sc_hydration = [{"hydratable":"apiClient","data":{"id":"tALhrIHY34p48SpHQCwLnMs3ivsiyNZI","isExpiring":false}}]"#;
        assert_eq!(
            extract_client_id(page).as_deref(),
            Some("tALhrIHY34p48SpHQCwLnMs3ivsiyNZI")
        );
        assert_eq!(extract_client_id("no data"), None);
    }
}

use std::path::PathBuf;

use reqwest::header::{CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use rustypipe::client::RustyPipe;
use rustypipe::model::{Thumbnail, TrackItem};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::Mutex;

use super::{ResolvedStream, BROWSER_USER_AGENT};
use crate::audio::stream::MediaSource;
use crate::error::{Error, Result};
use crate::model::{Collection, CollectionKind, Provider, Track};

const WATCH_URL: &str = "https://music.youtube.com/watch?v=";
const HOME_URL: &str = "https://www.youtube.com/";
const PLAYER_URL: &str = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
/// Album art large enough for the now-playing view without pulling full-size images.
const PREFERRED_ARTWORK_WIDTH: u32 = 544;
/// Upper bound for imported playlists; YouTube serves 100 tracks per page.
const PLAYLIST_LIMIT: usize = 2000;
const PLAYLIST_PARAM: &str = "list=";
const ALBUM_PATH: &str = "/browse/";
const ALBUM_ID_PREFIX: &str = "MPREb";

/// The visionOS client returns plain stream URLs that are fully downloadable without
/// PO tokens or player JS deobfuscation. When YouTube changes this, only these values
/// need to follow yt-dlp's `INNERTUBE_CLIENTS` table.
mod stream_client {
    pub const NAME: &str = "VISIONOS";
    pub const ID: &str = "101";
    pub const VERSION: &str = "1.02";
    pub const DEVICE_MAKE: &str = "Apple";
    pub const DEVICE_MODEL: &str = "RealityDevice17,1";
    pub const OS_NAME: &str = "visionOS";
    pub const OS_VERSION: &str = "26.5.23O471";
    pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
}

const VISITOR_DATA_MARKER: &str = "\"VISITOR_DATA\":\"";
const STATUS_OK: &str = "OK";
const STATUS_LOGIN_REQUIRED: &str = "LOGIN_REQUIRED";
/// Fresh visitor ids to try before giving up on a stream.
const STREAM_ATTEMPTS: usize = 3;
const TAIL_PROBE_BYTES: u64 = 1024;
/// AAC-LC: HE-AAC (itag 139) needs SBR, which Symphonia does not implement.
const AAC_LC_CODEC: &str = "mp4a.40.2";

pub struct Youtube {
    client: RustyPipe,
    http: Client,
    visitor_data: Mutex<Option<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerResponse {
    playability_status: PlayabilityStatus,
    streaming_data: Option<StreamingData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayabilityStatus {
    status: String,
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamingData {
    #[serde(default)]
    adaptive_formats: Vec<Format>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Format {
    url: Option<String>,
    mime_type: String,
    bitrate: u64,
    content_length: Option<String>,
    loudness_db: Option<f32>,
}

impl Youtube {
    pub fn new(http: Client, storage_dir: PathBuf) -> Result<Self> {
        let client = RustyPipe::builder()
            .storage_dir(storage_dir)
            .no_reporter()
            .no_botguard()
            .build()?;
        Ok(Self {
            client,
            http,
            visitor_data: Mutex::new(None),
        })
    }

    pub async fn warm_up(&self) -> Result<()> {
        self.visitor_data(false).await.map(drop)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Track>> {
        let result = self.client.query().music_search_tracks(query).await?;
        Ok(result.items.items.into_iter().map(to_track).collect())
    }

    pub async fn import(&self, link: &str) -> Result<Collection> {
        match parse_link(link).ok_or(Error::UnsupportedLink)? {
            Link::Playlist(id) => self.import_playlist(id).await,
            Link::Album(id) => self.import_album(id).await,
        }
    }

    async fn import_playlist(&self, id: &str) -> Result<Collection> {
        let query = self.client.query();
        let mut playlist = query.music_playlist(id).await?;
        playlist.tracks.extend_limit(&query, PLAYLIST_LIMIT).await?;
        Ok(Collection {
            kind: CollectionKind::Playlist,
            name: playlist.name,
            owner: playlist.channel.map(|c| c.name),
            artwork: pick_artwork(&playlist.thumbnail),
            provider: Provider::Youtube,
            tracks: playlist.tracks.items.into_iter().map(to_track).collect(),
        })
    }

    async fn import_album(&self, id: &str) -> Result<Collection> {
        let album = self.client.query().music_album(id).await?;
        let artwork = pick_artwork(&album.cover);
        let tracks = album
            .tracks
            .into_iter()
            .map(|item| {
                let mut track = to_track(item);
                // Album pages omit per-track covers and the album name.
                track.artwork = track.artwork.or_else(|| artwork.clone());
                track.album = track.album.or_else(|| Some(album.name.clone()));
                track
            })
            .collect();
        Ok(Collection {
            kind: CollectionKind::Album,
            name: album.name,
            owner: Some(album.artists.into_iter().map(|a| a.name).collect::<Vec<_>>().join(", ")),
            artwork,
            provider: Provider::Youtube,
            tracks,
        })
    }

    pub async fn stream(&self, video_id: &str) -> Result<ResolvedStream> {
        let mut refresh_visitor = false;
        for _ in 0..STREAM_ATTEMPTS {
            let response = self.player(video_id, refresh_visitor).await?;
            refresh_visitor = true;

            let status = response.playability_status;
            // A stale visitor id gets the bot check; a fresh one is accepted.
            if status.status == STATUS_LOGIN_REQUIRED {
                continue;
            }
            if status.status != STATUS_OK {
                return Err(Error::Unavailable(status.reason.unwrap_or(status.status)));
            }

            let (url, size, gain) = pick_audio(response.streaming_data)?;
            // Some visitor ids land in an experiment that serves only the first megabyte
            // without a PO token. Checking the tail catches it before playback starts.
            if self.is_fully_served(&url, size).await {
                return Ok(ResolvedStream {
                    gain,
                    source: MediaSource::Youtube {
                        url,
                        user_agent: stream_client::USER_AGENT.to_owned(),
                        size,
                    },
                    format_hint: "m4a",
                });
            }
        }
        Err(Error::Unavailable("YouTube refused to serve this track, try again later".into()))
    }

    async fn is_fully_served(&self, url: &str, size: u64) -> bool {
        let start = size.saturating_sub(TAIL_PROBE_BYTES);
        let probe = format!("{url}&range={start}-{}", size.saturating_sub(1));
        self.http
            .get(probe)
            .header(USER_AGENT, stream_client::USER_AGENT)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
    }

    async fn player(&self, video_id: &str, refresh_visitor: bool) -> Result<PlayerResponse> {
        let visitor_data = self.visitor_data(refresh_visitor).await?;
        let body = json!({
            "videoId": video_id,
            "context": {
                "client": {
                    "clientName": stream_client::NAME,
                    "clientVersion": stream_client::VERSION,
                    "deviceMake": stream_client::DEVICE_MAKE,
                    "deviceModel": stream_client::DEVICE_MODEL,
                    "osName": stream_client::OS_NAME,
                    "osVersion": stream_client::OS_VERSION,
                    "visitorData": visitor_data,
                    "hl": "en",
                    "gl": "US",
                }
            },
            "contentCheckOk": true,
            "racyCheckOk": true,
        });

        Ok(self
            .http
            .post(PLAYER_URL)
            .header(USER_AGENT, stream_client::USER_AGENT)
            .header(CONTENT_TYPE, "application/json")
            .header("X-YouTube-Client-Name", stream_client::ID)
            .header("X-YouTube-Client-Version", stream_client::VERSION)
            .header("X-Goog-Visitor-Id", &visitor_data)
            .body(body.to_string())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    async fn visitor_data(&self, force_refresh: bool) -> Result<String> {
        let mut cached = self.visitor_data.lock().await;
        if let (Some(data), false) = (cached.as_ref(), force_refresh) {
            return Ok(data.clone());
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
        let data = extract_visitor_data(&page)
            .ok_or_else(|| Error::Unavailable("YouTube visitor id not found".into()))?;
        *cached = Some(data.clone());
        Ok(data)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Link<'a> {
    Playlist(&'a str),
    Album(&'a str),
}

fn parse_link(link: &str) -> Option<Link<'_>> {
    let link = link.trim();
    if !link.contains("youtube.com") && !link.contains("youtu.be") {
        return None;
    }
    let is_id_char = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';

    if let Some((_, rest)) = link.split_once(ALBUM_PATH) {
        let id = rest.split(|c: char| !is_id_char(c)).next()?;
        return id.starts_with(ALBUM_ID_PREFIX).then_some(Link::Album(id));
    }
    let (_, rest) = link.split_once(PLAYLIST_PARAM)?;
    let id = rest.split(|c: char| !is_id_char(c)).next()?;
    (!id.is_empty()).then_some(Link::Playlist(id))
}

/// Best AAC-LC stream: URL, byte size and loudness normalization gain.
fn pick_audio(streaming_data: Option<StreamingData>) -> Result<(String, u64, f32)> {
    let format = streaming_data
        .into_iter()
        .flat_map(|data| data.adaptive_formats)
        .filter(|f| f.url.is_some() && f.mime_type.contains(AAC_LC_CODEC))
        .max_by_key(|f| f.bitrate)
        .ok_or_else(|| Error::Unavailable("no playable audio stream".into()))?;
    let size = format
        .content_length
        .as_deref()
        .and_then(|len| len.parse().ok())
        .ok_or_else(|| Error::Unavailable("stream size unknown".into()))?;
    let gain = format.loudness_db.map_or(1.0, normalization_gain);
    Ok((format.url.unwrap_or_default(), size, gain))
}

fn extract_visitor_data(page: &str) -> Option<String> {
    let start = page.find(VISITOR_DATA_MARKER)? + VISITOR_DATA_MARKER.len();
    let data = page[start..].split('"').next()?;
    (!data.is_empty()).then(|| data.to_owned())
}

/// YouTube reports how much louder than the reference a track is; only attenuate.
fn normalization_gain(loudness_db: f32) -> f32 {
    10f32.powf(-loudness_db.max(0.0) / 20.0)
}

fn to_track(item: TrackItem) -> Track {
    Track {
        url: Some(format!("{WATCH_URL}{}", item.id)),
        artwork: pick_artwork(&item.cover),
        id: item.id,
        provider: Provider::Youtube,
        title: item.name,
        artists: item.artists.into_iter().map(|a| a.name).collect(),
        album: item.album.map(|a| a.name),
        duration_ms: item.duration.map(|s| u64::from(s) * 1000),
    }
}

fn pick_artwork(thumbnails: &[Thumbnail]) -> Option<String> {
    thumbnails
        .iter()
        .filter(|t| t.width <= PREFERRED_ARTWORK_WIDTH)
        .max_by_key(|t| t.width)
        .or_else(|| thumbnails.iter().min_by_key(|t| t.width))
        .map(|t| t.url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_visitor_data() {
        let page = r#"ytcfg.set({"VISITOR_DATA":"CgtJQ1RsQmNmNE0zSSiJ%3D","OTHER":1})"#;
        assert_eq!(
            extract_visitor_data(page).as_deref(),
            Some("CgtJQ1RsQmNmNE0zSSiJ%3D")
        );
        assert_eq!(extract_visitor_data(r#""VISITOR_DATA":"""#), None);
    }

    #[test]
    fn parses_playlist_and_album_links() {
        assert_eq!(
            parse_link("https://music.youtube.com/playlist?list=PL5dDx681T4bR7ZF1IuWzOv1omlRbE7PiJ&si=x"),
            Some(Link::Playlist("PL5dDx681T4bR7ZF1IuWzOv1omlRbE7PiJ"))
        );
        assert_eq!(
            parse_link("https://www.youtube.com/watch?v=abc&list=OLAK5uy_nZpcQys48R0aNb046hV-n1OAHGE4reftQ"),
            Some(Link::Playlist("OLAK5uy_nZpcQys48R0aNb046hV-n1OAHGE4reftQ"))
        );
        assert_eq!(
            parse_link("https://music.youtube.com/browse/MPREb_O2gXCdCVGsZ"),
            Some(Link::Album("MPREb_O2gXCdCVGsZ"))
        );
        assert_eq!(parse_link("https://music.youtube.com/watch?v=abc"), None);
        assert_eq!(parse_link("https://soundcloud.com/a/sets/b"), None);
    }

    #[test]
    fn normalization_only_attenuates() {
        assert_eq!(normalization_gain(-3.0), 1.0);
        assert!((normalization_gain(6.0) - 0.501).abs() < 0.001);
    }
}

pub mod soundcloud;
pub mod spotify;
pub mod youtube;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use reqwest::Url;

use reqwest::Client;

use crate::audio::stream::MediaSource;
use crate::error::{Error, Result};
use crate::model::{Collection, Provider, ProviderResult, Track};
use soundcloud::SoundCloud;
use youtube::Youtube;

pub const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";

/// Candidates further apart than this are likely a different edit (live, extended, etc.).
const MATCH_DURATION_TOLERANCE_MS: u64 = 15_000;
const MATCH_CANDIDATES: usize = 5;
/// A cached stream must stay valid at least this long, or playback could start and then fail.
const EXPIRY_MARGIN: Duration = Duration::from_secs(30);
/// Lifetime assumed for signed URLs whose expiry cannot be read.
const FALLBACK_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Clone)]
pub struct ResolvedStream {
    pub source: MediaSource,
    pub format_hint: &'static str,
    pub gain: f32,
}

pub struct Sources {
    http: Client,
    youtube: Youtube,
    soundcloud: SoundCloud,
    /// Spotify URI -> matched YouTube video id, so replays skip the search.
    spotify_matches: Mutex<HashMap<String, String>>,
    /// Streams resolved ahead of time (hover, next in queue) so playback starts instantly,
    /// with the moment their signed URL expires.
    streams: Mutex<HashMap<String, (SystemTime, ResolvedStream)>>,
}

impl Sources {
    pub fn new(http: Client, storage_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            youtube: Youtube::new(http.clone(), storage_dir)?,
            soundcloud: SoundCloud::new(http.clone()),
            http,
            spotify_matches: Mutex::default(),
            streams: Mutex::default(),
        })
    }

    /// Fetches per-session tokens up front so the first search and play are not slower.
    pub async fn warm_up(&self) {
        let _ = tokio::join!(self.youtube.warm_up(), self.soundcloud.warm_up());
    }

    pub async fn search_youtube(&self, query: &str) -> ProviderResult {
        self.youtube.search(query).await.into()
    }

    pub async fn search_soundcloud(&self, query: &str) -> ProviderResult {
        self.soundcloud.search(query).await.into()
    }

    /// Imports a playlist or album, picking the service from the link.
    pub async fn import(&self, link: &str) -> Result<Collection> {
        let link = link.trim();
        if link.contains("spotify") {
            spotify::import(&self.http, link).await
        } else if link.contains("soundcloud.com") {
            self.soundcloud.import(link).await
        } else if link.contains("youtube.com") || link.contains("youtu.be") {
            self.youtube.import(link).await
        } else {
            Err(Error::UnsupportedLink)
        }
    }

    /// Returns a cached stream when one was prefetched and is still valid, resolving otherwise.
    pub async fn stream(&self, track: &Track) -> Result<ResolvedStream> {
        let key = stream_key(track);
        let cached = lock(&self.streams)
            .get(&key)
            .filter(|(expires, _)| is_fresh(*expires))
            .map(|(_, stream)| stream.clone());
        if let Some(stream) = cached {
            return Ok(stream);
        }

        let stream = self.resolve(track).await?;
        let expires = url_expiry(stream.source.url());
        let mut streams = lock(&self.streams);
        streams.retain(|_, (expires, _)| is_fresh(*expires));
        streams.insert(key, (expires, stream.clone()));
        Ok(stream)
    }

    /// Drops a cached stream, e.g. after the CDN refused it.
    pub fn forget_stream(&self, track: &Track) {
        lock(&self.streams).remove(&stream_key(track));
    }

    async fn resolve(&self, track: &Track) -> Result<ResolvedStream> {
        match track.provider {
            Provider::Youtube => self.youtube.stream(&track.id).await,
            Provider::Soundcloud => self.soundcloud.stream(&track.id).await,
            Provider::Spotify => {
                let video_id = self.match_on_youtube(track).await?;
                self.youtube.stream(&video_id).await
            }
        }
    }

    async fn match_on_youtube(&self, track: &Track) -> Result<String> {
        if let Some(id) = lock(&self.spotify_matches).get(&track.id) {
            return Ok(id.clone());
        }

        let query = format!("{} {}", track.artists.join(" "), track.title);
        let candidates = self.youtube.search(&query).await?;
        let best = candidates
            .iter()
            .take(MATCH_CANDIDATES)
            .filter_map(|c| {
                let diff = c.duration_ms?.abs_diff(track.duration_ms?);
                (diff <= MATCH_DURATION_TOLERANCE_MS).then_some((diff, c))
            })
            .min_by_key(|(diff, _)| *diff)
            .map(|(_, c)| c)
            .or_else(|| candidates.first())
            .ok_or_else(|| Error::Unavailable("no match found on YouTube Music".into()))?;

        lock(&self.spotify_matches).insert(track.id.clone(), best.id.clone());
        Ok(best.id.clone())
    }
}

fn is_fresh(expires: SystemTime) -> bool {
    SystemTime::now() + EXPIRY_MARGIN < expires
}

/// When a signed stream URL stops working: YouTube puts it in `expire`, SoundCloud's
/// CloudFront URLs in the `DateLessThan` condition of the base64 `Policy`.
fn url_expiry(url: &str) -> SystemTime {
    let fallback = SystemTime::now() + FALLBACK_LIFETIME;
    let Ok(url) = Url::parse(url) else { return fallback };

    let epoch = url.query_pairs().find_map(|(key, value)| match key.as_ref() {
        "expire" => value.parse::<u64>().ok(),
        "Policy" => policy_epoch(&value),
        _ => None,
    });
    epoch.map_or(fallback, |secs| UNIX_EPOCH + Duration::from_secs(secs))
}

fn policy_epoch(policy: &str) -> Option<u64> {
    // CloudFront's URL-safe alphabet swaps these characters of standard base64.
    let standard: String = policy
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '=',
            '~' => '/',
            other => other,
        })
        .collect();
    let json = base64::engine::general_purpose::STANDARD.decode(standard).ok()?;
    let policy: serde_json::Value = serde_json::from_slice(&json).ok()?;
    policy
        .pointer("/Statement/0/Condition/DateLessThan/AWS:EpochTime")?
        .as_u64()
}

fn stream_key(track: &Track) -> String {
    format!("{:?}:{}", track.provider, track.id)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::audio::stream::{download, StreamBuffer, StreamReader};
    use rodio::decoder::DecoderBuilder;
    use rodio::Source;

    const DECODE_SECONDS: usize = 5;
    const SEEK_TO_SECONDS: u64 = 60;

    fn decode_some(stream: ResolvedStream) -> usize {
        let buffer = StreamBuffer::new();
        let runtime = tauri::async_runtime::handle();
        runtime.spawn(download(Client::new(), stream.source, buffer.clone()));
        buffer.wait_for(128 * 1024).unwrap();
        let mut builder = DecoderBuilder::new()
            .with_data(StreamReader::new(buffer.clone()))
            .with_hint(stream.format_hint)
            .with_seekable(true);
        if let Some(len) = buffer.total_len() {
            builder = builder.with_byte_len(len);
        }
        let mut decoder = builder.build().expect("decoder");
        let rate = decoder.sample_rate().get() as usize;
        let channels = decoder.channels().get() as usize;
        decoder
            .try_seek(std::time::Duration::from_secs(SEEK_TO_SECONDS))
            .expect("seek");
        let samples = decoder.take(rate * channels * DECODE_SECONDS).count();
        // Some streams are cut after the first megabyte; only a full download proves playback.
        buffer.wait_for(usize::MAX).expect("stream download was cut short");
        assert_eq!(buffer.total_len(), Some(buffer.downloaded_len()));
        samples / (rate * channels)
    }

    #[test]
    #[ignore = "requires network access"]
    fn plays_tracks_from_every_provider() {
        let dir = std::env::temp_dir().join("scarpify-live-test");
        std::fs::create_dir_all(&dir).unwrap();
        let sources = Sources::new(Client::new(), dir).unwrap();

        let youtube = tauri::async_runtime::block_on(sources.search_youtube("daft punk get lucky"));
        let soundcloud = tauri::async_runtime::block_on(sources.search_soundcloud("daft punk get lucky"));
        assert!(youtube.error.is_none(), "{:?}", youtube.error);
        assert!(soundcloud.error.is_none(), "{:?}", soundcloud.error);

        let playlist = tauri::async_runtime::block_on(
            sources.import("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M"),
        )
        .unwrap();
        println!("spotify: {} ({} tracks)", playlist.name, playlist.tracks.len());

        for track in [
            &youtube.tracks[0],
            &soundcloud.tracks[0],
            &playlist.tracks[0],
        ] {
            let stream = tauri::async_runtime::block_on(sources.stream(track)).unwrap();
            let seconds = decode_some(stream);
            println!("{:?} {} - {}: decoded {seconds}s", track.provider, track.artists.join(", "), track.title);
            assert_eq!(seconds, DECODE_SECONDS);
        }
    }
}


#[cfg(test)]
mod live_import_tests {
    use super::*;

    #[test]
    #[ignore = "requires network access"]
    fn imports_playlists_from_every_provider() {
        let dir = std::env::temp_dir().join("scarpify-live-test");
        let sources = Sources::new(Client::new(), dir).unwrap();
        for link in [
            "https://soundcloud.com/dabbackwood/sets/crests",
            "https://music.youtube.com/playlist?list=RDCLAK5uy_kmPRjHDECIcuVwnKsx2Ng7fyNgFKWNJFs",
            "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M",
        ] {
            let collection = tauri::async_runtime::block_on(sources.import(link)).unwrap();
            let distinct_covers: std::collections::HashSet<_> =
                collection.tracks.iter().filter_map(|t| t.artwork.as_deref()).collect();
            println!(
                "{:?} '{}' by {:?}: {} tracks, {} distinct covers",
                collection.provider,
                collection.name,
                collection.owner,
                collection.tracks.len(),
                distinct_covers.len()
            );
            assert!(!collection.tracks.is_empty());
        }
    }
}


#[cfg(test)]
mod expiry_tests {
    use super::*;

    fn epoch(time: SystemTime) -> u64 {
        time.duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    #[test]
    fn reads_youtube_and_cloudfront_expiry() {
        assert_eq!(epoch(url_expiry("https://rr1.googlevideo.com/videoplayback?expire=1791448435&itag=140")), 1791448435);

        let policy = r#"{"Statement":[{"Resource":"*","Condition":{"DateLessThan":{"AWS:EpochTime":1791385947}}}]}"#;
        let encoded: String = base64::engine::general_purpose::STANDARD
            .encode(policy)
            .chars()
            .map(|c| match c { '+' => '-', '=' => '_', '/' => '~', other => other })
            .collect();
        let url = format!("https://cf-media.sndcdn.com/a.128.mp3?Policy={encoded}&Signature=x");
        assert_eq!(epoch(url_expiry(&url)), 1791385947);
    }

    #[test]
    fn unknown_urls_get_a_short_lifetime() {
        let expiry = url_expiry("https://example.com/audio.mp3");
        assert!(expiry > SystemTime::now() && expiry <= SystemTime::now() + FALLBACK_LIFETIME);
        assert!(!is_fresh(SystemTime::now() + Duration::from_secs(10)));
    }
}

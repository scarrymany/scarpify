//! Progressive download into memory with a blocking `Read + Seek` view for the decoder.
//!
//! Playback starts as soon as the first bytes arrive; the decoder simply blocks when it
//! reads past what has been downloaded so far.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;

use reqwest::header::{CONTENT_RANGE, ORIGIN, RANGE, REFERER, USER_AGENT};
use reqwest::{Client, StatusCode, Url};

/// Small enough for a fast start, large enough to keep the request count low.
const CHUNK_SIZE: u64 = 1024 * 1024;
const YOUTUBE_ORIGIN: &str = "https://www.youtube.com";
const YOUTUBE_REFERER: &str = "https://www.youtube.com/";
const MAX_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_millis(400);

#[derive(Debug, Clone)]
pub enum MediaSource {
    /// A single file that supports HTTP range requests.
    Direct { url: String },
    /// A googlevideo stream: it rejects the Range header and expects a `range` query parameter.
    Youtube {
        url: String,
        user_agent: String,
        size: u64,
    },
    /// An HLS media playlist whose segments concatenate into a playable stream.
    Hls { playlist_url: String },
}

#[derive(Default)]
struct State {
    data: Vec<u8>,
    total: Option<u64>,
    finished: bool,
    failure: Option<String>,
}

#[derive(Default)]
pub struct StreamBuffer {
    state: Mutex<State>,
    changed: Condvar,
    cancelled: AtomicBool,
}

impl StreamBuffer {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.changed.notify_all();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn total_len(&self) -> Option<u64> {
        self.lock().total
    }

    /// Blocks until `len` bytes are available or the download ends.
    pub fn wait_for(&self, len: usize) -> Result<(), String> {
        let mut state = self.lock();
        loop {
            if let Some(failure) = &state.failure {
                return Err(failure.clone());
            }
            if state.data.len() >= len || state.finished || self.is_cancelled() {
                return Ok(());
            }
            state = self.wait(state);
        }
    }

    fn push(&self, chunk: &[u8]) {
        self.lock().data.extend_from_slice(chunk);
        self.changed.notify_all();
    }

    fn set_total(&self, total: u64) {
        let mut state = self.lock();
        state.total = Some(total);
        let additional = (total as usize).saturating_sub(state.data.len());
        state.data.reserve(additional);
    }

    fn finish(&self, failure: Option<String>) {
        let mut state = self.lock();
        state.finished = true;
        state.failure = failure;
        if state.total.is_none() {
            state.total = Some(state.data.len() as u64);
        }
        drop(state);
        self.changed.notify_all();
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn wait<'a>(&self, guard: MutexGuard<'a, State>) -> MutexGuard<'a, State> {
        self.changed
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct StreamReader {
    buffer: Arc<StreamBuffer>,
    pos: u64,
}

impl StreamReader {
    pub fn new(buffer: Arc<StreamBuffer>) -> Self {
        Self { buffer, pos: 0 }
    }
}

impl Read for StreamReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let mut state = self.buffer.lock();
        loop {
            let available = state.data.len() as u64;
            if self.pos < available {
                let start = self.pos as usize;
                let count = out.len().min(state.data.len() - start);
                out[..count].copy_from_slice(&state.data[start..start + count]);
                self.pos += count as u64;
                return Ok(count);
            }
            if self.buffer.is_cancelled() {
                return Ok(0);
            }
            if state.finished {
                return match &state.failure {
                    Some(failure) => Err(io::Error::other(failure.clone())),
                    None => Ok(0),
                };
            }
            state = self.buffer.wait(state);
        }
    }
}

impl Seek for StreamReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let target = match from {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::Current(delta) => self.pos.checked_add_signed(delta),
            SeekFrom::End(delta) => {
                let mut state = self.buffer.lock();
                while state.total.is_none() && !self.buffer.is_cancelled() {
                    state = self.buffer.wait(state);
                }
                state.total.unwrap_or(0).checked_add_signed(delta)
            }
        };
        let target = target.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "seek before start of stream")
        })?;
        self.pos = target;
        Ok(target)
    }
}

pub async fn download(http: Client, source: MediaSource, buffer: Arc<StreamBuffer>) {
    let result = match source {
        MediaSource::Direct { url } => download_ranged(&http, &url, &buffer).await,
        MediaSource::Youtube {
            url,
            user_agent,
            size,
        } => download_youtube(&http, &url, &user_agent, size, &buffer).await,
        MediaSource::Hls { playlist_url } => download_hls(&http, &playlist_url, &buffer).await,
    };
    buffer.finish(result.err());
}

async fn download_ranged(http: &Client, url: &str, buffer: &StreamBuffer) -> Result<(), String> {
    let mut offset = 0u64;
    let mut attempt = 0;

    loop {
        if buffer.is_cancelled() {
            return Ok(());
        }

        let request = http
            .get(url)
            .header(RANGE, format!("bytes={}-{}", offset, offset + CHUNK_SIZE - 1));

        match fetch_range(request, offset, buffer).await {
            Ok(RangeOutcome::Partial { received, total }) => {
                attempt = 0;
                offset += received;
                if offset >= total {
                    return Ok(());
                }
            }
            Ok(RangeOutcome::Complete) => return Ok(()),
            Err(err) => {
                attempt += 1;
                if attempt >= MAX_ATTEMPTS {
                    return Err(err);
                }
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

async fn download_youtube(
    http: &Client,
    url: &str,
    user_agent: &str,
    size: u64,
    buffer: &StreamBuffer,
) -> Result<(), String> {
    let base = Url::parse(url).map_err(|e| e.to_string())?;
    buffer.set_total(size);

    let mut offset = 0u64;
    let mut attempt = 0;
    while offset < size {
        if buffer.is_cancelled() {
            return Ok(());
        }

        let end = (offset + CHUNK_SIZE).min(size) - 1;
        let mut chunk_url = base.clone();
        chunk_url
            .query_pairs_mut()
            .append_pair("range", &format!("{offset}-{end}"));
        let request = http
            .get(chunk_url)
            .header(USER_AGENT, user_agent)
            .header(ORIGIN, YOUTUBE_ORIGIN)
            .header(REFERER, YOUTUBE_REFERER);

        match fetch_body(request, buffer).await {
            Ok(0) => return Err(format!("empty chunk at {offset}")),
            Ok(received) => {
                attempt = 0;
                offset += received;
            }
            Err(err) => {
                attempt += 1;
                if attempt >= MAX_ATTEMPTS {
                    return Err(err);
                }
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
    Ok(())
}

/// Streams a response body into the buffer, returning the number of bytes received.
async fn fetch_body(request: reqwest::RequestBuilder, buffer: &StreamBuffer) -> Result<u64, String> {
    let mut response = request
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;

    let mut received = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if buffer.is_cancelled() {
            break;
        }
        buffer.push(&chunk);
        received += chunk.len() as u64;
    }
    Ok(received)
}

enum RangeOutcome {
    Partial { received: u64, total: u64 },
    Complete,
}

async fn fetch_range(
    request: reqwest::RequestBuilder,
    offset: u64,
    buffer: &StreamBuffer,
) -> Result<RangeOutcome, String> {
    let mut response = request
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;

    // A server that ignores Range sends the whole file: only usable from the start.
    if response.status() == StatusCode::OK {
        if offset != 0 {
            return Err("server does not support range requests".into());
        }
        if let Some(len) = response.content_length() {
            buffer.set_total(len);
        }
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if buffer.is_cancelled() {
                break;
            }
            buffer.push(&chunk);
        }
        return Ok(RangeOutcome::Complete);
    }

    let total = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit('/').next())
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or("missing Content-Range total")?;
    if offset == 0 {
        buffer.set_total(total);
    }

    let mut received = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if buffer.is_cancelled() {
            break;
        }
        buffer.push(&chunk);
        received += chunk.len() as u64;
    }
    Ok(RangeOutcome::Partial { received, total })
}

async fn download_hls(
    http: &Client,
    playlist_url: &str,
    buffer: &StreamBuffer,
) -> Result<(), String> {
    let base = Url::parse(playlist_url).map_err(|e| e.to_string())?;
    let playlist = http
        .get(base.clone())
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    for segment in parse_media_playlist(&playlist) {
        if buffer.is_cancelled() {
            return Ok(());
        }
        let url = base.join(segment).map_err(|e| e.to_string())?;
        let mut attempt = 0;
        loop {
            let result = http
                .get(url.clone())
                .send()
                .await
                .and_then(|r| r.error_for_status());
            let bytes = match result {
                Ok(response) => response.bytes().await,
                Err(err) => Err(err),
            };
            match bytes {
                Ok(bytes) => {
                    buffer.push(&bytes);
                    break;
                }
                Err(err) => {
                    attempt += 1;
                    if attempt >= MAX_ATTEMPTS {
                        return Err(err.to_string());
                    }
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }
    Ok(())
}

/// Returns the init segment (if any) followed by media segment URIs, in playback order.
fn parse_media_playlist(playlist: &str) -> Vec<&str> {
    const MAP_TAG: &str = "#EXT-X-MAP:";
    const URI_ATTR: &str = "URI=\"";

    let mut segments = Vec::new();
    for line in playlist.lines().map(str::trim) {
        if let Some(attrs) = line.strip_prefix(MAP_TAG) {
            let uri = attrs
                .find(URI_ATTR)
                .map(|start| &attrs[start + URI_ATTR.len()..])
                .and_then(|rest| rest.split('"').next());
            segments.extend(uri);
        } else if !line.is_empty() && !line.starts_with('#') {
            segments.push(line);
        }
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_init_and_media_segments() {
        let playlist = "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:10,\nseg1.m4s\n\n#EXTINF:5,\nhttps://cdn/seg2.m4s\n#EXT-X-ENDLIST";
        assert_eq!(
            parse_media_playlist(playlist),
            vec!["init.mp4", "seg1.m4s", "https://cdn/seg2.m4s"]
        );
    }

    #[test]
    fn reader_blocks_until_data_and_seeks_from_end() {
        let buffer = StreamBuffer::new();
        let mut reader = StreamReader::new(buffer.clone());
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            buffer.push(b"hello");
            buffer.finish(None);
        });

        let mut out = Vec::new();
        reader.read_to_end(&mut out).unwrap();
        writer.join().unwrap();
        assert_eq!(out, b"hello");
        assert_eq!(reader.seek(SeekFrom::End(-2)).unwrap(), 3);
    }
}

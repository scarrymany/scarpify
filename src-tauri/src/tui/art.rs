//! Pictures on screen: cover artwork (downloaded in the background, or inline covers chosen in
//! the window) and the interface's own drawn pieces. Each is encoded once per size and
//! background. Windows Terminal shows them as Sixel; a classic console gets half blocks for
//! covers and plain symbols instead of drawn pieces.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{self, Receiver, Sender};

use base64::Engine;
use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::Resize;
use reqwest::Client;

use super::paint::{self, Rgb, Visual};

/// Plenty for the largest cover on screen while keeping each decoded image small.
const THUMBNAIL_PX: u32 = 240;
const MAX_IMAGES: usize = 64;
const MAX_ENCODED: usize = 512;
const DATA_URL_PREFIX: &str = "data:";
const BASE64_MARKER: &str = ";base64,";

enum Slot {
    Loading,
    Ready(DynamicImage),
    Failed,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    /// Hashed: inline covers are data URLs tens of kilobytes long.
    Cover { url: u64, bg: Rgb, width: u16, height: u16 },
    Visual { visual: Visual, width: u16, height: u16 },
}

pub struct Art {
    picker: Picker,
    http: Client,
    images: HashMap<String, Slot>,
    loaded_order: VecDeque<String>,
    encoded: HashMap<Key, Protocol>,
    encoded_order: VecDeque<Key>,
    results: Receiver<(String, Option<DynamicImage>)>,
    sender: Sender<(String, Option<DynamicImage>)>,
}

impl Art {
    /// Must run after the terminal entered raw mode: it asks the terminal what it supports.
    pub fn new(http: Client, rich_terminal: bool) -> Self {
        let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
        // ConPTY may swallow the answer to the query, but Windows Terminal draws Sixel since 1.22.
        if rich_terminal && picker.protocol_type() == ProtocolType::Halfblocks {
            picker.set_protocol_type(ProtocolType::Sixel);
        }
        let (sender, results) = mpsc::channel();
        Self {
            picker,
            http,
            images: HashMap::new(),
            loaded_order: VecDeque::new(),
            encoded: HashMap::new(),
            encoded_order: VecDeque::new(),
            results,
            sender,
        }
    }

    /// Whether drawn pieces (icons, corners, sliders) can be shown, not only covers.
    pub fn pictures(&self) -> bool {
        self.picker.protocol_type() != ProtocolType::Halfblocks
    }

    /// Picks up finished downloads; returns whether anything new can be drawn.
    pub fn drain(&mut self) -> bool {
        let mut changed = false;
        while let Ok((url, image)) = self.results.try_recv() {
            changed = true;
            let slot = match image {
                Some(image) => {
                    self.loaded_order.push_back(url.clone());
                    Slot::Ready(image)
                }
                None => Slot::Failed,
            };
            self.images.insert(url, slot);
        }
        while self.loaded_order.len() > MAX_IMAGES {
            if let Some(oldest) = self.loaded_order.pop_front() {
                self.images.remove(&oldest);
            }
        }
        changed
    }

    fn pixels(&self, area: Rect) -> (u32, u32) {
        let font = self.picker.font_size();
        (area.width as u32 * font.width as u32, area.height as u32 * font.height as u32)
    }

    /// The cover for `url` with rounded corners on `bg`, or `None` while it loads.
    pub fn cover(&mut self, url: &str, area: Rect, bg: Rgb) -> Option<&Protocol> {
        if area.is_empty() {
            return None;
        }
        let key = Key::Cover { url: hash(url), bg, width: area.width, height: area.height };
        if !self.encoded.contains_key(&key) {
            let image = match self.images.get(url) {
                Some(Slot::Ready(image)) => image,
                Some(Slot::Loading | Slot::Failed) => return None,
                None => {
                    self.request(url);
                    return None;
                }
            };
            let protocol = if self.pictures() {
                let (width, height) = self.pixels(area);
                let rounded = paint::cover(image, width, height, bg);
                self.picker.new_protocol(rounded, area.as_size(), Resize::Fit(None)).ok()?
            } else {
                self.picker.new_protocol(image.clone(), area.as_size(), Resize::Scale(None)).ok()?
            };
            self.remember(key.clone(), protocol);
        }
        self.encoded.get(&key)
    }

    /// A drawn piece sized to `area`; `None` on consoles without pictures.
    pub fn visual(&mut self, visual: Visual, area: Rect) -> Option<&Protocol> {
        if area.is_empty() || !self.pictures() {
            return None;
        }
        let key = Key::Visual { visual, width: area.width, height: area.height };
        if !self.encoded.contains_key(&key) {
            let (width, height) = self.pixels(area);
            let image = paint::draw(visual, width, height)?;
            let protocol = self.picker.new_protocol(image, area.as_size(), Resize::Fit(None)).ok()?;
            self.remember(key.clone(), protocol);
        }
        self.encoded.get(&key)
    }

    fn remember(&mut self, key: Key, protocol: Protocol) {
        self.encoded.insert(key.clone(), protocol);
        self.encoded_order.push_back(key);
        if self.encoded_order.len() > MAX_ENCODED {
            if let Some(oldest) = self.encoded_order.pop_front() {
                self.encoded.remove(&oldest);
            }
        }
    }

    fn request(&mut self, url: &str) {
        self.images.insert(url.to_owned(), Slot::Loading);
        let http = self.http.clone();
        let sender = self.sender.clone();
        let url = url.to_owned();
        tauri::async_runtime::spawn(async move {
            let bytes = if url.starts_with(DATA_URL_PREFIX) {
                inline_bytes(&url)
            } else {
                match http.get(&url).send().await.and_then(|r| r.error_for_status()) {
                    Ok(response) => response.bytes().await.ok().map(|b| b.to_vec()),
                    Err(_) => None,
                }
            };
            let image = match bytes {
                Some(bytes) => tauri::async_runtime::spawn_blocking(move || decode(&bytes)).await.ok().flatten(),
                None => None,
            };
            let _ = sender.send((url, image));
        });
    }
}

fn hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Covers picked in the window are stored inline as `data:image/…;base64,…`.
fn inline_bytes(url: &str) -> Option<Vec<u8>> {
    let (_, payload) = url.split_once(BASE64_MARKER)?;
    base64::engine::general_purpose::STANDARD.decode(payload).ok()
}

fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    let image = image::load_from_memory(bytes).ok()?;
    Some(DynamicImage::ImageRgb8(image.thumbnail(THUMBNAIL_PX, THUMBNAIL_PX).to_rgb8()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_inline_covers() {
        assert_eq!(inline_bytes("data:image/png;base64,AAEC"), Some(vec![0, 1, 2]));
        assert_eq!(inline_bytes("https://example.com/cover.jpg"), None);
    }
}

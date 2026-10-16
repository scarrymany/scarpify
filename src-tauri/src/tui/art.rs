//! Cover artwork: downloaded in the background, shrunk to thumbnails and encoded once per
//! size. Windows Terminal shows real pictures (Sixel); a classic console gets half blocks.

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};

use image::DynamicImage;
use ratatui::layout::Rect;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::Resize;
use reqwest::Client;

/// Plenty for the largest cover on screen while keeping each decoded image under 80 KB.
const THUMBNAIL_PX: u32 = 160;
const MAX_IMAGES: usize = 64;
const MAX_ENCODED: usize = 160;

enum Slot {
    Loading,
    Ready(DynamicImage),
    Failed,
}

pub struct Art {
    picker: Picker,
    http: Client,
    images: HashMap<String, Slot>,
    loaded_order: VecDeque<String>,
    encoded: HashMap<(String, u16, u16), Protocol>,
    encoded_order: VecDeque<(String, u16, u16)>,
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

    /// The picture for `url` sized to `area`, or `None` while it loads (draw a placeholder).
    pub fn get(&mut self, url: &str, area: Rect) -> Option<&Protocol> {
        if area.width == 0 || area.height == 0 {
            return None;
        }
        let key = (url.to_owned(), area.width, area.height);
        if !self.encoded.contains_key(&key) {
            let image = match self.images.get(url) {
                Some(Slot::Ready(image)) => image.clone(),
                Some(Slot::Loading | Slot::Failed) => return None,
                None => {
                    self.request(url);
                    return None;
                }
            };
            let protocol = self.picker.new_protocol(image, area.as_size(), Resize::Scale(None)).ok()?;
            self.encoded.insert(key.clone(), protocol);
            self.encoded_order.push_back(key.clone());
            if self.encoded_order.len() > MAX_ENCODED {
                if let Some(oldest) = self.encoded_order.pop_front() {
                    self.encoded.remove(&oldest);
                }
            }
        }
        self.encoded.get(&key)
    }

    fn request(&mut self, url: &str) {
        self.images.insert(url.to_owned(), Slot::Loading);
        let http = self.http.clone();
        let sender = self.sender.clone();
        let url = url.to_owned();
        tauri::async_runtime::spawn(async move {
            let bytes = match http.get(&url).send().await.and_then(|r| r.error_for_status()) {
                Ok(response) => response.bytes().await.ok(),
                Err(_) => None,
            };
            let image = match bytes {
                Some(bytes) => tauri::async_runtime::spawn_blocking(move || decode(&bytes)).await.ok().flatten(),
                None => None,
            };
            let _ = sender.send((url, image));
        });
    }
}

fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    let image = image::load_from_memory(bytes).ok()?;
    Some(DynamicImage::ImageRgb8(image.thumbnail(THUMBNAIL_PX, THUMBNAIL_PX).to_rgb8()))
}

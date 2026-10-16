//! Pictures the terminal draws itself, so it looks like the window rather than like text:
//! the window's icons, round buttons, rounded corners, sliders, switches and placeholders.
//! Each one is a small SVG rasterized to the exact pixels of the cells it covers.

use image::imageops::FilterType;
use image::{DynamicImage, Rgb as Pixel, RgbImage};
use ratatui::style::Color;
use resvg::{tiny_skia, usvg};

use super::icon_paths::{self as paths, Shape};
use crate::model::Provider;

pub type Rgb = [u8; 3];

/// Icons fill this share of their square, leaving the breathing room Phosphor expects.
const ICON_SCALE: f32 = 0.8;
const ROUND_ICON_SCALE: f32 = 0.46;
const PLACEHOLDER_ICON_SCALE: f32 = 0.42;
/// Matches `--radius-card` in the window, in pixels.
const COVER_RADIUS: f32 = 6.0;
const SLIDER_TRACK: f32 = 4.0;
const SLIDER_KNOB: f32 = 6.0;
/// Slider fill is quantized to tenths of a percent, which is finer than a pixel.
pub const SLIDER_STEPS: u16 = 1000;
pub const BAR_LEVELS: u8 = 8;

pub fn rgb(color: Color) -> Rgb {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => [0, 0, 0],
    }
}

pub fn mix(from: Rgb, to: Rgb, amount: f32) -> Rgb {
    let channel = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
    [channel(from[0], to[0]), channel(from[1], to[1]), channel(from[2], to[2])]
}

fn hex(color: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Home,
    HomeFill,
    Search,
    SearchBold,
    Library,
    Plus,
    Gear,
    GearFill,
    Heart,
    HeartFill,
    Pin,
    Unpin,
    Shuffle,
    Previous,
    Next,
    Play,
    Pause,
    Repeat,
    RepeatOnce,
    Queue,
    VolumeHigh,
    VolumeLow,
    Muted,
    Back,
    Forward,
    Down,
    Close,
    More,
    Enqueue,
    Notes,
    NotesPlus,
    Link,
    Trash,
    Pencil,
    External,
    Restore,
    Clock,
    Warning,
    Window,
    Check,
    Telegram,
    YoutubeMusic,
    Soundcloud,
    Spotify,
}

impl Icon {
    pub fn provider(provider: Provider) -> Icon {
        match provider {
            Provider::Youtube => Icon::YoutubeMusic,
            Provider::Soundcloud => Icon::Soundcloud,
            Provider::Spotify => Icon::Spotify,
        }
    }

    fn shape(self) -> &'static Shape {
        match self {
            Icon::Home => &paths::HOUSE,
            Icon::HomeFill => &paths::HOUSE_FILL,
            Icon::Search => &paths::SEARCH,
            Icon::SearchBold => &paths::SEARCH_BOLD,
            Icon::Library => &paths::BOOKS,
            Icon::Plus => &paths::PLUS,
            Icon::Gear => &paths::GEAR,
            Icon::GearFill => &paths::GEAR_FILL,
            Icon::Heart => &paths::HEART,
            Icon::HeartFill => &paths::HEART_FILL,
            Icon::Pin => &paths::PIN,
            Icon::Unpin => &paths::UNPIN,
            Icon::Shuffle => &paths::SHUFFLE,
            Icon::Previous => &paths::SKIP_BACK,
            Icon::Next => &paths::SKIP_FORWARD,
            Icon::Play => &paths::PLAY,
            Icon::Pause => &paths::PAUSE,
            Icon::Repeat => &paths::REPEAT,
            Icon::RepeatOnce => &paths::REPEAT_ONCE,
            Icon::Queue => &paths::QUEUE,
            Icon::VolumeHigh => &paths::SPEAKER_HIGH,
            Icon::VolumeLow => &paths::SPEAKER_LOW,
            Icon::Muted => &paths::SPEAKER_SLASH,
            Icon::Back => &paths::CARET_LEFT,
            Icon::Forward => &paths::CARET_RIGHT,
            Icon::Down => &paths::CARET_DOWN,
            Icon::Close => &paths::CLOSE,
            Icon::More => &paths::MORE,
            Icon::Enqueue => &paths::LIST_PLUS,
            Icon::Notes => &paths::MUSIC_NOTES,
            Icon::NotesPlus => &paths::MUSIC_NOTES_PLUS,
            Icon::Link => &paths::LINK,
            Icon::Trash => &paths::TRASH,
            Icon::Pencil => &paths::PENCIL,
            Icon::External => &paths::EXTERNAL,
            Icon::Restore => &paths::RESTORE,
            Icon::Clock => &paths::CLOCK,
            Icon::Warning => &paths::WARNING,
            Icon::Window => &paths::WINDOW,
            Icon::Check => &paths::CHECK,
            Icon::Telegram => &paths::TELEGRAM,
            Icon::YoutubeMusic => &paths::YOUTUBE_MUSIC,
            Icon::Soundcloud => &paths::SOUNDCLOUD,
            Icon::Spotify => &paths::SPOTIFY,
        }
    }

    /// A one-cell stand-in for consoles that cannot show pictures.
    pub fn glyph(self) -> &'static str {
        match self {
            Icon::Home | Icon::HomeFill => "⌂",
            Icon::Search | Icon::SearchBold => "⌕",
            Icon::Library => "≡",
            Icon::Plus | Icon::NotesPlus => "+",
            Icon::Gear | Icon::GearFill => "☼",
            Icon::Heart => "♡",
            Icon::HeartFill => "♥",
            Icon::Pin | Icon::Unpin => "•",
            Icon::Shuffle => "⇄",
            Icon::Previous => "«",
            Icon::Next => "»",
            Icon::Play => "▶",
            Icon::Pause => "‖",
            Icon::Repeat | Icon::RepeatOnce | Icon::Restore => "↻",
            Icon::Queue | Icon::Enqueue => "☰",
            Icon::VolumeHigh | Icon::VolumeLow => "♪",
            Icon::Muted => "×",
            Icon::Back => "‹",
            Icon::Forward => "›",
            Icon::Down => "▾",
            Icon::Close | Icon::Trash => "×",
            Icon::More => "…",
            Icon::Notes => "♫",
            Icon::Link | Icon::External | Icon::Telegram => "↗",
            Icon::Pencil => "✎",
            Icon::Clock => "◷",
            Icon::Warning => "!",
            Icon::Window => "□",
            Icon::Check => "✓",
            Icon::YoutubeMusic | Icon::Soundcloud | Icon::Spotify => "●",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// Half-circle ends of a one-row pill.
    Left,
    Right,
}

/// Everything a picture depends on, so it doubles as its cache key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visual {
    Icon { icon: Icon, fg: Rgb, bg: Rgb },
    Round { icon: Icon, fill: Rgb, fg: Rgb, bg: Rgb },
    Corner { corner: Corner, fill: Rgb, bg: Rgb },
    Notes { fill: Rgb, fg: Rgb, bg: Rgb },
    Liked { accent: Rgb, surface: Rgb, bg: Rgb },
    Bars { levels: [u8; 3], color: Rgb, bg: Rgb },
    Slider { filled: u16, knob: bool, played: Rgb, rest: Rgb, bg: Rgb },
    Switch { on: bool, track: Rgb, knob: Rgb, bg: Rgb },
    Swatch { color: Rgb, chosen: bool, ring: Rgb, check: Rgb, bg: Rgb },
}

pub fn draw(visual: Visual, width: u32, height: u32) -> Option<DynamicImage> {
    rasterize(&svg(visual, width as f32, height as f32), width, height)
}

fn rasterize(svg: &str, width: u32, height: u32) -> Option<DynamicImage> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    // Every picture paints its background first, so it is opaque and needs no unpremultiply.
    let rgba = image::RgbaImage::from_raw(width, height, pixmap.take())?;
    Some(DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(rgba).to_rgb8()))
}

/// The icon's paths scaled into a `size` square centered in the picture.
fn icon_group(icon: Icon, fill: Rgb, size: f32, width: f32, height: f32) -> String {
    let shape = icon.shape();
    let scale = size / shape.view_box;
    let (x, y) = ((width - size) / 2.0, (height - size) / 2.0);
    let mut group = format!(r#"<g transform="translate({x} {y}) scale({scale})" fill="{}">"#, hex(fill));
    for path in shape.paths {
        group.push_str(&format!(r#"<path d="{path}"/>"#));
    }
    group.push_str("</g>");
    group
}

fn background(color: Rgb) -> String {
    format!(r#"<rect width="100%" height="100%" fill="{}"/>"#, hex(color))
}

fn svg(visual: Visual, w: f32, h: f32) -> String {
    let side = w.min(h);
    let body = match visual {
        Visual::Icon { icon, fg, bg } => {
            format!("{}{}", background(bg), icon_group(icon, fg, side * ICON_SCALE, w, h))
        }
        Visual::Round { icon, fill, fg, bg } => format!(
            r#"{}<circle cx="{}" cy="{}" r="{}" fill="{}"/>{}"#,
            background(bg),
            w / 2.0,
            h / 2.0,
            side / 2.0,
            hex(fill),
            icon_group(icon, fg, side * ROUND_ICON_SCALE, w, h)
        ),
        Visual::Corner { corner, fill, bg } => {
            let (x, y, rw, rh, radius) = match corner {
                Corner::TopLeft => (0.0, 0.0, w * 3.0, h * 3.0, w),
                Corner::TopRight => (-w * 2.0, 0.0, w * 3.0, h * 3.0, w),
                Corner::BottomLeft => (0.0, -h * 2.0, w * 3.0, h * 3.0, w),
                Corner::BottomRight => (-w * 2.0, -h * 2.0, w * 3.0, h * 3.0, w),
                Corner::Left => (0.0, 0.0, w * 3.0, h, h / 2.0),
                Corner::Right => (-w * 2.0, 0.0, w * 3.0, h, h / 2.0),
            };
            format!(
                r#"{}<rect x="{x}" y="{y}" width="{rw}" height="{rh}" rx="{radius}" fill="{}"/>"#,
                background(bg),
                hex(fill)
            )
        }
        Visual::Notes { fill, fg, bg } => format!(
            r#"{}<rect width="{w}" height="{h}" rx="{COVER_RADIUS}" fill="{}"/>{}"#,
            background(bg),
            hex(fill),
            icon_group(Icon::Notes, fg, side * PLACEHOLDER_ICON_SCALE, w, h)
        ),
        Visual::Liked { accent, surface, bg } => format!(
            r#"{}<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient></defs><rect width="{w}" height="{h}" rx="{COVER_RADIUS}" fill="url(#g)"/>{}"#,
            background(bg),
            hex(mix(accent, [0, 0, 0], 0.15)),
            hex(mix(accent, surface, 0.55)),
            icon_group(Icon::HeartFill, mix([255, 255, 255], accent, 0.08), side * PLACEHOLDER_ICON_SCALE, w, h)
        ),
        Visual::Bars { levels, color, bg } => {
            let bar = w / 5.0;
            let mut bars = background(bg);
            for (i, level) in levels.iter().enumerate() {
                let height = h * 0.15 + h * 0.6 * (*level as f32 / BAR_LEVELS as f32);
                let x = bar * 0.5 + i as f32 * bar * 1.5;
                let y = h * 0.8 - height;
                bars.push_str(&format!(
                    r#"<rect x="{x}" y="{y}" width="{bar}" height="{height}" rx="{}" fill="{}"/>"#,
                    bar / 2.0,
                    hex(color)
                ));
            }
            bars
        }
        Visual::Slider { filled, knob, played, rest, bg } => {
            let span = (w - SLIDER_KNOB * 2.0).max(1.0);
            let end = SLIDER_KNOB + span * (filled as f32 / SLIDER_STEPS as f32);
            let y = (h - SLIDER_TRACK) / 2.0;
            let mut slider = format!(
                r#"{}<rect x="{SLIDER_KNOB}" y="{y}" width="{span}" height="{SLIDER_TRACK}" rx="2" fill="{}"/><rect x="{SLIDER_KNOB}" y="{y}" width="{}" height="{SLIDER_TRACK}" rx="2" fill="{}"/>"#,
                background(bg),
                hex(rest),
                end - SLIDER_KNOB,
                hex(played)
            );
            if knob {
                slider.push_str(&format!(
                    r#"<circle cx="{end}" cy="{}" r="{SLIDER_KNOB}" fill="{}"/>"#,
                    h / 2.0,
                    hex(played)
                ));
            }
            slider
        }
        Visual::Switch { on, track, knob, bg } => {
            let height = h * 0.8;
            let y = (h - height) / 2.0;
            let radius = height / 2.0 - 2.0;
            let cx = if on { w - height / 2.0 } else { height / 2.0 };
            format!(
                r#"{}<rect x="0" y="{y}" width="{w}" height="{height}" rx="{}" fill="{}"/><circle cx="{cx}" cy="{}" r="{radius}" fill="{}"/>"#,
                background(bg),
                height / 2.0,
                hex(track),
                h / 2.0,
                hex(knob)
            )
        }
        Visual::Swatch { color, chosen, ring, check, bg } => {
            let radius = side * 0.42;
            let mut swatch = background(bg);
            if chosen {
                swatch.push_str(&format!(
                    r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
                    w / 2.0,
                    h / 2.0,
                    side / 2.0 - 0.75,
                    hex(ring)
                ));
            }
            let inner = if chosen { radius * 0.82 } else { radius };
            swatch.push_str(&format!(
                r#"<circle cx="{}" cy="{}" r="{inner}" fill="{}"/>"#,
                w / 2.0,
                h / 2.0,
                hex(color)
            ));
            if chosen {
                swatch.push_str(&icon_group(Icon::Check, check, inner * 1.2, w, h));
            }
            swatch
        }
    };
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">{body}</svg>"#)
}

/// Crops a cover to fill `width`×`height` and rounds its corners onto `bg`, as the window does.
pub fn cover(image: &DynamicImage, width: u32, height: u32, bg: Rgb) -> DynamicImage {
    let mut cover: RgbImage = image.resize_to_fill(width, height, FilterType::Triangle).to_rgb8();
    let radius = COVER_RADIUS.min(width as f32 / 2.0).min(height as f32 / 2.0);
    let reach = radius.ceil() as u32;
    for y in 0..reach.min(height) {
        for x in 0..reach.min(width) {
            // Distance from the corner arc's center, mirrored into all four corners.
            let dx = radius - x as f32 - 0.5;
            let dy = radius - y as f32 - 0.5;
            let coverage = (radius - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if coverage >= 1.0 {
                continue;
            }
            for (px, py) in [(x, y), (width - 1 - x, y), (x, height - 1 - y), (width - 1 - x, height - 1 - y)] {
                let pixel = cover.get_pixel_mut(px, py);
                *pixel = Pixel(mix(bg, pixel.0, coverage));
            }
        }
    }
    DynamicImage::ImageRgb8(cover)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: Rgb = [12, 12, 14];
    const FG: Rgb = [237, 237, 239];

    #[test]
    fn every_icon_rasterizes() {
        use Icon::*;
        for icon in [
            Home, HomeFill, Search, SearchBold, Library, Plus, Gear, GearFill, Heart, HeartFill, Pin, Unpin,
            Shuffle, Previous, Next, Play, Pause, Repeat, RepeatOnce, Queue, VolumeHigh, VolumeLow, Muted, Back,
            Forward, Down, Close, More, Enqueue, Notes, NotesPlus, Link, Trash, Pencil, External, Restore, Clock,
            Warning, Window, Check, Telegram, YoutubeMusic, Soundcloud, Spotify,
        ] {
            let image = draw(Visual::Icon { icon, fg: FG, bg: BG }, 20, 20).unwrap().to_rgb8();
            assert!(image.pixels().any(|p| p.0 != BG), "{icon:?} drew nothing");
        }
    }

    #[test]
    fn corners_leave_the_outside_in_the_background() {
        let fill = [40, 40, 46];
        let image = draw(Visual::Corner { corner: Corner::TopLeft, fill, bg: BG }, 10, 20).unwrap().to_rgb8();
        assert_eq!(image.get_pixel(0, 0).0, BG);
        assert_eq!(image.get_pixel(9, 19).0, fill);
    }

    #[test]
    fn covers_get_rounded_corners() {
        let white = DynamicImage::ImageRgb8(RgbImage::from_pixel(50, 50, Pixel([255, 255, 255])));
        let rounded = cover(&white, 40, 20, BG).to_rgb8();
        assert_eq!(rounded.get_pixel(0, 0).0, BG);
        assert_eq!(rounded.get_pixel(20, 10).0, [255, 255, 255]);
    }
}

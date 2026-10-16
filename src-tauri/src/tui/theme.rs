//! The window's palettes (src/app.css) and the icons the terminal can show.

use ratatui::style::Color;

use crate::mode::{self, Theme as ThemeChoice, UiPrefs};
use crate::model::Provider;

pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub raised: Color,
    pub hover: Color,
    pub active: Color,
    pub line: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub danger: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub icons: &'static Icons,
}

pub struct Icons {
    pub home: &'static str,
    pub search: &'static str,
    pub library: &'static str,
    pub settings: &'static str,
    pub liked: &'static str,
    pub add: &'static str,
    pub import: &'static str,
    pub pin: &'static str,
    pub note: &'static str,
    pub album: &'static str,
    pub queue: &'static str,
    pub volume_high: &'static str,
    pub volume_low: &'static str,
    pub muted: &'static str,
    pub warning: &'static str,
    pub trash: &'static str,
    pub rename: &'static str,
    pub link: &'static str,
    pub telegram: &'static str,
    pub window: &'static str,
}

/// Windows Terminal draws color emoji; the classic console gets plain symbols.
const EMOJI: Icons = Icons {
    home: "🏠",
    search: "🔍",
    library: "📚",
    settings: "🔧",
    liked: "💜",
    add: "➕",
    import: "🔗",
    pin: "📌",
    note: "🎵",
    album: "💿",
    queue: "📜",
    volume_high: "🔊",
    volume_low: "🔉",
    muted: "🔇",
    warning: "⚠️",
    trash: "🗑️",
    rename: "✏️",
    link: "🌐",
    telegram: "✈️",
    window: "🪟",
};

const PLAIN: Icons = Icons {
    home: "⌂",
    search: "⌕",
    library: "≡",
    settings: "☼",
    liked: "♥",
    add: "+",
    import: "↗",
    pin: "•",
    note: "♪",
    album: "◎",
    queue: "☰",
    volume_high: "◄)",
    volume_low: "◄",
    muted: "◄×",
    warning: "!",
    trash: "×",
    rename: "✎",
    link: "↗",
    telegram: "➤",
    window: "□",
};

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub fn provider_color(provider: Provider) -> Color {
    match provider {
        Provider::Youtube => rgb(0xff0033),
        Provider::Soundcloud => rgb(0xff5500),
        Provider::Spotify => rgb(0x1ed760),
    }
}

pub fn parse_hex(hex: &str) -> Option<Color> {
    let value = u32::from_str_radix(hex.strip_prefix('#')?, 16).ok()?;
    (hex.len() == 7).then(|| rgb(value))
}

impl Theme {
    pub fn new(prefs: &UiPrefs, rich_terminal: bool) -> Theme {
        let light = match prefs.theme {
            ThemeChoice::Dark => false,
            ThemeChoice::Light => true,
            ThemeChoice::System => system_prefers_light(),
        };
        let accent = parse_hex(mode::accent_hex(&prefs.accent)).unwrap_or(Color::Reset);
        let icons = if rich_terminal { &EMOJI } else { &PLAIN };
        if light {
            Theme {
                bg: rgb(0xe9e9ec),
                surface: rgb(0xf8f8f9),
                raised: rgb(0xffffff),
                hover: rgb(0xededf0),
                active: rgb(0xe2e2e6),
                line: rgb(0xdcdce1),
                text: rgb(0x18181b),
                muted: rgb(0x55555e),
                faint: rgb(0x8a8a93),
                danger: rgb(0xc62828),
                accent,
                on_accent: rgb(0x16110f),
                icons,
            }
        } else {
            Theme {
                bg: rgb(0x0c0c0e),
                surface: rgb(0x151518),
                raised: rgb(0x1c1c20),
                hover: rgb(0x242429),
                active: rgb(0x2d2d33),
                line: rgb(0x26262b),
                text: rgb(0xededef),
                muted: rgb(0xa3a3ab),
                faint: rgb(0x6e6e78),
                danger: rgb(0xf2777a),
                accent,
                on_accent: rgb(0x16110f),
                icons,
            }
        }
    }
}

#[cfg(windows)]
fn system_prefers_light() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value = wide("AppsUseLightTheme");
    let mut data: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: the strings are NUL-terminated and `data` has room for the DWORD asked for.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut size,
        )
    };
    status == 0 && data == 1
}

#[cfg(not(windows))]
fn system_prefers_light() -> bool {
    false
}

/// Hour of the day on the user's clock, for the home greeting.
#[cfg(windows)]
pub fn local_hour() -> u32 {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    // SAFETY: GetLocalTime only fills the struct it is given.
    let mut time: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut time) };
    time.wHour as u32
}

#[cfg(not(windows))]
pub fn local_hour() -> u32 {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ((seconds / 3600) % 24) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_hex("#3fbf8a"), Some(Color::Rgb(0x3f, 0xbf, 0x8a)));
        assert_eq!(parse_hex("3fbf8a"), None);
        assert_eq!(parse_hex("#zzz"), None);
    }
}

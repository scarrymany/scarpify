//! Which interface SCARPIFY starts in, and the settings both interfaces share.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Must match `identifier` in tauri.conf.json: the terminal mode runs without Tauri and
/// rebuilds the same data and cache directories by hand.
pub const APP_IDENTIFIER: &str = "com.scarpify.app";
const PREFS_FILE: &str = "ui.json";
pub const TUI_FLAG: &str = "--tui";
pub const GUI_FLAG: &str = "--gui";
/// Set when a terminal window was opened for us, so the console belongs to our parent.
pub const HOSTED_FLAG: &str = "--hosted";

/// Mirrors `ACCENTS` in src/lib/state/settings.svelte.ts.
pub const ACCENTS: [(&str, &str); 6] = [
    ("ember", "#ec6a45"),
    ("mint", "#3fbf8a"),
    ("azure", "#4f8cf0"),
    ("rose", "#e0577d"),
    ("amber", "#e3a33c"),
    ("iris", "#8b7ff0"),
];

pub fn accent_hex(id: &str) -> &'static str {
    ACCENTS.iter().find(|(name, _)| *name == id).unwrap_or(&ACCENTS[0]).1
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Gui,
    Tui,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

/// The window keeps its settings in webview storage; this file carries them across modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiPrefs {
    pub mode: Mode,
    pub locale: String,
    pub theme: Theme,
    pub accent: String,
    pub animations: bool,
    pub volume: f32,
    pub muted: bool,
    pub audio_device: Option<String>,
    pub discord_presence: bool,
    /// The window's close button hides it in the tray instead of quitting.
    pub close_to_tray: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            mode: Mode::Gui,
            locale: system_locale().into(),
            theme: Theme::Dark,
            accent: ACCENTS[0].0.into(),
            animations: true,
            volume: 0.7,
            muted: false,
            audio_device: None,
            discord_presence: true,
            close_to_tray: true,
        }
    }
}

/// The same directories Tauri resolves as `app_data_dir` and `app_cache_dir`.
#[derive(Clone)]
pub struct AppDirs {
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    pub fn locate() -> Option<Self> {
        let (data, cache) = base_dirs()?;
        Some(Self {
            data: data.join(APP_IDENTIFIER),
            cache: cache.join(APP_IDENTIFIER),
        })
    }

    pub fn create(&self) -> io::Result<()> {
        fs::create_dir_all(&self.data)?;
        fs::create_dir_all(&self.cache)
    }

    /// A missing or unreadable file falls back to defaults: it only holds preferences.
    pub fn load_prefs(&self) -> UiPrefs {
        fs::read(self.data.join(PREFS_FILE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save_prefs(&self, prefs: &UiPrefs) -> io::Result<()> {
        fs::create_dir_all(&self.data)?;
        let json = serde_json::to_vec_pretty(prefs).map_err(io::Error::other)?;
        fs::write(self.data.join(PREFS_FILE), json)
    }
}

/// The window picks its language from the browser locale; this is the same choice.
#[cfg(windows)]
fn system_locale() -> &'static str {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

    const PRIMARY_LANGUAGE_MASK: u16 = 0x3ff;
    const LANG_RUSSIAN: u16 = 0x19;
    const LANG_UKRAINIAN: u16 = 0x22;
    // SAFETY: no arguments; returns the user's display language id.
    match unsafe { GetUserDefaultUILanguage() } & PRIMARY_LANGUAGE_MASK {
        LANG_RUSSIAN => "ru",
        LANG_UKRAINIAN => "uk",
        _ => "en",
    }
}

#[cfg(not(windows))]
fn system_locale() -> &'static str {
    "en"
}

#[cfg(windows)]
fn base_dirs() -> Option<(PathBuf, PathBuf)> {
    let data = std::env::var_os("APPDATA")?;
    let cache = std::env::var_os("LOCALAPPDATA")?;
    Some((data.into(), cache.into()))
}

#[cfg(not(windows))]
fn base_dirs() -> Option<(PathBuf, PathBuf)> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let from_env = |name: &str, fallback: &str| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(fallback))
    };
    Some((from_env("XDG_DATA_HOME", ".local/share"), from_env("XDG_CACHE_HOME", ".cache")))
}

pub fn has_flag(flag: &str) -> bool {
    std::env::args().skip(1).any(|arg| arg == flag)
}

/// A command-line flag wins over the remembered mode.
pub fn requested(args: impl IntoIterator<Item = String>, prefs: &UiPrefs) -> Mode {
    for arg in args {
        match arg.as_str() {
            TUI_FLAG => return Mode::Tui,
            GUI_FLAG => return Mode::Gui,
            _ => {}
        }
    }
    prefs.mode
}

/// Opens the window in a new process; the terminal exits right after.
pub fn relaunch_gui() -> io::Result<()> {
    Command::new(std::env::current_exe()?).arg(GUI_FLAG).spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_matches_tauri_config() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["identifier"], APP_IDENTIFIER);
    }

    #[test]
    fn accents_match_the_window() {
        let source = include_str!("../../src/lib/state/settings.svelte.ts");
        for (id, color) in ACCENTS {
            assert!(
                source.contains(&format!(r#"{{ id: "{id}", color: "{color}" }}"#)),
                "accent {id} {color} differs from settings.svelte.ts"
            );
        }
    }

    #[test]
    fn flags_override_remembered_mode() {
        let tui = UiPrefs { mode: Mode::Tui, ..UiPrefs::default() };
        let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        assert_eq!(requested(args(&["scarpify"]), &tui), Mode::Tui);
        assert_eq!(requested(args(&["scarpify", "--gui"]), &tui), Mode::Gui);
        assert_eq!(requested(args(&["scarpify", "--tui"]), &UiPrefs::default()), Mode::Tui);
    }

    #[test]
    fn partial_prefs_keep_defaults() {
        let prefs: UiPrefs = serde_json::from_str(r#"{"mode":"tui","volume":0.4}"#).unwrap();
        assert_eq!(prefs.mode, Mode::Tui);
        assert_eq!(prefs.volume, 0.4);
        assert_eq!(prefs.accent, UiPrefs::default().accent);
        assert!(["en", "ru", "uk"].contains(&prefs.locale.as_str()));
    }
}

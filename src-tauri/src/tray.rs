//! The tray icon. The close button can hide the window there instead of quitting, so music
//! keeps playing; the icon brings it back, and its menu quits for real.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Deserialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

const EN: &str = include_str!("../../src/lib/i18n/en.json");
const RU: &str = include_str!("../../src/lib/i18n/ru.json");
const UK: &str = include_str!("../../src/lib/i18n/uk.json");

const MAIN_WINDOW: &str = "main";
const TRAY_ID: &str = "main";
const TOOLTIP: &str = "SCARPIFY";
const OPEN_ITEM: &str = "open";
const TOGGLE_ITEM: &str = "toggle";
const QUIT_ITEM: &str = "quit";
/// Asks the window to play or pause; it owns the player state.
pub const TOGGLE_EVENT: &str = "tray-toggle";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Labels {
    open: String,
    play_pause: String,
    quit: String,
}

#[derive(Deserialize)]
struct Dictionary {
    tray: Labels,
}

fn labels(locale: &str) -> Labels {
    let source = match locale {
        "ru" => RU,
        "uk" => UK,
        _ => EN,
    };
    // The dictionaries are compiled in and covered by tests, so this cannot fail at runtime.
    serde_json::from_str::<Dictionary>(source).expect("bundled dictionary is valid").tray
}

pub struct Tray {
    close_to_tray: AtomicBool,
    open: MenuItem<Wry>,
    toggle: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

impl Tray {
    pub fn create(app: &AppHandle, locale: &str, close_to_tray: bool) -> tauri::Result<Tray> {
        let text = labels(locale);
        let open = MenuItem::with_id(app, OPEN_ITEM, &text.open, true, None::<&str>)?;
        let toggle = MenuItem::with_id(app, TOGGLE_ITEM, &text.play_pause, true, None::<&str>)?;
        let quit = MenuItem::with_id(app, QUIT_ITEM, &text.quit, true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&open, &toggle, &PredefinedMenuItem::separator(app)?, &quit])?;

        let mut builder = TrayIconBuilder::with_id(TRAY_ID)
            .tooltip(TOOLTIP)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                OPEN_ITEM => show(app),
                TOGGLE_ITEM => {
                    let _ = app.emit(TOGGLE_EVENT, ());
                }
                QUIT_ITEM => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    show(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app)?;

        Ok(Tray { close_to_tray: AtomicBool::new(close_to_tray), open, toggle, quit })
    }

    pub fn closes_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }

    /// Follows the window's settings: its language and whether closing hides it.
    pub fn update(&self, locale: &str, close_to_tray: bool) {
        self.close_to_tray.store(close_to_tray, Ordering::Relaxed);
        let text = labels(locale);
        // A label that fails to update only stays in the previous language.
        let _ = self.open.set_text(text.open);
        let _ = self.toggle.set_text(text.play_pause);
        let _ = self.quit.set_text(text.quit);
    }
}

/// Brings the window back from the tray, or to the front when a second copy was started.
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_tray_labels() {
        for locale in ["en", "ru", "uk"] {
            let text = labels(locale);
            assert!(!text.open.is_empty() && !text.play_pause.is_empty() && !text.quit.is_empty());
        }
    }
}

//! A terminal interface that mirrors the window and plays through the same engine, sources
//! and library, without starting Tauri or WebView2, so it needs a fraction of the memory.
//! It is driven with the mouse, like the window: Windows Terminal reports clicks, drags,
//! hovering and the wheel.

mod app;
mod art;
mod console;
mod i18n;
mod queue;
mod theme;
mod ui;

use std::error::Error;
use std::io::{self, stdout};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::DefaultTerminal;

use app::{App, Exit, Hits, Message, Services, Status};
use art::Art;
use console::Console;
use crate::audio::engine::AudioEngine;
use crate::library::Library;
use crate::mode::{self, AppDirs, Mode, UiPrefs};
use crate::presence::Presence;
use crate::sources::Sources;

/// Also bounds how late a finished search or a player event reaches the screen.
const POLL_INTERVAL: Duration = Duration::from_millis(100);
/// Pace of the "now playing" bars.
const BEAT: Duration = Duration::from_millis(180);
/// Windows Terminal sets this for everything it hosts.
const WINDOWS_TERMINAL_ENV: &str = "WT_SESSION";

pub fn run(dirs: &AppDirs, prefs: UiPrefs) -> Result<(), Box<dyn Error>> {
    dirs.create()?;
    if let Console::Relaunched = console::prepare(dirs, mode::accent_hex(&prefs.accent))? {
        return Ok(());
    }
    let rich_terminal = std::env::var_os(WINDOWS_TERMINAL_ENV).is_some();

    let (tx, rx) = mpsc::channel();
    let http = crate::http_client()?;
    let services = Arc::new(start_services(dirs, &prefs, tx.clone(), http.clone())?);
    let mut app = App::new(services, tx, Arc::new(dirs.clone()), prefs, rich_terminal)?;

    let mut terminal = ratatui::try_init()?;
    execute!(stdout(), EnableMouseCapture)?;
    let mut art = Art::new(http, rich_terminal);
    let result = event_loop(&mut terminal, &mut app, &mut art, &rx);
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
    app.shutdown();
    result?;
    Ok(())
}

fn start_services(
    dirs: &AppDirs,
    prefs: &UiPrefs,
    tx: mpsc::Sender<Message>,
    http: reqwest::Client,
) -> Result<Services, Box<dyn Error>> {
    let engine = AudioEngine::spawn(http.clone(), move |event| {
        let _ = tx.send(Message::Player(event));
    });
    engine.set_volume(if prefs.muted { 0.0 } else { prefs.volume });
    if prefs.audio_device.is_some() {
        // A missing device is not fatal: the engine falls back to the system default.
        let _ = tauri::async_runtime::block_on(engine.set_output_device(prefs.audio_device.clone()));
    }

    let presence = Presence::spawn();
    presence.set_enabled(prefs.discord_presence);

    let sources = Sources::new(http, dirs.cache.clone())?;
    Ok(Services {
        sources,
        engine,
        library: Library::open(&dirs.data.join(crate::LIBRARY_FILE))?,
        presence,
    })
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, art: &mut Art, rx: &Receiver<Message>) -> io::Result<()> {
    let mut hits = Hits::default();
    let mut dirty = true;
    let mut beat = 0u64;
    let mut last_beat = Instant::now();

    loop {
        if app.repaint {
            // Pictures are only re-sent with a full redraw, e.g. after a menu covered one.
            terminal.clear()?;
            app.repaint = false;
            dirty = true;
        }
        if dirty {
            terminal.draw(|frame| ui::render(frame, app, art, &mut hits, beat))?;
            dirty = false;
        }

        let animating = app.status == Status::Playing && app.prefs().animations;
        let timeout = if animating { BEAT.saturating_sub(last_beat.elapsed()) } else { POLL_INTERVAL };
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
                Event::Mouse(mouse) => app.on_mouse(mouse, &hits),
                Event::Paste(text) => app.on_paste(&text),
                Event::Resize(..) => app.repaint = true,
                _ => {}
            }
            dirty = true;
        }
        while let Ok(message) = rx.try_recv() {
            app.on_message(message);
            dirty = true;
        }
        dirty |= art.drain();
        dirty |= app.tick();
        if animating && last_beat.elapsed() >= BEAT {
            beat += 1;
            last_beat = Instant::now();
            dirty = true;
        }

        match app.exit.take() {
            None => {}
            Some(Exit::Quit) => {
                app.save_prefs();
                return Ok(());
            }
            Some(Exit::Window) => {
                // The window reads the mode on start, so it must be saved before launching it.
                app.remember_mode(Mode::Gui);
                match mode::relaunch_gui() {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        app.remember_mode(Mode::Tui);
                        app.toast(err.to_string(), true);
                        dirty = true;
                    }
                }
            }
        }
    }
}

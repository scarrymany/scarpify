mod audio;
mod error;
mod library;
mod mode;
mod model;
mod presence;
mod sources;
#[cfg(windows)]
mod terminal_host;
#[cfg(desktop)]
mod tray;
#[cfg(desktop)]
mod tui;

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

use audio::devices::{self, OutputDevice};
use audio::engine::{AudioEngine, LoadRequest};
use error::{Error, Result};
use library::{LegacyLibrary, Library, SavedCollection, Snapshot};
use mode::{AppDirs, Mode, UiPrefs};
use model::{ProviderResult, Track};
use presence::{NowPlaying, Presence};
use sources::Sources;

const PLAYER_EVENT: &str = "player";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const LIBRARY_FILE: &str = "library.db";

struct AppState {
    sources: Sources,
    engine: Arc<AudioEngine>,
    presence: Presence,
    library: Library,
}

#[tauri::command]
async fn search_youtube(query: String, state: State<'_, AppState>) -> Result<ProviderResult> {
    Ok(state.sources.search_youtube(query.trim()).await)
}

#[tauri::command]
async fn search_soundcloud(query: String, state: State<'_, AppState>) -> Result<ProviderResult> {
    Ok(state.sources.search_soundcloud(query.trim()).await)
}

/// Resolves a stream in advance; a failure surfaces later, when the track is actually played.
#[tauri::command]
async fn prefetch(track: Track, state: State<'_, AppState>) -> Result<()> {
    let _ = state.sources.stream(&track).await;
    Ok(())
}

#[tauri::command]
async fn import_playlist(link: String, state: State<'_, AppState>) -> Result<SavedCollection> {
    let link = link.trim();
    let collection = state.sources.import(link).await?;
    state.library.save_import(&collection, link)
}

#[tauri::command]
async fn play(track: Track, state: State<'_, AppState>) -> Result<()> {
    play_track(&state.sources, &state.engine, &track).await
}

async fn play_track(sources: &Sources, engine: &AudioEngine, track: &Track) -> Result<()> {
    match start_playback(sources, engine, track).await {
        // A cached URL can expire or be refused by the CDN; one fresh resolution fixes that.
        Err(err) if !matches!(err, Error::Cancelled) => {
            sources.forget_stream(track);
            start_playback(sources, engine, track).await
        }
        result => result,
    }
}

async fn start_playback(sources: &Sources, engine: &AudioEngine, track: &Track) -> Result<()> {
    let stream = sources.stream(track).await?;
    engine
        .load(LoadRequest {
            source: stream.source,
            format_hint: stream.format_hint,
            gain: stream.gain,
        })
        .await
}

/// `None` hides the activity (paused or stopped), mirroring Spotify's behaviour.
#[tauri::command]
fn update_presence(now_playing: Option<NowPlaying>, state: State<'_, AppState>) {
    match now_playing {
        Some(now_playing) => state.presence.show(now_playing),
        None => state.presence.hide(),
    }
}

#[tauri::command]
fn set_presence_enabled(enabled: bool, state: State<'_, AppState>) {
    state.presence.set_enabled(enabled);
}

#[tauri::command]
fn pause(state: State<'_, AppState>) {
    state.engine.pause();
}

#[tauri::command]
fn resume(state: State<'_, AppState>) {
    state.engine.resume();
}

#[tauri::command]
fn stop(state: State<'_, AppState>) {
    state.engine.stop();
}

#[tauri::command]
fn seek(position_ms: u64, state: State<'_, AppState>) {
    state.engine.seek(Duration::from_millis(position_ms));
}

#[tauri::command]
fn set_volume(volume: f32, state: State<'_, AppState>) {
    state.engine.set_volume(volume);
}

#[tauri::command]
async fn audio_devices() -> Vec<OutputDevice> {
    devices::list()
}

/// `None` follows the system default output.
#[tauri::command]
async fn set_audio_device(device: Option<String>, state: State<'_, AppState>) -> Result<()> {
    state.engine.set_output_device(device).await
}

// Library commands are async so SQLite writes never run on the window's main thread.

#[tauri::command]
async fn library_snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    state.library.snapshot()
}

#[tauri::command]
async fn library_import_legacy(legacy: LegacyLibrary, state: State<'_, AppState>) -> Result<Snapshot> {
    state.library.import_legacy(&legacy)?;
    state.library.snapshot()
}

#[tauri::command]
async fn library_set_liked(track: Track, liked: bool, state: State<'_, AppState>) -> Result<()> {
    state.library.set_liked(&track, liked)
}

#[tauri::command]
async fn library_mark_played(track: Track, state: State<'_, AppState>) -> Result<()> {
    state.library.mark_played(&track)
}

#[tauri::command]
async fn playlist_create(name: String, state: State<'_, AppState>) -> Result<SavedCollection> {
    state.library.create_playlist(name.trim())
}

#[tauri::command]
async fn playlist_rename(id: String, name: String, state: State<'_, AppState>) -> Result<()> {
    state.library.rename(&id, name.trim())
}

#[tauri::command]
async fn playlist_add_tracks(id: String, tracks: Vec<Track>, state: State<'_, AppState>) -> Result<SavedCollection> {
    state.library.add_tracks(&id, &tracks)
}

#[tauri::command]
async fn playlist_remove_track(id: String, track: Track, state: State<'_, AppState>) -> Result<SavedCollection> {
    state.library.remove_track(&id, &track)
}

#[tauri::command]
async fn playlist_move_track(id: String, from: usize, to: usize, state: State<'_, AppState>) -> Result<SavedCollection> {
    state.library.move_track(&id, from, to)
}

#[tauri::command]
async fn collection_set_cover(id: String, cover: Option<String>, state: State<'_, AppState>) -> Result<()> {
    state.library.set_cover(&id, cover.as_deref())
}

#[tauri::command]
async fn collection_set_pinned(id: String, pinned: bool, state: State<'_, AppState>) -> Result<()> {
    state.library.set_pinned(&id, pinned)
}

#[tauri::command]
async fn collection_delete(id: String, state: State<'_, AppState>) -> Result<()> {
    state.library.delete(&id)
}

#[tauri::command]
async fn collections_reorder(ids: Vec<String>, state: State<'_, AppState>) -> Result<()> {
    state.library.reorder_collections(&ids)
}

fn app_dirs(app: &AppHandle) -> Result<AppDirs> {
    let unavailable = |e: tauri::Error| Error::Unavailable(e.to_string());
    Ok(AppDirs {
        data: app.path().app_data_dir().map_err(unavailable)?,
        cache: app.path().app_cache_dir().map_err(unavailable)?,
    })
}

/// Hands over to the terminal mode in Windows Terminal; `false` means none is available yet.
#[cfg(windows)]
#[tauri::command]
async fn open_terminal_mode(prefs: UiPrefs, app: AppHandle) -> Result<bool> {
    let dirs = app_dirs(&app)?;
    let Some(terminal) = terminal_host::find(&dirs.cache) else { return Ok(false) };

    let io_error = |e: std::io::Error| Error::Unavailable(e.to_string());
    let prefs = UiPrefs { mode: Mode::Tui, ..prefs };
    dirs.save_prefs(&prefs).map_err(io_error)?;
    let executable = std::env::current_exe().map_err(io_error)?;
    terminal_host::launch(&terminal, &executable).map_err(io_error)?;
    app.exit(0);
    Ok(true)
}

/// Downloads Microsoft's portable Windows Terminal for systems that do not have one.
#[cfg(windows)]
#[tauri::command]
async fn install_terminal(app: AppHandle) -> Result<()> {
    let dirs = app_dirs(&app)?;
    let http = http_client()?;
    terminal_host::install(&http, &dirs.cache)
        .await
        .map(drop)
        .map_err(|e| Error::Unavailable(e.to_string()))
}

/// Keeps the shared settings file current, so the terminal mode always starts with them.
#[tauri::command]
async fn sync_terminal_prefs(prefs: UiPrefs, app: AppHandle) -> Result<()> {
    #[cfg(desktop)]
    app.state::<tray::Tray>().update(&prefs.locale, prefs.close_to_tray);
    let prefs = UiPrefs { mode: Mode::Gui, ..prefs };
    app_dirs(&app)?
        .save_prefs(&prefs)
        .map_err(|e| Error::Unavailable(e.to_string()))
}

/// Settings changed in the terminal, handed to the window it switched back to.
#[tauri::command]
fn terminal_handoff(app: AppHandle) -> Result<Option<UiPrefs>> {
    if !mode::has_flag(mode::GUI_FLAG) {
        return Ok(None);
    }
    Ok(Some(app_dirs(&app)?.load_prefs()))
}

fn http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .gzip(true)
        .build()
}

fn init_state(app: &AppHandle) -> std::result::Result<AppState, Box<dyn std::error::Error>> {
    let http = http_client()?;
    let cache_dir = app.path().app_cache_dir()?;
    std::fs::create_dir_all(&cache_dir)?;
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;

    let emitter = app.clone();
    let engine = AudioEngine::spawn(http.clone(), move |event| {
        let _ = emitter.emit(PLAYER_EVENT, event);
    });

    Ok(AppState {
        sources: Sources::new(http, cache_dir)?,
        engine,
        presence: Presence::spawn(),
        library: Library::open(&data_dir.join(LIBRARY_FILE))?,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(desktop)]
    run_terminal_if_requested();

    let builder = tauri::Builder::default();
    // Registered first: a second launch only brings the running window back, e.g. from the tray.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show(app)));

    builder
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = init_state(app.handle())?;
            app.manage(state);

            #[cfg(desktop)]
            {
                let prefs = app_dirs(app.handle())?.load_prefs();
                let tray = tray::Tray::create(app.handle(), &prefs.locale, prefs.close_to_tray)?;
                app.manage(tray);
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                handle.state::<AppState>().sources.warm_up().await;
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.app_handle().state::<tray::Tray>().closes_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            search_youtube,
            search_soundcloud,
            prefetch,
            import_playlist,
            play,
            pause,
            resume,
            stop,
            seek,
            set_volume,
            audio_devices,
            set_audio_device,
            update_presence,
            set_presence_enabled,
            library_snapshot,
            library_import_legacy,
            library_set_liked,
            library_mark_played,
            playlist_create,
            playlist_rename,
            playlist_add_tracks,
            playlist_remove_track,
            playlist_move_track,
            collection_set_cover,
            collection_set_pinned,
            collection_delete,
            collections_reorder,
            #[cfg(windows)]
            open_terminal_mode,
            #[cfg(windows)]
            install_terminal,
            terminal_handoff,
            sync_terminal_prefs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Runs the terminal interface when it is asked for; returns only if the window should open.
#[cfg(desktop)]
fn run_terminal_if_requested() {
    let Some(dirs) = AppDirs::locate() else { return };
    let prefs = dirs.load_prefs();
    if mode::requested(std::env::args().skip(1), &prefs) != Mode::Tui {
        if prefs.mode == Mode::Tui {
            // `--gui` was passed explicitly; the next plain launch opens the window too.
            let _ = dirs.save_prefs(&UiPrefs { mode: Mode::Gui, ..prefs });
        }
        return;
    }

    match tui::run(&dirs, prefs.clone()) {
        Ok(()) => std::process::exit(0),
        // A terminal that cannot start must not lock the user out: fall back to the window.
        Err(err) => {
            eprintln!("terminal mode failed: {err}");
            let _ = dirs.save_prefs(&UiPrefs { mode: Mode::Gui, ..prefs });
        }
    }
}

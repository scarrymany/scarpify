mod audio;
mod error;
mod library;
mod model;
mod presence;
mod sources;

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};

use audio::devices::{self, OutputDevice};
use audio::engine::{AudioEngine, LoadRequest};
use error::{Error, Result};
use library::{LegacyLibrary, Library, SavedCollection, Snapshot};
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
    match start_playback(&state, &track).await {
        // A cached URL can expire or be refused by the CDN; one fresh resolution fixes that.
        Err(err) if !matches!(err, Error::Cancelled) => {
            state.sources.forget_stream(&track);
            start_playback(&state, &track).await
        }
        result => result,
    }
}

async fn start_playback(state: &AppState, track: &Track) -> Result<()> {
    let stream = state.sources.stream(track).await?;
    state
        .engine
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

fn init_state(app: &AppHandle) -> std::result::Result<AppState, Box<dyn std::error::Error>> {
    let http = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .gzip(true)
        .build()?;
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let state = init_state(app.handle())?;
            app.manage(state);

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                handle.state::<AppState>().sources.warm_up().await;
            });
            Ok(())
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
            collections_reorder
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

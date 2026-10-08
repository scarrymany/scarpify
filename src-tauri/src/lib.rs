mod audio;
mod error;
mod model;
mod presence;
mod sources;

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};

use audio::engine::{AudioEngine, LoadRequest};
use error::Result;
use model::{Collection, ProviderResult, Track};
use presence::{NowPlaying, Presence};
use sources::Sources;

const PLAYER_EVENT: &str = "player";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

struct AppState {
    sources: Sources,
    engine: AudioEngine,
    presence: Presence,
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
async fn import_playlist(link: String, state: State<'_, AppState>) -> Result<Collection> {
    state.sources.import(&link).await
}

#[tauri::command]
async fn play(track: Track, state: State<'_, AppState>) -> Result<()> {
    let stream = state.sources.stream(&track).await?;
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

fn init_state(app: &AppHandle) -> std::result::Result<AppState, Box<dyn std::error::Error>> {
    let http = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .gzip(true)
        .build()?;
    let cache_dir = app.path().app_cache_dir()?;
    std::fs::create_dir_all(&cache_dir)?;

    let emitter = app.clone();
    let engine = AudioEngine::spawn(http.clone(), move |event| {
        let _ = emitter.emit(PLAYER_EVENT, event);
    });

    Ok(AppState {
        sources: Sources::new(http, cache_dir)?,
        engine,
        presence: Presence::spawn(),
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
            update_presence,
            set_presence_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

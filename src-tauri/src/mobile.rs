//! Android media lifecycle. Controls travel through a native channel, so pause/focus
//! loss does not depend on JavaScript timers while the WebView is backgrounded.
use std::sync::Arc;
use serde::Serialize;
use tauri::{ipc::Channel, plugin::{Builder, PluginHandle, TauriPlugin}, AppHandle, Emitter, Manager, Wry};
use crate::{audio::engine::{AudioEngine, PlayerEvent}, error::{Error, Result}, model::Track};

pub struct MobilePlayback(PluginHandle<Wry>);

pub fn init() -> TauriPlugin<Wry> {
    Builder::new("mobilePlayback").setup(|app, api| {
        let handle = api.register_android_plugin("com.scarpify.app", "PlaybackPlugin")?;
        app.manage(MobilePlayback(handle));
        Ok(())
    }).build()
}

#[derive(Serialize)]
struct Configure { channel: Channel<String> }

pub async fn configure(app: &AppHandle, engine: Arc<AudioEngine>) -> Result<()> {
    let emitter = app.clone();
    let channel = Channel::new(move |message: tauri::ipc::InvokeResponseBody| {
        let action: String = message.deserialize()?;
        match action.as_str() {
            "pause" => engine.pause(),
            "resume" => engine.resume(),
            "stop" => engine.stop(),
            _ => { let _ = emitter.emit("media-control", action); }
        }
        Ok(())
    });
    app.state::<MobilePlayback>().0.run_mobile_plugin_async::<()>("configure", Configure { channel })
        .await.map_err(|e| Error::Output(e.to_string()))
}

pub async fn prepare(app: &AppHandle, track: &Track) -> Result<()> {
    app.state::<MobilePlayback>().0.run_mobile_plugin_async::<()>("prepare", serde_json::json!({
        "title": track.title, "artist": track.artists.join(", "), "durationMs": track.duration_ms
    })).await.map_err(|e| Error::Output(e.to_string()))
}

pub fn event(app: &AppHandle, event: PlayerEvent) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.state::<MobilePlayback>().0.run_mobile_plugin_async::<()>("update", event).await;
    });
}

pub async fn resume(app: &AppHandle) -> Result<()> {
    app.state::<MobilePlayback>().0.run_mobile_plugin_async::<()>("resume", serde_json::json!({}))
        .await.map_err(|e| Error::Output(e.to_string()))
}

pub async fn background(app: &AppHandle) -> Result<()> {
    app.state::<MobilePlayback>().0.run_mobile_plugin_async::<()>("background", serde_json::json!({}))
        .await.map_err(|e| Error::Output(e.to_string()))
}

//! Android does not expose the desktop Discord IPC socket. Commands remain wire-compatible;
//! the capability response lets the shared UI explain this platform limitation.
pub use crate::model::NowPlaying;
pub struct Presence;
impl Presence {
    pub fn spawn() -> Self { Self }
    pub fn show(&self, _: NowPlaying) {}
    pub fn hide(&self) {}
    pub fn set_enabled(&self, _: bool) {}
}

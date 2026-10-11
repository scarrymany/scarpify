# Linux

SCARPIFY shares the Windows frontend, SQLite library, providers, streaming decoder and
player. Tauri uses WebKitGTK 4.1 and GTK3; rodio/CPAL uses ALSA. With desktop ALSA
plugins installed this also works through PipeWire/PulseAudio. Device selection is
available in Settings; the default output follows the system.

## Build

Use Node 22.12+ (or 24 LTS), stable Rust and the
[Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux).
On Debian 12 / Ubuntu 22.04+ a development machine needs:

```sh
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libasound2-dev \
  libayatana-appindicator3-dev librsvg2-dev libssl-dev libdbus-1-dev patchelf
npm ci
npm run check
npm test
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri build -- --bundles deb,appimage
```

Fedora additionally supports `--bundles rpm` with the corresponding Tauri development
packages and `alsa-lib-devel`. Build distribution binaries on the oldest supported
host (CI uses Ubuntu 22.04) to avoid requiring a newer glibc on users' systems.
`tauri.linux.conf.json` automatically overrides Windows installer targets. No source
edits or Windows toolchain are required. `npm run tauri dev` runs the native app.

## Install and update

Install the `.deb` from `src-tauri/target/release/bundle/deb` with
`sudo apt install ./SCARPIFY_*.deb`, or the `.rpm` with `sudo dnf install ./SCARPIFY-*.rpm`.
For AppImage: mark it executable and launch it. Systems without FUSE2 can run
`./SCARPIFY*.AppImage --appimage-extract-and-run`.
Install a newer package in place to update; do not delete the app data directory.
No automatic updater or unsigned remote code download is introduced.

Tauri stores `library.db` in the user's application data directory (normally
`$XDG_DATA_HOME/com.scarpify.app`, falling back to `~/.local/share/com.scarpify.app`),
with caches under the corresponding XDG cache directory. No root privileges are
needed to launch or save a library. Cover selection uses the WebView's native file
picker; the cover is saved inside the database, not as an arbitrary filesystem path.

## Displays and diagnostics

GTK selects the session backend. Test both `GDK_BACKEND=x11` and
`GDK_BACKEND=wayland` inside their respective display sessions. A headless Xvfb test
proves X11 startup only, not Wayland or audible output on physical hardware.
Do not globally force X11 or disable WebKit's sandbox. Discord presence requires
a local desktop Discord IPC socket; sandboxed Discord packages may hide that socket.

Playback needs an available output device; missing or busy output is reported as a
player error and the library remains usable. Verify headphone hotplug, manual device
changes and permissions on the target desktop. Linux dialogs follow the installed
WebKitGTK and desktop portal/file-picker integration.

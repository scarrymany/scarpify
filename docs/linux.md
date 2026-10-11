# Linux build and installation

SCARPIFY uses the same Rust playback, SQLite library and Svelte UI on Linux.
Target packages are `.deb`, `.rpm` and `.AppImage`; Windows retains NSIS through
Tauri's platform defaults. Build on the oldest supported distribution rather than
assuming binaries built against a newer glibc will work on older hosts.

## Ubuntu 24.04 / Debian-family build host

Install Node 22.22.2+ and stable Rust, then:

```sh
sudo apt update
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev \
  libasound2-dev libssl-dev librsvg2-dev patchelf libayatana-appindicator3-dev
npm ci
npm run tauri dev
npm run tauri build
```

Tauri automatically merges `src-tauri/tauri.linux.conf.json`. Packages are under
`src-tauri/target/release/bundle`. The explicit DEB dependencies target Ubuntu
24.04's `libasound2t64`; older Debian/Ubuntu versions need their corresponding
ALSA package and a build against that distribution. RPM/AppImage dependency
availability must be validated on their actual target distributions.

```sh
sudo apt install ./SCARPIFY_*.deb
# Or make the AppImage executable and launch it:
chmod +x SCARPIFY_*.AppImage
./SCARPIFY_*.AppImage
```

Install a newer package with the same package manager to update. There is no
automatic updater in the existing application. AppImage users replace the binary.
Do not delete the app-data directory when updating: the existing SQLite library
is kept under the XDG app-data directory for `com.scarpify.app`, and provider
caches under the corresponding XDG cache directory.

Audio uses ALSA through CPAL. Ensure the distribution's PipeWire/PulseAudio ALSA
compatibility layer is installed for desktop audio routing. The output picker
uses platform device IDs; hotplug must be checked with actual audio hardware.
Custom covers use the system WebKit file picker and are stored in the library.
Discord presence requires the desktop Discord IPC socket to be accessible.

## Display smoke matrix

Validate both `GDK_BACKEND=x11` and `GDK_BACKEND=wayland` in real logged-in desktop
sessions. Check startup, close/minimize/maximize, search entry, file picker,
play/pause/seek/queue, headphone switch, SQLite persistence and package upgrade.
Xvfb can establish X11 startup only; it cannot establish Wayland compatibility or
audible playback. Do not disable the WebKit sandbox to make checks pass.

No Linux runtime or package build is claimed by this packaging change alone.

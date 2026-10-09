<p align="center">
  <img src="assets/icon.svg" width="88" alt="SCARPIFY logo">
</p>

<h1 align="center">SCARPIFY</h1>

<p align="center">
  An open-source desktop music player for YouTube Music and SoundCloud.<br>
  No ads, no accounts, one library.
</p>

<p align="center">
  <a href="https://github.com/scarrymany/scarpify/releases/latest"><img src="https://img.shields.io/github/v/release/scarrymany/scarpify?color=ec6a45&label=release" alt="Latest release"></a>
  <a href="https://github.com/scarrymany/scarpify/releases"><img src="https://img.shields.io/github/downloads/scarrymany/scarpify/total?color=ec6a45" alt="Downloads"></a>
  <a href="https://github.com/scarrymany/scarpify/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/scarrymany/scarpify/ci.yml?label=ci" alt="CI status"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-555" alt="Windows 10 and 11">
  <a href="LICENSE"><img src="https://img.shields.io/github/license/scarrymany/scarpify?color=555" alt="GPL-3.0 license"></a>
</p>

<p align="center">
  <a href="https://github.com/scarrymany/scarpify/releases/latest"><b>Download for Windows</b></a> ·
  <a href="README.ru.md">Русский</a>
</p>

<p align="center">
  <img src="docs/screenshots/playlist.png" alt="SCARPIFY playing a playlist imported from Spotify">
</p>

## Why SCARPIFY

Music lives on several services, and each of them wants an account, a subscription or your attention for ads. SCARPIFY puts YouTube Music and SoundCloud behind one calm interface, lets you bring your Spotify playlists along by link, and keeps everything in a library on your own computer.

## Features

**Listening**
- Search YouTube Music and SoundCloud at once. Every track shows the service it comes from.
- Native playback in Rust that starts while the track is still downloading, with volume normalization from YouTube loudness data.
- Queue you can reorder by dragging, shuffle and repeat, keyboard shortcuts.
- Pick the output device, or follow the Windows default when headphones are plugged in. Playback moves between devices without losing its place.

**Library**
- Your own playlists: create, rename in place, add tracks from the right-click menu, drag to reorder, set a custom cover.
- Import public playlists and albums from Spotify, SoundCloud and YouTube Music by link. Spotify tracks are matched and played through YouTube Music, without a Spotify account.
- Liked tracks, recently played, pinned playlists. Everything is stored locally in SQLite.

**Interface**
- Dark, light and system themes, six accent colors, animations that can be turned off.
- English, Russian and Ukrainian.
- "Listening to SCARPIFY" in your Discord status, with cover and progress.

<table>
  <tr>
    <td><img src="docs/screenshots/home.png" alt="Home screen"></td>
    <td><img src="docs/screenshots/search.png" alt="Search across YouTube Music and SoundCloud"></td>
  </tr>
  <tr>
    <td><img src="docs/screenshots/playlist-menu.png" alt="Own playlist with the track menu"></td>
    <td><img src="docs/screenshots/settings-light.png" alt="Settings in the light theme"></td>
  </tr>
</table>

## Install

Download `SCARPIFY_x.y.z_x64-setup.exe` from the [latest release](https://github.com/scarrymany/scarpify/releases/latest) and run it. The installer does not need administrator rights and lets you pick English, Russian or Ukrainian.

Windows 10 and 11 (x64) are supported. WebView2 is installed automatically when it is missing.

The installer is not code-signed yet, so Windows SmartScreen may ask for confirmation: choose **More info**, then **Run anyway**.

## Keyboard shortcuts

| Action | Keys |
| --- | --- |
| Play / pause | `Space` |
| Next / previous track | `Ctrl + →` / `Ctrl + ←` |
| Search | `Ctrl + L`, `Ctrl + F` or `/` |
| Back / forward | `Alt + ←` / `Alt + →`, mouse side buttons |

## How it works

| Part | Implementation |
| --- | --- |
| Shell | [Tauri 2](https://tauri.app), Rust backend and a [Svelte 5](https://svelte.dev) interface |
| Audio | [rodio](https://github.com/RustAudio/rodio) + [Symphonia](https://github.com/pdeljanov/Symphonia), streamed into memory and played while downloading |
| Library | SQLite through [rusqlite](https://github.com/rusqlite/rusqlite), in the app data folder |
| YouTube Music | search through [RustyPipe](https://codeberg.org/ThetaDev/rustypipe), streams through the InnerTube player API |
| SoundCloud | public `api-v2` with the web client id |
| Spotify | public embed and oEmbed pages, no login and no audio from Spotify |
| Discord | Rich Presence over local IPC |

YouTube and SoundCloud change their internal APIs from time to time. When playback from one of them breaks, the fix is usually a small change in `src-tauri/src/sources`.

## Building from source

Requirements: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```bash
npm install
npm run tauri dev      # development build with hot reload
npm run tauri build    # installer in src-tauri/target/release/bundle
```

Tests:

```bash
npm test                            # interface tests (Vitest + Testing Library)
npm run test:perf                   # smoothness and interaction checks, needs `npm run dev`
cd src-tauri
cargo test                          # backend unit tests
cargo test -- --ignored --nocapture # live tests against all providers
```

`npm run dev` opened in a regular browser runs the interface on demo data, without Tauri.

## Contact

Questions and ideas: open an [issue](https://github.com/scarrymany/scarpify/issues) or write to the author on [Telegram](https://t.me/yeet17).

## Disclaimer

SCARPIFY is not affiliated with YouTube, Google, SoundCloud, Spotify or Discord. It uses publicly available endpoints the same way their web clients do. Use it in accordance with the terms of the services and the laws of your country.

## License

[GPL-3.0-or-later](LICENSE).

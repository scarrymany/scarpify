<p align="center">
  <img src="assets/icon.svg" width="96" alt="SCARPIFY logo">
</p>

<h1 align="center">SCARPIFY</h1>

<p align="center">
  An open-source desktop music player for YouTube Music and SoundCloud.<br>
  No ads, no accounts, one library.
</p>

<p align="center">
  <a href="README.ru.md">Русский</a> ·
  <a href="https://github.com/scarrymany/scarpify/releases/latest">Download</a>
</p>

## Features

- Search YouTube Music and SoundCloud at the same time. Every track shows which service it comes from.
- Import public Spotify playlists and albums by link, without a Spotify account. Tracks are matched and played through YouTube Music.
- Liked tracks, recently played, queue, shuffle and repeat.
- Volume normalization based on YouTube loudness data.
- Dark, light and system themes, six accent colors, animations that can be turned off.
- Interface in English, Russian and Ukrainian.
- Small footprint: audio is decoded natively in Rust, the interface is a single lightweight web view.

## Install

Download the latest `SCARPIFY_x.y.z_x64-setup.exe` from the [releases page](https://github.com/scarrymany/scarpify/releases/latest) and run it. The installer does not require administrator rights.

Windows 10 and 11 (x64) are supported. WebView2 is installed automatically when missing.

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
| YouTube Music | search through [RustyPipe](https://codeberg.org/ThetaDev/rustypipe), streams through the InnerTube player API |
| SoundCloud | public `api-v2` with the web client id |
| Spotify | public embed pages, no login and no audio from Spotify |

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
cd src-tauri
cargo test                          # unit tests
cargo test -- --ignored --nocapture # live tests against all providers
```

## Disclaimer

SCARPIFY is not affiliated with YouTube, Google, SoundCloud or Spotify. It uses publicly available endpoints the same way their web clients do. Use it in accordance with the terms of the services and the laws of your country.

## License

[GPL-3.0-or-later](LICENSE).

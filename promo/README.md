# SCARPIFY promo

A 30-second promotional video for SCARPIFY, built as a Three.js scene and rendered
offline to `out/scarpify-promo.mp4` (1920×1080, 60 fps, H.264). The promo is separate from
the app and does not touch its code.

## What it shows

| Time | Scene |
| --- | --- |
| 0–4 s | The logo draws its three arcs, then the wordmark and "No ads, no accounts, one library." |
| 4–12 s | The app in 3D: the playlist, home and search windows (real screenshots from `docs/screenshots`) rise and float over a reflective floor while the camera glides past |
| 12–15 s | 01: ad-free player for YouTube Music and SoundCloud |
| 15–18 s | 02: Spotify playlists imported by link, without an account |
| 18–21 s | 03: what you play shows up in your Discord status |
| 21–24.5 s | 04: the window (~200 MB) next to terminal mode in Windows Terminal (~30 MB), with counters |
| 24.5–30 s | Closing title with github.com/scarrymany/scarpify |

The terminal mode has no screenshot, so `src/art.js` redraws it cell by cell from the TUI
layout in `src-tauri/src/tui/ui`: the same sidebar, home shelves and player bar, with real
covers cut from the home screenshot (the app draws them with Sixel).

## Preview

```sh
cd promo
npm install
npm run dev
```

Open the address Vite prints. Space pauses, ←/→ seek by a second, and `?t=18.5` in the URL
opens a paused frame at that moment. The browser preview plays in real time; the rendered
file is what to judge smoothness by.

## Render

```sh
npm run render
```

This builds the scene, serves `dist/` locally, opens it in headless Chromium with `?render`,
asks the page for each of the 1800 frames at a fixed clock (frame ÷ 60 s), and pipes the raw
pixels into ffmpeg. Every animation is a pure function of time and the particles use a
seeded generator, so each run produces the same video.

It needs `ffmpeg` on the PATH and a Chrome or Chromium. The script looks in common places;
set `CHROME_PATH` to point at another one. WebGL runs through SwiftShader, so no GPU is
required (about 6 minutes for the full video on 4 CPU cores).

Useful options:

```sh
node scripts/render.mjs --stills 2,9,14     # PNG frames in out/stills/
node scripts/render.mjs --from 12 --to 16   # a partial render
```

Text inside the terminal mode uses JetBrains Mono, with system fonts as a fallback for its
emoji icons, so that panel can differ slightly between machines.

## Layout

- `src/main.js`: the scene, the timeline and the render hook (`window.__promo`)
- `src/art.js`: everything flat drawn with the 2D canvas: titles, captions, chips, the
  presence card, the memory counters, the terminal mode
- `src/anim.js`: easing, a seeded random generator and the smooth camera path
- `scripts/render.mjs`: the frame-by-frame renderer

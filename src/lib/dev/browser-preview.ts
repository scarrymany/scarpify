/**
 * Development-only stand-in for the Rust backend, used when the UI runs in a plain browser
 * (`npm run dev`). It serves deterministic demo data so layout and animations can be
 * inspected and profiled without Tauri. Never bundled into release builds.
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Collection, Provider, Track } from "$lib/types";

const PROVIDERS: Provider[] = ["youtube", "soundcloud", "spotify"];

function demoTrack(n: number): Track {
  return {
    id: `demo-${n}`,
    provider: PROVIDERS[n % PROVIDERS.length],
    title: `Demo track ${n}`,
    artists: [`Artist ${(n % 7) + 1}`],
    album: n % 2 === 0 ? `Album ${(n % 5) + 1}` : null,
    durationMs: 120_000 + ((n * 7919) % 180_000),
    artwork: `https://picsum.photos/seed/scarpify-${n}/300/300`,
    url: null,
  };
}

function demoTracks(count: number, offset = 0): Track[] {
  return Array.from({ length: count }, (_, i) => demoTrack(offset + i + 1));
}

function playerEvent(payload: object) {
  void emit("player", payload);
}

mockWindows("main");

mockIPC(
  (command, args) => {
    const query = (args as { query?: string } | undefined)?.query ?? "";
    switch (command) {
      case "search_youtube":
        return { tracks: demoTracks(20, query.length * 3).filter((t) => t.provider !== "soundcloud"), error: null };
      case "search_soundcloud":
        return { tracks: demoTracks(20, query.length * 5).filter((t) => t.provider === "soundcloud"), error: null };
      case "import_playlist":
        return {
          kind: "playlist",
          name: "Demo playlist",
          owner: "SCARPIFY",
          artwork: "https://picsum.photos/seed/scarpify-playlist/400/400",
          provider: "spotify",
          tracks: demoTracks(300),
        } satisfies Collection;
      case "play":
        setTimeout(() => playerEvent({ kind: "playing", positionMs: 0 }), 120);
        return null;
      case "resume":
        playerEvent({ kind: "playing", positionMs: 0 });
        return null;
      case "pause":
        playerEvent({ kind: "paused", positionMs: 0 });
        return null;
      case "plugin:app|version":
        return "preview";
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

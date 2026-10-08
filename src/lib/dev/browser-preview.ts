/**
 * Development-only stand-in for the Rust backend, used when the UI runs in a plain browser
 * (`npm run dev`). It keeps an in-memory library and serves deterministic demo data so layout
 * and animations can be inspected, profiled and screenshotted without Tauri. Tests and the
 * README screenshot script may provide real data through `window.__SCARPIFY_FIXTURE__`.
 * Never bundled into release builds.
 */
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { LibrarySnapshot, OutputDevice, ProviderResult, SavedCollection, Track } from "$lib/types";

interface Fixture {
  search?: { youtube: Track[]; soundcloud: Track[] };
  imports?: Omit<SavedCollection, "id" | "origin" | "customArtwork" | "source" | "createdAt" | "pinned">[];
  library?: Partial<LibrarySnapshot>;
}

const fixture: Fixture = (window as { __SCARPIFY_FIXTURE__?: Fixture }).__SCARPIFY_FIXTURE__ ?? {};

function demoTrack(n: number): Track {
  const providers = ["youtube", "soundcloud", "spotify"] as const;
  return {
    id: `demo-${n}`,
    provider: providers[n % providers.length],
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

const key = (track: Track) => `${track.provider}:${track.id}`;
let nextId = 1;

const library: LibrarySnapshot = {
  liked: fixture.library?.liked ?? [],
  recent: fixture.library?.recent ?? [],
  collections: fixture.library?.collections ?? [],
};

const devices: OutputDevice[] = [
  { id: "speakers", name: "Speakers", isDefault: true },
  { id: "headphones", name: "Headphones", isDefault: false },
];

function find(id: string): SavedCollection {
  const collection = library.collections.find((c) => c.id === id);
  if (!collection) throw new Error("playlist not found");
  return collection;
}

function save(collection: Omit<SavedCollection, "id" | "createdAt" | "pinned" | "customArtwork">): SavedCollection {
  const saved: SavedCollection = {
    ...collection,
    id: `preview-${nextId++}`,
    customArtwork: null,
    createdAt: Date.now(),
    pinned: false,
  };
  library.collections = [saved, ...library.collections];
  return saved;
}

function search(provider: "youtube" | "soundcloud", query: string): ProviderResult {
  const real = fixture.search?.[provider];
  if (real) return { tracks: real, error: null };
  const tracks = demoTracks(20, query.length * 3).filter((t) =>
    provider === "soundcloud" ? t.provider === "soundcloud" : t.provider !== "soundcloud",
  );
  return { tracks, error: null };
}

function playerEvent(payload: object) {
  void emit("player", payload);
}

mockWindows("main");

mockIPC(
  (command, raw) => {
    const args = (raw ?? {}) as Record<string, never>;
    switch (command) {
      case "search_youtube":
        return search("youtube", args.query);
      case "search_soundcloud":
        return search("soundcloud", args.query);
      case "import_playlist": {
        const imported = fixture.imports?.shift();
        return save({
          origin: "import",
          source: args.link,
          ...(imported ?? {
            kind: "playlist",
            name: "Demo playlist",
            owner: "SCARPIFY",
            artwork: "https://picsum.photos/seed/scarpify-playlist/400/400",
            provider: "spotify",
            tracks: demoTracks(300),
          }),
        });
      }
      case "play":
        setTimeout(() => playerEvent({ kind: "playing", positionMs: 0 }), 120);
        return null;
      case "resume":
        playerEvent({ kind: "playing", positionMs: 0 });
        return null;
      case "pause":
        playerEvent({ kind: "paused", positionMs: 0 });
        return null;
      case "audio_devices":
        return devices;
      case "library_snapshot":
      case "library_import_legacy":
        return structuredClone(library);
      case "library_set_liked": {
        const track = args.track as Track;
        library.liked = library.liked.filter((t) => key(t) !== key(track));
        if (args.liked) library.liked.unshift(track);
        return null;
      }
      case "library_mark_played": {
        const track = args.track as Track;
        library.recent = [track, ...library.recent.filter((t) => key(t) !== key(track))].slice(0, 24);
        return null;
      }
      case "playlist_create":
        return save({ origin: "user", kind: "playlist", name: args.name, owner: null, artwork: null, provider: null, source: null, tracks: [] });
      case "playlist_rename":
        find(args.id).name = args.name;
        return null;
      case "playlist_add_tracks": {
        const playlist = find(args.id);
        const present = new Set(playlist.tracks.map(key));
        playlist.tracks.push(...(args.tracks as Track[]).filter((t) => !present.has(key(t))));
        return structuredClone(playlist);
      }
      case "playlist_remove_track": {
        const playlist = find(args.id);
        playlist.tracks = playlist.tracks.filter((t) => key(t) !== key(args.track as Track));
        return structuredClone(playlist);
      }
      case "playlist_move_track": {
        const playlist = find(args.id);
        const [moved] = playlist.tracks.splice(args.from, 1);
        playlist.tracks.splice(args.to, 0, moved);
        return structuredClone(playlist);
      }
      case "collection_set_cover":
        find(args.id).customArtwork = args.cover;
        return null;
      case "collection_set_pinned":
        find(args.id).pinned = args.pinned;
        return null;
      case "collection_delete":
        library.collections = library.collections.filter((c) => c.id !== args.id);
        return null;
      case "collections_reorder": {
        const order = args.ids as string[];
        library.collections.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
        return null;
      }
      case "plugin:app|version":
        return "preview";
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);

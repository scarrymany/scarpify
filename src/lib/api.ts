import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LibrarySnapshot, OutputDevice, PlayerEvent, ProviderResult, SavedCollection, Track } from "./types";

const PLAYER_EVENT = "player";

export const api = {
  searchYoutube: (query: string) => invoke<ProviderResult>("search_youtube", { query }),
  searchSoundcloud: (query: string) => invoke<ProviderResult>("search_soundcloud", { query }),
  importPlaylist: (link: string) => invoke<SavedCollection>("import_playlist", { link }),
  prefetch: (track: Track) => invoke<void>("prefetch", { track }),
  play: (track: Track) => invoke<void>("play", { track }),
  pause: () => invoke<void>("pause"),
  resume: () => invoke<void>("resume"),
  stop: () => invoke<void>("stop"),
  seek: (positionMs: number) => invoke<void>("seek", { positionMs: Math.round(positionMs) }),
  setVolume: (volume: number) => invoke<void>("set_volume", { volume }),
  audioDevices: () => invoke<OutputDevice[]>("audio_devices"),
  setAudioDevice: (device: string | null) => invoke<void>("set_audio_device", { device }),
  updatePresence: (nowPlaying: NowPlaying | null) => invoke<void>("update_presence", { nowPlaying }),
  setPresenceEnabled: (enabled: boolean) => invoke<void>("set_presence_enabled", { enabled }),

  librarySnapshot: () => invoke<LibrarySnapshot>("library_snapshot"),
  libraryImportLegacy: (legacy: object) => invoke<LibrarySnapshot>("library_import_legacy", { legacy }),
  setLiked: (track: Track, liked: boolean) => invoke<void>("library_set_liked", { track, liked }),
  markPlayed: (track: Track) => invoke<void>("library_mark_played", { track }),
  createPlaylist: (name: string) => invoke<SavedCollection>("playlist_create", { name }),
  renamePlaylist: (id: string, name: string) => invoke<void>("playlist_rename", { id, name }),
  addToPlaylist: (id: string, tracks: Track[]) => invoke<SavedCollection>("playlist_add_tracks", { id, tracks }),
  removeFromPlaylist: (id: string, track: Track) => invoke<SavedCollection>("playlist_remove_track", { id, track }),
  moveInPlaylist: (id: string, from: number, to: number) =>
    invoke<SavedCollection>("playlist_move_track", { id, from, to }),
  setCover: (id: string, cover: string | null) => invoke<void>("collection_set_cover", { id, cover }),
  setPinned: (id: string, pinned: boolean) => invoke<void>("collection_set_pinned", { id, pinned }),
  deleteCollection: (id: string) => invoke<void>("collection_delete", { id }),
  reorderCollections: (ids: string[]) => invoke<void>("collections_reorder", { ids }),
};

/** What Discord shows under "Listening to SCARPIFY". */
export interface NowPlaying {
  title: string;
  artists: string[];
  album: string | null;
  artwork: string | null;
  url: string | null;
  positionMs: number;
  durationMs: number | null;
}

export function onPlayerEvent(handler: (event: PlayerEvent) => void): Promise<UnlistenFn> {
  return listen<PlayerEvent>(PLAYER_EVENT, (event) => handler(event.payload));
}

/** Tauri rejects with the serialized Rust error, which is a plain string. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

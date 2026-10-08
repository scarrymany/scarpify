import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Collection, PlayerEvent, ProviderResult, Track } from "./types";

const PLAYER_EVENT = "player";

export const api = {
  searchYoutube: (query: string) => invoke<ProviderResult>("search_youtube", { query }),
  searchSoundcloud: (query: string) => invoke<ProviderResult>("search_soundcloud", { query }),
  importPlaylist: (link: string) => invoke<Collection>("import_playlist", { link }),
  prefetch: (track: Track) => invoke<void>("prefetch", { track }),
  play: (track: Track) => invoke<void>("play", { track }),
  pause: () => invoke<void>("pause"),
  resume: () => invoke<void>("resume"),
  stop: () => invoke<void>("stop"),
  seek: (positionMs: number) => invoke<void>("seek", { positionMs: Math.round(positionMs) }),
  setVolume: (volume: number) => invoke<void>("set_volume", { volume }),
  updatePresence: (nowPlaying: NowPlaying | null) => invoke<void>("update_presence", { nowPlaying }),
  setPresenceEnabled: (enabled: boolean) => invoke<void>("set_presence_enabled", { enabled }),
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

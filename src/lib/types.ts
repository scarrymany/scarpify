export type Provider = "youtube" | "soundcloud" | "spotify";

export interface Track {
  id: string;
  provider: Provider;
  title: string;
  artists: string[];
  album: string | null;
  durationMs: number | null;
  artwork: string | null;
  url: string | null;
}

export interface Collection {
  /** Absent in collections saved by early builds, which were all playlists. */
  kind?: "playlist" | "album";
  name: string;
  owner: string | null;
  artwork: string | null;
  provider: Provider;
  tracks: Track[];
}

export interface SavedCollection extends Collection {
  id: string;
  source: string;
  importedAt: number;
}

export interface ProviderResult {
  tracks: Track[];
  error: string | null;
}

export type PlayerEvent =
  | { kind: "playing"; positionMs: number }
  | { kind: "paused"; positionMs: number }
  | { kind: "progress"; positionMs: number }
  | { kind: "ended" }
  | { kind: "error"; message: string };

export function trackKey(track: Track): string {
  return `${track.provider}:${track.id}`;
}

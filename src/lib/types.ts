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

export type CollectionKind = "playlist" | "album";

/** `import`: brought in by link, read-only. `user`: made and edited in SCARPIFY. */
export type CollectionOrigin = "import" | "user";

export interface SavedCollection {
  id: string;
  origin: CollectionOrigin;
  kind: CollectionKind;
  name: string;
  owner: string | null;
  artwork: string | null;
  /** Cover picked by the user; shown instead of `artwork`. */
  customArtwork: string | null;
  provider: Provider | null;
  source: string | null;
  createdAt: number;
  pinned: boolean;
  tracks: Track[];
}

export interface LibrarySnapshot {
  liked: Track[];
  recent: Track[];
  collections: SavedCollection[];
}

export interface OutputDevice {
  id: string;
  name: string;
  isDefault: boolean;
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

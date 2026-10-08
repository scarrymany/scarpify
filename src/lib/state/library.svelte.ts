import { api, errorMessage } from "$lib/api";
import { format, i18n } from "$lib/i18n/index.svelte";
import { trackKey, type LibrarySnapshot, type SavedCollection, type Track } from "$lib/types";
import { toasts } from "./toasts.svelte";

const RECENT_LIMIT = 24;
/** Browser-storage keys used before the library moved to SQLite. */
const LEGACY_PREFIX = "scarpify:";
const LEGACY_KEYS = ["liked", "recent", "collections"] as const;

function report(error: unknown): void {
  toasts.error(format(i18n.t.errors.generic, { error: errorMessage(error) }));
}

/** Reads the pre-SQLite library once; `null` when there is nothing to migrate. */
function readLegacy(): Record<string, unknown> | null {
  try {
    const entries = LEGACY_KEYS.map((key) => [key, localStorage.getItem(LEGACY_PREFIX + key)] as const);
    if (entries.every(([, raw]) => raw === null)) return null;
    return Object.fromEntries(entries.map(([key, raw]) => [key, raw ? JSON.parse(raw) : []]));
  } catch {
    return null;
  }
}

function clearLegacy(): void {
  try {
    for (const key of LEGACY_KEYS) localStorage.removeItem(LEGACY_PREFIX + key);
  } catch {
    // Leaving the keys behind only repeats an idempotent import next launch.
  }
}

/**
 * Mirror of the SQLite library. Edits apply here first so the interface reacts at once,
 * then persist in the background; a failed write is reported and the state reloaded.
 */
class Library {
  liked = $state<Track[]>([]);
  recent = $state<Track[]>([]);
  collections = $state<SavedCollection[]>([]);
  ready = $state(false);

  #likedKeys = $derived(new Set(this.liked.map(trackKey)));
  userPlaylists = $derived(this.collections.filter((c) => c.origin === "user"));

  async init(): Promise<void> {
    const legacy = readLegacy();
    const snapshot = legacy ? await api.libraryImportLegacy(legacy) : await api.librarySnapshot();
    if (legacy) clearLegacy();
    this.#apply(snapshot);
    this.ready = true;
  }

  isLiked(track: Track): boolean {
    return this.#likedKeys.has(trackKey(track));
  }

  toggleLike(track: Track): void {
    const liked = !this.isLiked(track);
    const key = trackKey(track);
    this.liked = liked ? [track, ...this.liked] : this.liked.filter((t) => trackKey(t) !== key);
    this.#persist(api.setLiked(track, liked));
  }

  markPlayed(track: Track): void {
    const key = trackKey(track);
    this.recent = [track, ...this.recent.filter((t) => trackKey(t) !== key)].slice(0, RECENT_LIMIT);
    this.#persist(api.markPlayed(track));
  }

  collection(id: string): SavedCollection | undefined {
    return this.collections.find((c) => c.id === id);
  }

  async importPlaylist(link: string): Promise<SavedCollection> {
    const saved = await api.importPlaylist(link);
    this.#upsert(saved);
    return saved;
  }

  async createPlaylist(): Promise<SavedCollection> {
    const name = format(i18n.t.playlist.defaultName, { number: this.userPlaylists.length + 1 });
    const created = await api.createPlaylist(name);
    this.#upsert(created);
    return created;
  }

  rename(id: string, name: string): void {
    const trimmed = name.trim();
    if (!trimmed) return;
    this.#patch(id, { name: trimmed });
    this.#persist(api.renamePlaylist(id, trimmed));
  }

  setCover(id: string, cover: string | null): void {
    this.#patch(id, { customArtwork: cover });
    this.#persist(api.setCover(id, cover));
  }

  setPinned(id: string, pinned: boolean): void {
    this.#patch(id, { pinned });
    this.collections = sortForSidebar(this.collections);
    this.#persist(api.setPinned(id, pinned));
  }

  remove(id: string): void {
    this.collections = this.collections.filter((c) => c.id !== id);
    this.#persist(api.deleteCollection(id));
  }

  /** Reorders the sidebar; pinned and unpinned playlists each keep to their own group. */
  move(from: number, to: number): void {
    const list = [...this.collections];
    const pinned = list[from]?.pinned;
    const groupEnd = list.filter((c) => c.pinned).length;
    const target = pinned ? Math.min(to, groupEnd - 1) : Math.max(to, groupEnd);
    const [moved] = list.splice(from, 1);
    list.splice(target, 0, moved);
    this.collections = list;
    this.#persist(api.reorderCollections(list.map((c) => c.id)));
  }

  async addTracks(id: string, tracks: Track[]): Promise<void> {
    try {
      this.#upsert(await api.addToPlaylist(id, tracks));
      const playlist = this.collection(id);
      if (playlist) toasts.show(format(i18n.t.playlist.added, { name: playlist.name }));
    } catch (error) {
      report(error);
    }
  }

  removeTrack(id: string, track: Track): void {
    const key = trackKey(track);
    this.#patch(id, { tracks: this.collection(id)?.tracks.filter((t) => trackKey(t) !== key) ?? [] });
    this.#persist(api.removeFromPlaylist(id, track));
  }

  moveTrack(id: string, from: number, to: number): void {
    const tracks = [...(this.collection(id)?.tracks ?? [])];
    const [moved] = tracks.splice(from, 1);
    if (!moved) return;
    tracks.splice(to, 0, moved);
    this.#patch(id, { tracks });
    this.#persist(api.moveInPlaylist(id, from, to));
  }

  #apply(snapshot: LibrarySnapshot): void {
    this.liked = snapshot.liked;
    this.recent = snapshot.recent;
    this.collections = snapshot.collections;
  }

  #upsert(saved: SavedCollection): void {
    const exists = this.collections.some((c) => c.id === saved.id);
    this.collections = exists
      ? this.collections.map((c) => (c.id === saved.id ? saved : c))
      : sortForSidebar([saved, ...this.collections]);
  }

  #patch(id: string, change: Partial<SavedCollection>): void {
    this.collections = this.collections.map((c) => (c.id === id ? { ...c, ...change } : c));
  }

  #persist(write: Promise<unknown>): void {
    write.catch((error) => {
      report(error);
      api.librarySnapshot().then((snapshot) => this.#apply(snapshot), report);
    });
  }
}

/** Pinned playlists first, otherwise keeping the current order (same rule as the database). */
function sortForSidebar(collections: SavedCollection[]): SavedCollection[] {
  return [...collections.filter((c) => c.pinned), ...collections.filter((c) => !c.pinned)];
}

export const library = new Library();

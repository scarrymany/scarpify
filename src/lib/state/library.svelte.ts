import { trackKey, type Collection, type SavedCollection, type Track } from "$lib/types";
import { load, save } from "./storage";

const RECENT_LIMIT = 24;

class Library {
  liked = $state<Track[]>(load("liked", []));
  recent = $state<Track[]>(load("recent", []));
  collections = $state<SavedCollection[]>(load("collections", []));

  #likedKeys = $derived(new Set(this.liked.map(trackKey)));

  isLiked(track: Track): boolean {
    return this.#likedKeys.has(trackKey(track));
  }

  toggleLike(track: Track): void {
    const key = trackKey(track);
    this.liked = this.isLiked(track)
      ? this.liked.filter((t) => trackKey(t) !== key)
      : [track, ...this.liked];
    save("liked", this.liked);
  }

  markPlayed(track: Track): void {
    const key = trackKey(track);
    this.recent = [track, ...this.recent.filter((t) => trackKey(t) !== key)].slice(0, RECENT_LIMIT);
    save("recent", this.recent);
  }

  addCollection(collection: Collection, source: string): SavedCollection {
    const existing = this.collections.find((c) => c.source === source);
    const saved: SavedCollection = {
      ...collection,
      id: existing?.id ?? crypto.randomUUID(),
      source,
      importedAt: Date.now(),
    };
    this.collections = existing
      ? this.collections.map((c) => (c.id === saved.id ? saved : c))
      : [saved, ...this.collections];
    save("collections", this.collections);
    return saved;
  }

  removeCollection(id: string): void {
    this.collections = this.collections.filter((c) => c.id !== id);
    save("collections", this.collections);
  }

  collection(id: string): SavedCollection | undefined {
    return this.collections.find((c) => c.id === id);
  }
}

export const library = new Library();

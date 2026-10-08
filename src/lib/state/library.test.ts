import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SavedCollection } from "$lib/types";
import { flush, makeTrack } from "../../test/fixtures";
import { invoke } from "../../test/tauri";

type Library = (typeof import("./library.svelte"))["library"];

let library: Library;

function collection(id: string, pinned = false): SavedCollection {
  return {
    id,
    origin: "user",
    kind: "playlist",
    name: id,
    owner: null,
    artwork: null,
    customArtwork: null,
    provider: null,
    source: null,
    createdAt: 0,
    pinned,
    tracks: [],
  };
}

function calls(command: string): unknown[] {
  return invoke.mock.calls.filter(([name]) => name === command).map(([, args]) => args);
}

beforeEach(async () => {
  vi.resetModules();
  localStorage.clear();
  invoke.mockReset();
  invoke.mockImplementation(async (command: string) => {
    if (command === "library_snapshot" || command === "library_import_legacy") {
      return { liked: [], recent: [], collections: [collection("a", true), collection("b"), collection("c")] };
    }
    if (command === "playlist_create") return collection("new");
    return undefined;
  });
  ({ library } = await import("./library.svelte"));
});

describe("library", () => {
  it("moves the pre-SQLite browser library into the database once", async () => {
    localStorage.setItem("scarpify:liked", JSON.stringify([makeTrack(1)]));
    await library.init();

    expect(calls("library_import_legacy")).toEqual([
      { legacy: { liked: [makeTrack(1)], recent: [], collections: [] } },
    ]);
    expect(localStorage.getItem("scarpify:liked")).toBeNull();
    expect(library.ready).toBe(true);
  });

  it("loads straight from the database when there is nothing to migrate", async () => {
    await library.init();
    expect(calls("library_import_legacy")).toHaveLength(0);
    expect(library.collections.map((c) => c.id)).toEqual(["a", "b", "c"]);
  });

  it("applies likes immediately and persists them", async () => {
    await library.init();
    const track = makeTrack(5);
    library.toggleLike(track);
    expect(library.isLiked(track)).toBe(true);
    library.toggleLike(track);
    expect(library.isLiked(track)).toBe(false);
    expect(calls("library_set_liked")).toEqual([
      { track, liked: true },
      { track, liked: false },
    ]);
  });

  it("names new playlists by count in the interface language", async () => {
    await library.init();
    await library.createPlaylist();
    expect(calls("playlist_create")).toEqual([{ name: "My playlist #4" }]);
    expect(library.collection("new")).toBeDefined();
  });

  it("keeps pinned and unpinned playlists in their own groups when dragged", async () => {
    await library.init();
    library.move(2, 0);
    expect(library.collections.map((c) => c.id)).toEqual(["a", "c", "b"]);
    expect(calls("collections_reorder")).toEqual([{ ids: ["a", "c", "b"] }]);
  });

  it("reloads the library when a write fails", async () => {
    await library.init();
    invoke.mockImplementationOnce(async () => {
      throw "disk full";
    });
    library.rename("b", "Renamed");
    expect(library.collection("b")?.name).toBe("Renamed");
    await flush();
    expect(library.collection("b")?.name).toBe("b");
  });
});

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PlayerEvent } from "$lib/types";
import { flush, makeTrack, makeTracks } from "../../test/fixtures";
import { invoke, listen } from "../../test/tauri";

type PlayerModule = typeof import("./player.svelte");

let player: PlayerModule["player"];
let emit: (event: PlayerEvent) => void;

function commands(name: string): unknown[][] {
  return invoke.mock.calls.filter(([command]) => command === name).map(([, args]) => [args]);
}

beforeEach(async () => {
  vi.resetModules();
  invoke.mockReset();
  invoke.mockResolvedValue(undefined);
  localStorage.clear();
  listen.mockImplementation(async (_event, handler) => {
    emit = (payload) => handler({ payload });
    return () => {};
  });

  ({ player } = await import("./player.svelte"));
  await player.init();
});

describe("playback state", () => {
  it("follows engine events, including the camelCase position field", async () => {
    player.playTracks(makeTracks(3), 1);
    expect(player.status).toBe("loading");
    expect(commands("play")).toEqual([[{ track: makeTrack(2) }]]);

    emit({ kind: "playing", positionMs: 0 });
    expect(player.status).toBe("playing");

    emit({ kind: "progress", positionMs: 42_000 });
    expect(player.positionMs).toBe(42_000);
  });

  it("reloads instead of resuming when the previous load failed", async () => {
    invoke.mockImplementation(async (command) => {
      if (command === "play") throw "network error";
    });
    player.playTracks(makeTracks(1));
    await flush();
    expect(player.status).toBe("paused");

    player.toggle();
    expect(commands("resume")).toHaveLength(0);
    expect(commands("play")).toHaveLength(2);
  });

  it("ignores toggles while a track is loading", () => {
    player.playTracks(makeTracks(1));
    player.toggle();
    expect(commands("pause")).toHaveLength(0);
    expect(commands("play")).toHaveLength(1);
  });
});

describe("queue navigation", () => {
  it("restarts the track on previous after a few seconds, otherwise goes back", () => {
    player.playTracks(makeTracks(3), 1);
    emit({ kind: "playing", positionMs: 0 });
    emit({ kind: "progress", positionMs: 10_000 });

    player.previous();
    expect(player.index).toBe(1);
    expect(commands("seek")).toEqual([[{ positionMs: 0 }]]);

    player.previous();
    expect(player.index).toBe(0);
  });

  it("advances on end and stops after the last track when repeat is off", () => {
    player.playTracks(makeTracks(2));
    emit({ kind: "ended" });
    expect(player.index).toBe(1);

    emit({ kind: "ended" });
    expect(player.index).toBe(1);
    expect(player.status).toBe("paused");
  });

  it("wraps around with repeat all and replays with repeat one", () => {
    player.playTracks(makeTracks(2), 1);
    player.cycleRepeat();
    expect(player.repeat).toBe("all");
    emit({ kind: "ended" });
    expect(player.index).toBe(0);

    player.cycleRepeat();
    expect(player.repeat).toBe("one");
    const plays = commands("play").length;
    emit({ kind: "ended" });
    expect(player.index).toBe(0);
    expect(commands("play")).toHaveLength(plays + 1);
  });

  it("shuffles around the current track and restores the original order", () => {
    const tracks = makeTracks(20);
    player.playTracks(tracks, 4);
    player.toggleShuffle();
    expect(player.current?.id).toBe("id-5");
    expect(player.index).toBe(0);
    expect(new Set(player.queue.map((t) => t.id)).size).toBe(20);

    player.toggleShuffle();
    expect(player.queue.map((t) => t.id)).toEqual(tracks.map((t) => t.id));
    expect(player.index).toBe(4);
  });
});

describe("prefetching", () => {
  it("resolves a hovered track once, after the hover settles", () => {
    vi.useFakeTimers();
    try {
      const track = makeTrack(7, "soundcloud");
      player.prefetchSoon(track);
      player.cancelPrefetch();
      vi.runAllTimers();
      expect(commands("prefetch")).toHaveLength(0);

      player.prefetchSoon(track);
      vi.runAllTimers();
      player.prefetchSoon(track);
      vi.runAllTimers();
      expect(commands("prefetch")).toEqual([[{ track }]]);
    } finally {
      vi.useRealTimers();
    }
  });

  it("prefetches the next track as soon as playback starts", () => {
    const tracks = makeTracks(3);
    player.playTracks(tracks);
    emit({ kind: "playing", positionMs: 0 });
    expect(commands("prefetch")).toEqual([[{ track: tracks[1] }]]);
  });
});

describe("Discord presence", () => {
  it("shows the playing track with its position and hides it on pause", () => {
    const track = makeTrack(3, "soundcloud");
    player.playTracks([track]);
    emit({ kind: "playing", positionMs: 0 });
    emit({ kind: "progress", positionMs: 30_000 });
    player.seek(60_000);

    const updates = commands("update_presence").map(([args]) => (args as { nowPlaying: unknown }).nowPlaying);
    expect(updates[0]).toMatchObject({ title: track.title, artists: track.artists, positionMs: 0 });
    expect(updates.at(-1)).toMatchObject({ positionMs: 60_000, durationMs: track.durationMs });

    emit({ kind: "paused", positionMs: 61_000 });
    expect(commands("update_presence").at(-1)).toEqual([{ nowPlaying: null }]);
  });
});

describe("queue editing", () => {
  it("reorders and shuffles only what comes after the current track", () => {
    const tracks = makeTracks(12);
    player.playTracks(tracks, 2);

    player.moveUpcoming(0, 3);
    expect(player.upcoming.slice(0, 4).map((t) => t.id)).toEqual(["id-5", "id-6", "id-7", "id-4"]);

    player.shuffleUpcoming();
    expect(player.current?.id).toBe("id-3");
    expect(player.queue.slice(0, 3).map((t) => t.id)).toEqual(["id-1", "id-2", "id-3"]);
    expect(new Set(player.upcoming.map((t) => t.id))).toEqual(new Set(tracks.slice(3).map((t) => t.id)));
  });
});

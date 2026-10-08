import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProviderResult } from "$lib/types";
import { flush, makeTracks } from "../../test/fixtures";
import { invoke } from "../../test/tauri";

type Search = (typeof import("./search.svelte"))["search"];

let search: Search;

/** Each provider command waits on its own promise, so tests decide who answers first. */
let pending: Record<string, (result: ProviderResult) => void>;

beforeEach(async () => {
  vi.resetModules();
  vi.useFakeTimers();
  pending = {};
  invoke.mockReset();
  invoke.mockImplementation(
    (command: string, args?: unknown) =>
      new Promise((resolve) => {
        pending[`${command}:${(args as { query: string }).query}`] = resolve;
      }),
  );
  ({ search } = await import("./search.svelte"));
});

afterEach(() => {
  vi.useRealTimers();
});

const ok = (count: number, provider: "youtube" | "soundcloud"): ProviderResult => ({
  tracks: makeTracks(count, provider),
  error: null,
});

describe("search", () => {
  it("debounces typing into a single request per provider", () => {
    search.setQuery("d");
    search.setQuery("da");
    search.setQuery("daft");
    expect(invoke).not.toHaveBeenCalled();

    vi.runAllTimers();
    expect(invoke.mock.calls.map(([command]) => command).sort()).toEqual([
      "search_soundcloud",
      "search_youtube",
    ]);
  });

  it("shows the faster provider without waiting for the slower one", async () => {
    search.setQuery("daft");
    vi.runAllTimers();

    pending["search_soundcloud:daft"](ok(3, "soundcloud"));
    await flush();
    expect(search.results.soundcloud?.tracks).toHaveLength(3);
    expect(search.results.youtube).toBeNull();
    expect(search.pending.youtube).toBe(true);

    pending["search_youtube:daft"](ok(2, "youtube"));
    await flush();
    expect(search.results.youtube?.tracks).toHaveLength(2);
    expect(search.loading).toBe(false);
  });

  it("drops answers that arrive for an outdated query", async () => {
    search.setQuery("old");
    vi.runAllTimers();
    search.setQuery("new");
    vi.runAllTimers();

    pending["search_youtube:new"](ok(1, "youtube"));
    pending["search_youtube:old"](ok(9, "youtube"));
    await flush();
    expect(search.resultsFor).toBe("new");
    expect(search.results.youtube?.tracks).toHaveLength(1);
  });

  it("keeps a failing provider from hiding the other one", async () => {
    search.setQuery("x");
    vi.runAllTimers();
    pending["search_youtube:x"]({ tracks: [], error: "timeout" });
    pending["search_soundcloud:x"](ok(2, "soundcloud"));
    await flush();
    expect(search.results.youtube?.error).toBe("timeout");
    expect(search.results.soundcloud?.tracks).toHaveLength(2);
  });

  it("clears everything when the query is emptied", async () => {
    search.setQuery("x");
    vi.runAllTimers();
    search.setQuery("");
    pending["search_youtube:x"](ok(1, "youtube"));
    await flush();
    expect(search.hasAny).toBe(false);
    expect(search.loading).toBe(false);
  });
});

import { fireEvent, render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { settings } from "$lib/state/settings.svelte";
import { flush, makeTrack, makeTracks } from "../../test/fixtures";
import { invoke } from "../../test/tauri";
import ImportDialog from "./ImportDialog.svelte";
import PlayPauseIcon from "./PlayPauseIcon.svelte";
import ProviderBadge from "./ProviderBadge.svelte";
import Slider from "./Slider.svelte";
import TrackList from "./TrackList.svelte";

beforeEach(() => {
  settings.locale = "en";
  invoke.mockReset();
  invoke.mockResolvedValue(undefined);
});

describe("ProviderBadge", () => {
  it("names the service the track comes from", () => {
    render(ProviderBadge, { provider: "soundcloud" });
    expect(screen.getByText("SoundCloud")).toBeInTheDocument();
  });

  it("explains that Spotify tracks play through YouTube Music", () => {
    const { container } = render(ProviderBadge, { provider: "spotify" });
    expect(container.querySelector(".badge")).toHaveAttribute(
      "title",
      "Spotify. Matched on YouTube Music",
    );
  });

  it("follows the interface language", async () => {
    render(ProviderBadge, { provider: "youtube", variant: "icon" });
    expect(screen.getByText("YouTube Music")).toHaveClass("sr-only");
  });
});

describe("TrackList", () => {
  it("shows every track with its provider under it", () => {
    const tracks = [makeTrack(1, "youtube"), makeTrack(2, "soundcloud"), makeTrack(3, "spotify")];
    render(TrackList, { tracks });

    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent("Track 1");
    expect(rows[0]).toHaveTextContent("YouTube Music");
    expect(rows[1]).toHaveTextContent("SoundCloud");
    expect(rows[2]).toHaveTextContent("Spotify");
  });

  it("plays the double-clicked row", async () => {
    const tracks = makeTracks(3);
    render(TrackList, { tracks });
    await fireEvent.dblClick(screen.getAllByRole("listitem")[2]);
    expect(invoke).toHaveBeenCalledWith("play", { track: tracks[2] });
  });

  it("toggles the like state of a row", async () => {
    render(TrackList, { tracks: [makeTrack(42)] });
    const like = screen.getByRole("button", { name: "Save to Liked tracks" });
    await fireEvent.click(like);
    expect(screen.getByRole("button", { name: "Remove from Liked tracks" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});

describe("PlayPauseIcon", () => {
  function shapes(container: HTMLElement): string[] {
    return [...container.querySelectorAll("path")].map((p) => p.style.getPropertyValue("d"));
  }

  it("morphs between two shapes with matching point counts", async () => {
    const { container, rerender } = render(PlayPauseIcon, { playing: false });
    const play = shapes(container);
    await rerender({ playing: true });
    const pause = shapes(container);

    expect(play).toHaveLength(2);
    expect(pause).not.toEqual(play);
    const points = (d: string) => d.match(/[ML]/g)?.length;
    expect(pause.map(points)).toEqual(play.map(points));
  });
});

describe("Slider", () => {
  const ratio = (container: HTMLElement) =>
    (container.querySelector(".slider") as HTMLElement).style.getPropertyValue("--ratio");

  it("never paints a full bar from an invalid position", () => {
    const { container } = render(Slider, { value: Number.NaN, max: 1000, label: "Seek", oncommit: () => {} });
    expect(ratio(container)).toBe("0");
  });

  it("clamps and reflects progress", () => {
    const { container } = render(Slider, { value: 250, max: 1000, label: "Seek", oncommit: () => {} });
    expect(ratio(container)).toBe("0.25");
  });

  it("steps with the keyboard", async () => {
    let committed = -1;
    render(Slider, { value: 500, max: 1000, step: 100, label: "Seek", oncommit: (v: number) => (committed = v) });
    await fireEvent.keyDown(screen.getByRole("slider"), { key: "ArrowRight" });
    expect(committed).toBe(600);
  });
});

describe("ImportDialog", () => {
  async function typeLink(link: string) {
    const input = screen.getByRole("textbox");
    await fireEvent.input(input, { target: { value: link } });
  }

  it("recognizes the service from the link while typing", async () => {
    const { container } = render(ImportDialog, { onclose: () => {} });
    await typeLink("https://soundcloud.com/dabbackwood/sets/crests");
    const dimmed = [...container.querySelectorAll(".provider.dimmed")].map((el) => el.textContent?.trim());
    expect(dimmed).toEqual(["Spotify", "YouTube Music"]);
  });

  it("rejects unsupported links without calling the backend", async () => {
    render(ImportDialog, { onclose: () => {} });
    await typeLink("https://example.com/playlist/1");
    await fireEvent.click(screen.getByRole("button", { name: "Import" }));
    expect(screen.getByText(/Paste a link to a playlist or album/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalledWith("import_playlist", expect.anything());
  });

  it("imports a supported link and closes", async () => {
    let closed = false;
    invoke.mockImplementation(async (command) =>
      command === "import_playlist"
        ? { kind: "album", name: "Crests", owner: null, artwork: null, provider: "soundcloud", tracks: [] }
        : undefined,
    );
    render(ImportDialog, { onclose: () => (closed = true) });
    await typeLink("https://soundcloud.com/dabbackwood/sets/crests");
    await fireEvent.click(screen.getByRole("button", { name: "Import" }));
    await flush();
    expect(invoke).toHaveBeenCalledWith("import_playlist", { link: "https://soundcloud.com/dabbackwood/sets/crests" });
    expect(closed).toBe(true);
  });
});

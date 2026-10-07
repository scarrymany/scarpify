import { api, errorMessage, onPlayerEvent } from "$lib/api";
import { format, i18n } from "$lib/i18n/index.svelte";
import { trackKey, type PlayerEvent, type Track } from "$lib/types";
import { library } from "./library.svelte";
import { load, save } from "./storage";
import { toasts } from "./toasts.svelte";

export type Status = "idle" | "loading" | "playing" | "paused";
export type RepeatMode = "off" | "all" | "one";

/** Pressing "previous" after this point restarts the track instead. */
const RESTART_THRESHOLD_MS = 3000;
const DEFAULT_VOLUME = 0.7;
/** Hover must rest this long before a stream is resolved, so scrolling does not fire requests. */
const PREFETCH_DELAY_MS = 180;
/** Slightly shorter than the backend stream cache, so a repeat hover refreshes in time. */
const PREFETCH_TTL_MS = 8 * 60 * 1000;

function shuffled<T>(items: T[]): T[] {
  const result = [...items];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

class Player {
  queue = $state<Track[]>([]);
  index = $state(-1);
  status = $state<Status>("idle");
  positionMs = $state(0);
  volume = $state(load("volume", DEFAULT_VOLUME));
  muted = $state(false);
  shuffle = $state(false);
  repeat = $state<RepeatMode>("off");

  current = $derived(this.queue[this.index] ?? null);
  upcoming = $derived(this.queue.slice(this.index + 1));
  durationMs = $derived(this.current?.durationMs ?? 0);

  /** Queue order before shuffling, restored when shuffle is turned off. */
  #ordered: Track[] = [];
  /** Guards against a slow load finishing after the user already picked another track. */
  #loadId = 0;
  /** Whether the engine holds a decoded track that `resume` can continue. */
  #engineLoaded = false;
  #anchorMs = 0;
  #anchorAt = 0;
  #frame = 0;
  #prefetchTimer: ReturnType<typeof setTimeout> | undefined;
  #prefetchedAt = new Map<string, number>();

  async init(): Promise<void> {
    await onPlayerEvent((event) => this.#handle(event));
    await api.setVolume(this.#effectiveVolume());
  }

  playTracks(tracks: Track[], startIndex = 0): void {
    if (tracks.length === 0) return;
    const start = tracks[startIndex];
    this.#ordered = [...tracks];
    if (this.shuffle) {
      const rest = shuffled(tracks.filter((_, i) => i !== startIndex));
      this.queue = [start, ...rest];
      this.index = 0;
    } else {
      this.queue = [...tracks];
      this.index = startIndex;
    }
    void this.#load();
  }

  enqueue(track: Track): void {
    this.queue = [...this.queue, track];
    this.#ordered = [...this.#ordered, track];
    if (this.index < 0) {
      this.index = this.queue.length - 1;
      void this.#load();
    }
  }

  jumpTo(index: number): void {
    if (index < 0 || index >= this.queue.length) return;
    this.index = index;
    void this.#load();
  }

  clearUpcoming(): void {
    this.queue = this.queue.slice(0, this.index + 1);
    this.#ordered = [...this.queue];
  }

  toggle(): void {
    if (this.status === "loading") return;
    if (this.status === "playing") void api.pause();
    else if (this.#engineLoaded) void api.resume();
    else if (this.current) void this.#load();
  }

  next(): void {
    if (this.index + 1 < this.queue.length) {
      this.index += 1;
    } else if (this.repeat === "all" && this.queue.length > 0) {
      this.index = 0;
    } else {
      return;
    }
    void this.#load();
  }

  previous(): void {
    if (this.positionMs > RESTART_THRESHOLD_MS || this.index === 0) {
      this.seek(0);
      return;
    }
    this.index -= 1;
    void this.#load();
  }

  seek(positionMs: number): void {
    if (!this.current) return;
    const clamped = Math.max(0, Math.min(positionMs, this.durationMs || positionMs));
    this.#setAnchor(clamped);
    void api.seek(clamped);
  }

  setVolume(volume: number): void {
    this.volume = Math.max(0, Math.min(1, volume));
    this.muted = false;
    save("volume", this.volume);
    void api.setVolume(this.#effectiveVolume());
  }

  toggleMute(): void {
    this.muted = !this.muted;
    void api.setVolume(this.#effectiveVolume());
  }

  toggleShuffle(): void {
    this.shuffle = !this.shuffle;
    const current = this.current;
    if (!current) return;

    if (this.shuffle) {
      this.#ordered = [...this.queue];
      const rest = shuffled(this.queue.filter((_, i) => i !== this.index));
      this.queue = [current, ...rest];
      this.index = 0;
    } else {
      this.queue = [...this.#ordered];
      const key = trackKey(current);
      this.index = Math.max(0, this.#ordered.findIndex((t) => trackKey(t) === key));
    }
  }

  cycleRepeat(): void {
    this.repeat = this.repeat === "off" ? "all" : this.repeat === "all" ? "one" : "off";
  }

  /** Resolves a stream ahead of time; the backend caches it, so starting it is near instant. */
  prefetch(track: Track): void {
    const key = trackKey(track);
    const now = Date.now();
    if (now - (this.#prefetchedAt.get(key) ?? -Infinity) < PREFETCH_TTL_MS) return;
    this.#prefetchedAt.set(key, now);
    api.prefetch(track).catch(() => this.#prefetchedAt.delete(key));
  }

  prefetchSoon(track: Track): void {
    clearTimeout(this.#prefetchTimer);
    this.#prefetchTimer = setTimeout(() => this.prefetch(track), PREFETCH_DELAY_MS);
  }

  cancelPrefetch(): void {
    clearTimeout(this.#prefetchTimer);
  }

  async #load(): Promise<void> {
    const track = this.current;
    if (!track) return;
    const loadId = ++this.#loadId;
    this.status = "loading";
    this.#engineLoaded = false;
    this.#setAnchor(0);
    this.#stopClock();

    try {
      await api.play(track);
      if (loadId === this.#loadId) library.markPlayed(track);
    } catch (error) {
      if (loadId !== this.#loadId) return;
      this.status = "paused";
      toasts.error(format(i18n.t.errors.playback, { title: track.title, error: errorMessage(error) }));
    }
  }

  #handle(event: PlayerEvent): void {
    switch (event.kind) {
      case "playing":
        this.#engineLoaded = true;
        this.status = "playing";
        this.#setAnchor(event.positionMs);
        this.#startClock();
        if (this.upcoming[0]) this.prefetch(this.upcoming[0]);
        break;
      case "paused":
        this.status = "paused";
        this.#setAnchor(event.positionMs);
        this.#stopClock();
        break;
      case "progress":
        this.#setAnchor(event.positionMs);
        break;
      case "ended":
        this.#engineLoaded = false;
        this.#stopClock();
        if (this.repeat === "one") void this.#load();
        else if (this.index + 1 < this.queue.length || this.repeat === "all") this.next();
        else this.status = "paused";
        break;
      case "error":
        toasts.error(format(i18n.t.errors.generic, { error: event.message }));
        break;
    }
  }

  #effectiveVolume(): number {
    return this.muted ? 0 : this.volume;
  }

  #setAnchor(positionMs: number): void {
    this.#anchorMs = positionMs;
    this.#anchorAt = performance.now();
    this.positionMs = positionMs;
  }

  /** The engine reports position four times a second; frames in between are interpolated. */
  #startClock(): void {
    this.#stopClock();
    const tick = () => {
      const elapsed = performance.now() - this.#anchorAt;
      const position = this.#anchorMs + elapsed;
      this.positionMs = this.durationMs > 0 ? Math.min(position, this.durationMs) : position;
      this.#frame = requestAnimationFrame(tick);
    };
    this.#frame = requestAnimationFrame(tick);
  }

  #stopClock(): void {
    cancelAnimationFrame(this.#frame);
  }
}

export const player = new Player();

import type { Provider, Track } from "$lib/types";

export function makeTrack(n: number, provider: Provider = "youtube", durationMs = 180_000): Track {
  return {
    id: `id-${n}`,
    provider,
    title: `Track ${n}`,
    artists: [`Artist ${n}`],
    album: null,
    durationMs,
    artwork: null,
    url: null,
  };
}

export function makeTracks(count: number, provider: Provider = "youtube"): Track[] {
  return Array.from({ length: count }, (_, i) => makeTrack(i + 1, provider));
}

/** Lets pending promise callbacks (mocked invokes, `.then` chains) run. */
export async function flush(): Promise<void> {
  for (let i = 0; i < 5; i++) await Promise.resolve();
}

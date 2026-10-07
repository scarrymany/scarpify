<script lang="ts">
  import WarningCircle from "phosphor-svelte/lib/WarningCircle";
  import Artwork from "$lib/components/Artwork.svelte";
  import PlayPauseIcon from "$lib/components/PlayPauseIcon.svelte";
  import ProviderBadge from "$lib/components/ProviderBadge.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import { format, i18n } from "$lib/i18n/index.svelte";
  import { player } from "$lib/state/player.svelte";
  import { search, SEARCH_PROVIDERS, type SearchProvider } from "$lib/state/search.svelte";
  import { trackKey, type Track } from "$lib/types";

  type Filter = "all" | SearchProvider;
  const SKELETON_ROWS = 8;

  let filter = $state<Filter>("all");

  /** Alternates providers so both services are visible at the top of "All". */
  function interleave(a: Track[], b: Track[]): Track[] {
    const merged: Track[] = [];
    for (let i = 0; i < Math.max(a.length, b.length); i++) {
      if (i < a.length) merged.push(a[i]);
      if (i < b.length) merged.push(b[i]);
    }
    return merged;
  }

  const results = $derived(search.results);
  const tracks = $derived.by(() => {
    const youtube = results.youtube?.tracks ?? [];
    const soundcloud = results.soundcloud?.tracks ?? [];
    if (filter === "all") return interleave(youtube, soundcloud);
    return filter === "youtube" ? youtube : soundcloud;
  });
  const shownProviders = $derived(filter === "all" ? SEARCH_PROVIDERS : [filter]);
  const waiting = $derived(shownProviders.some((p) => search.pending[p]));
  const top = $derived(filter === "all" ? tracks[0] : undefined);
  const listed = $derived(top ? tracks.slice(1) : tracks);
  const failures = $derived(shownProviders.filter((p) => results[p]?.error));
  const showSkeleton = $derived(tracks.length === 0 && waiting);
  const topPlaying = $derived(
    top !== undefined && player.status === "playing" && player.current !== null && trackKey(player.current) === trackKey(top),
  );

  function playTop() {
    if (!top) return;
    if (topPlaying) player.toggle();
    else player.playTracks(tracks, 0);
  }

  function filterLabel(value: Filter): string {
    return value === "all" ? i18n.t.search.all : i18n.t.providers[value];
  }
</script>

<div class="search-view">
  {#if search.query.trim() === ""}
    <div class="idle">
      <h1>{i18n.t.search.idleTitle}</h1>
      <p>{i18n.t.search.idleBody}</p>
      <div class="providers">
        <ProviderBadge provider="youtube" />
        <ProviderBadge provider="soundcloud" />
      </div>
    </div>
  {:else}
    <div class="filters" role="tablist">
      {#each ["all", ...SEARCH_PROVIDERS] as const as value (value)}
        <button
          class="chip"
          role="tab"
          aria-selected={filter === value}
          onclick={() => (filter = value)}
        >
          {filterLabel(value)}
        </button>
      {/each}
    </div>

    {#each failures as provider (provider)}
      <p class="notice">
        <WarningCircle weight="fill" />
        {format(i18n.t.search.providerError, { provider: i18n.t.providers[provider], error: results[provider]?.error ?? "" })}
      </p>
    {/each}

    {#if showSkeleton}
      <div class="skeletons" aria-busy="true">
        {#each { length: SKELETON_ROWS } as _, i (i)}
          <div class="skeleton-row">
            <span class="skeleton" style:width="44px" style:height="44px"></span>
            <span class="skeleton-lines">
              <span class="skeleton" style:width="{40 + ((i * 17) % 35)}%" style:height="12px"></span>
              <span class="skeleton" style:width="{20 + ((i * 11) % 20)}%" style:height="10px"></span>
            </span>
          </div>
        {/each}
      </div>
    {:else if search.hasAny && tracks.length === 0 && failures.length === 0}
      <div class="idle">
        <h1>{format(i18n.t.search.noResults, { query: search.resultsFor })}</h1>
        <p>{i18n.t.search.noResultsHint}</p>
      </div>
    {:else if tracks.length > 0}
      <div class="results" class:stale={search.resultsFor !== search.query.trim()}>
        {#if top}
          <section class="top">
            <h2>{i18n.t.search.topResult}</h2>
            <div class="top-card">
              <Artwork src={top.artwork} size="96px" radius="var(--radius-card)" eager />
              <div class="top-text">
                <span class="top-title">{top.title}</span>
                <span class="top-artists">{top.artists.join(", ")}</span>
                <ProviderBadge provider={top.provider} />
              </div>
              <button class="top-play" aria-label={topPlaying ? i18n.t.player.pause : i18n.t.player.play} onclick={playTop}>
                <PlayPauseIcon playing={topPlaying} />
              </button>
            </div>
          </section>
        {/if}
        <section>
          <h2>{i18n.t.search.tracks}</h2>
          <TrackList tracks={listed} showHeader={false} />
        </section>
      </div>
    {/if}
  {/if}
</div>

<style>
  .search-view {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 20px 12px 40px;
  }

  .idle {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 24px 12px;
  }

  .idle h1 {
    font-size: 24px;
    font-weight: 700;
  }

  .idle p {
    color: var(--text-muted);
  }

  .providers {
    display: flex;
    gap: 16px;
    margin-top: 8px;
  }

  .filters {
    display: flex;
    gap: 8px;
    padding: 0 12px;
  }

  .chip {
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--surface-hover);
    font-size: 13.5px;
    font-weight: 500;
    transition:
      background-color var(--fast) var(--ease),
      color var(--fast) var(--ease);
  }

  .chip:hover {
    background: var(--surface-active);
  }

  .chip[aria-selected="true"] {
    background: var(--text);
    color: var(--bg);
  }

  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 12px;
    padding: 10px 14px;
    border-radius: var(--radius-card);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--text);
    font-size: 13px;
  }

  .notice :global(svg) {
    flex: none;
    color: var(--danger);
    font-size: 18px;
  }

  .results {
    display: flex;
    flex-direction: column;
    gap: 24px;
    transition: opacity var(--base) var(--ease);
  }

  .results.stale {
    opacity: 0.55;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h2 {
    padding: 0 12px;
    font-size: 22px;
    font-weight: 700;
  }

  .top-card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 20px;
    max-width: 560px;
    margin: 0 12px;
    padding: 20px;
    border-radius: var(--radius-card);
    background: var(--surface-raised);
    transition: background-color var(--base) var(--ease);
  }

  .top-card:hover {
    background: var(--surface-hover);
  }

  .top-text {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .top-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .top-artists {
    color: var(--text-muted);
  }

  .top-play {
    position: absolute;
    right: 20px;
    bottom: 20px;
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 20px;
    box-shadow: 0 8px 20px rgb(0 0 0 / 0.3);
    opacity: 0;
    transition:
      opacity var(--base) var(--ease),
      transform var(--fast) var(--ease);
  }

  .top-card:hover .top-play,
  .top-play:focus-visible {
    opacity: 1;
  }

  .top-play:active {
    transform: scale(0.94);
  }

  .skeletons {
    display: flex;
    flex-direction: column;
  }

  .skeleton-row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 64px;
    padding: 0 12px 0 68px;
  }

  .skeleton-lines {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>

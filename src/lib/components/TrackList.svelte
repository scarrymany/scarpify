<script lang="ts">
  import Heart from "phosphor-svelte/lib/Heart";
  import ListPlus from "phosphor-svelte/lib/ListPlus";
  import Clock from "phosphor-svelte/lib/Clock";
  import Artwork from "./Artwork.svelte";
  import PlayPauseIcon from "./PlayPauseIcon.svelte";
  import ProviderBadge from "./ProviderBadge.svelte";
  import NowPlayingBars from "./NowPlayingBars.svelte";
  import { formatDuration, i18n } from "$lib/i18n/index.svelte";
  import { library } from "$lib/state/library.svelte";
  import { player } from "$lib/state/player.svelte";
  import { trackKey, type Track } from "$lib/types";

  interface Props {
    tracks: Track[];
    showAlbum?: boolean;
    showHeader?: boolean;
  }

  let { tracks, showAlbum = true, showHeader = true }: Props = $props();

  const currentKey = $derived(player.current ? trackKey(player.current) : null);

  function play(index: number) {
    const track = tracks[index];
    if (currentKey === trackKey(track)) {
      player.toggle();
      return;
    }
    player.playTracks(tracks, index);
  }
</script>

<div class="list" class:no-album={!showAlbum} role="list">
  {#if showHeader}
    <div class="row header" aria-hidden="true">
      <span class="index">#</span>
      <span>{i18n.t.track.title}</span>
      {#if showAlbum}<span>{i18n.t.track.album}</span>{/if}
      <span class="duration"><Clock /></span>
    </div>
  {/if}

  {#each tracks as track, index (trackKey(track) + index)}
    {@const active = currentKey === trackKey(track)}
    {@const playing = active && player.status === "playing"}
    {@const liked = library.isLiked(track)}
    <div
      class="row"
      class:active
      role="listitem"
      ondblclick={() => play(index)}
      onpointerenter={() => player.prefetchSoon(track)}
      onpointerleave={() => player.cancelPrefetch()}
    >
      <span class="index">
        {#if playing}
          <span class="bars"><NowPlayingBars /></span>
        {:else}
          <span class="number">{index + 1}</span>
        {/if}
        <button
          class="row-play"
          aria-label={playing ? i18n.t.track.pause : i18n.t.track.play}
          onclick={() => play(index)}
        >
          <PlayPauseIcon {playing} />
        </button>
      </span>

      <span class="main">
        <Artwork src={track.artwork} size="44px" />
        <span class="text">
          <span class="title">{track.title}</span>
          <span class="artists">{track.artists.join(", ")}</span>
          <ProviderBadge provider={track.provider} />
        </span>
      </span>

      {#if showAlbum}
        <span class="album">{track.album ?? ""}</span>
      {/if}

      <span class="duration">
        <button
          class="icon-button hover-only"
          aria-label={i18n.t.track.addToQueue}
          title={i18n.t.track.addToQueue}
          onclick={() => player.enqueue(track)}
        >
          <ListPlus />
        </button>
        <button
          class="icon-button like"
          class:hover-only={!liked}
          aria-pressed={liked}
          aria-label={liked ? i18n.t.track.unlike : i18n.t.track.like}
          title={liked ? i18n.t.track.unlike : i18n.t.track.like}
          onclick={() => library.toggleLike(track)}
        >
          <Heart weight={liked ? "fill" : "regular"} />
        </button>
        <span class="time">{formatDuration(track.durationMs)}</span>
      </span>
    </div>
  {/each}
</div>

<style>
  .list {
    --columns: 40px minmax(240px, 5fr) minmax(120px, 3fr) 152px;
    display: flex;
    flex-direction: column;
  }

  .list.no-album {
    --columns: 40px minmax(240px, 1fr) 152px;
  }

  .row {
    display: grid;
    grid-template-columns: var(--columns);
    align-items: center;
    gap: 16px;
    min-height: 64px;
    padding: 0 12px;
    border-radius: var(--radius-card);
    transition: background-color var(--fast) var(--ease);
  }

  .row:not(.header):hover {
    background: var(--surface-hover);
  }

  .header {
    min-height: 36px;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--line);
    border-radius: 0;
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .index {
    position: relative;
    display: grid;
    place-items: center;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .row-play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--text);
    opacity: 0;
  }

  .row:hover .row-play,
  .row-play:focus-visible {
    opacity: 1;
  }

  .row:hover .number,
  .row:hover .bars,
  .row:has(.row-play:focus-visible) .number {
    opacity: 0;
  }

  .main {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .title,
  .artists,
  .album {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: 15px;
    font-weight: 500;
  }

  .active .title {
    color: var(--accent);
  }

  .artists,
  .album {
    color: var(--text-muted);
    font-size: 13px;
  }

  .duration {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    color: var(--text-muted);
  }

  .time {
    min-width: 44px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .hover-only {
    opacity: 0;
  }

  .row:hover .hover-only,
  .hover-only:focus-visible {
    opacity: 1;
  }

  .like[aria-pressed="true"] {
    color: var(--accent);
  }
</style>

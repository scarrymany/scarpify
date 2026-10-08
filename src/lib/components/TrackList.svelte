<script lang="ts">
  import ListPlus from "phosphor-svelte/lib/ListPlus";
  import Clock from "phosphor-svelte/lib/Clock";
  import Queue from "phosphor-svelte/lib/Queue";
  import Plus from "phosphor-svelte/lib/Plus";
  import Heart from "phosphor-svelte/lib/Heart";
  import Trash from "phosphor-svelte/lib/Trash";
  import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
  import MusicNotesPlus from "phosphor-svelte/lib/MusicNotesPlus";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Artwork from "./Artwork.svelte";
  import LikeButton from "./LikeButton.svelte";
  import PlayPauseIcon from "./PlayPauseIcon.svelte";
  import ProviderBadge from "./ProviderBadge.svelte";
  import NowPlayingBars from "./NowPlayingBars.svelte";
  import { errorMessage } from "$lib/api";
  import { format, formatDuration, i18n } from "$lib/i18n/index.svelte";
  import { sortable } from "$lib/sortable";
  import { library } from "$lib/state/library.svelte";
  import { menu, type MenuItem } from "$lib/state/menu.svelte";
  import { player } from "$lib/state/player.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { trackKey, type Track } from "$lib/types";

  interface Props {
    tracks: Track[];
    showAlbum?: boolean;
    showHeader?: boolean;
    /** Set for the user's own playlists: rows can be removed and dragged into a new order. */
    playlistId?: string;
  }

  let { tracks, showAlbum = true, showHeader = true, playlistId }: Props = $props();

  /** Enough rows to fill the screen in the first frame; the rest arrive in later frames. */
  const FIRST_BATCH = 40;
  const BATCH = 60;

  let rendered = $state(FIRST_BATCH);
  const visible = $derived(tracks.slice(0, rendered));

  // Building hundreds of rows at once stalls the frame that opens a big playlist.
  $effect(() => {
    const total = tracks.length;
    let count = Math.min(total, FIRST_BATCH);
    rendered = count;
    let frame = 0;
    const grow = () => {
      count = Math.min(total, count + BATCH);
      rendered = count;
      if (count < total) frame = requestAnimationFrame(grow);
    };
    if (count < total) frame = requestAnimationFrame(grow);
    return () => cancelAnimationFrame(frame);
  });

  const currentKey = $derived(player.current ? trackKey(player.current) : null);

  function play(index: number) {
    const track = tracks[index];
    if (currentKey === trackKey(track)) {
      player.toggle();
      return;
    }
    player.playTracks(tracks, index);
  }

  async function addToNewPlaylist(track: Track) {
    try {
      const playlist = await library.createPlaylist();
      await library.addTracks(playlist.id, [track]);
    } catch (error) {
      toasts.error(format(i18n.t.errors.generic, { error: errorMessage(error) }));
    }
  }

  function trackActions(track: Track): MenuItem[] {
    const t = i18n.t;
    const liked = library.isLiked(track);
    const playlists: MenuItem[] = [
      { label: t.playlist.newPlaylist, icon: Plus, action: () => void addToNewPlaylist(track) },
      ...library.userPlaylists
        .filter((p) => p.id !== playlistId)
        .map((p) => ({ label: p.name, action: () => void library.addTracks(p.id, [track]) })),
    ];
    const items: MenuItem[] = [
      { label: t.track.addToQueue, icon: Queue, action: () => player.enqueue(track) },
      { label: t.playlist.addTo, icon: MusicNotesPlus, submenu: playlists },
      { label: liked ? t.track.unlike : t.track.like, icon: Heart, action: () => library.toggleLike(track) },
    ];
    if (playlistId) {
      items.push({
        label: t.playlist.removeFrom,
        icon: Trash,
        danger: true,
        action: () => library.removeTrack(playlistId, track),
      });
    }
    if (track.url) {
      const url = track.url;
      items.push({
        label: format(t.track.openSource, { provider: t.providers[track.provider] }),
        icon: ArrowSquareOut,
        action: () => void openUrl(url),
      });
    }
    return items;
  }
</script>

<div
  class="list"
  class:no-album={!showAlbum}
  role="list"
  use:sortable={{ onmove: (from, to) => playlistId && library.moveTrack(playlistId, from, to), disabled: !playlistId }}
>
  {#if showHeader}
    <div class="row header" aria-hidden="true">
      <span class="index">#</span>
      <span>{i18n.t.track.title}</span>
      {#if showAlbum}<span>{i18n.t.track.album}</span>{/if}
      <span class="duration"><Clock /></span>
    </div>
  {/if}

  {#each visible as track, index (trackKey(track) + index)}
    {@const active = currentKey === trackKey(track)}
    {@const playing = active && player.status === "playing"}
    {@const liked = library.isLiked(track)}
    <div
      class="row"
      class:active
      role="listitem"
      data-sortable={playlistId ? "" : undefined}
      ondblclick={() => play(index)}
      oncontextmenu={(event) => menu.show(event, trackActions(track))}
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
        <span class="like-slot" class:hover-only={!liked}>
          <LikeButton {track} />
        </span>
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
    transition:
      background-color var(--fast) var(--ease),
      transform var(--base) var(--ease);
    /* Long playlists: rows outside the viewport skip layout and paint entirely. */
    content-visibility: auto;
    contain-intrinsic-size: auto 64px;
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

  .like-slot {
    display: inline-grid;
  }
</style>

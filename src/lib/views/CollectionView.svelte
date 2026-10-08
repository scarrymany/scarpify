<script lang="ts">
  import Shuffle from "phosphor-svelte/lib/Shuffle";
  import Trash from "phosphor-svelte/lib/Trash";
  import Artwork from "$lib/components/Artwork.svelte";
  import PlayPauseIcon from "$lib/components/PlayPauseIcon.svelte";
  import ProviderBadge from "$lib/components/ProviderBadge.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { player } from "$lib/state/player.svelte";
  import { trackKey, type Provider, type Track } from "$lib/types";

  interface Props {
    kind: string;
    name: string;
    owner?: string | null;
    artwork: string | null;
    provider?: Provider;
    tracks: Track[];
    fallback?: "note" | "heart";
    emptyTitle: string;
    emptyBody?: string;
    onremove?: () => void;
  }

  let { kind, name, owner = null, artwork, provider, tracks, fallback = "note", emptyTitle, emptyBody, onremove }: Props = $props();

  const keys = $derived(new Set(tracks.map(trackKey)));
  // Spotify playlist embeds carry no album names; an empty column only wastes width.
  const hasAlbums = $derived(tracks.some((t) => t.album));
  const playingHere = $derived(
    player.status === "playing" && player.current !== null && keys.has(trackKey(player.current)),
  );

  function playAll() {
    if (playingHere) {
      player.toggle();
      return;
    }
    player.playTracks(tracks);
  }

  function shufflePlay() {
    if (!player.shuffle) player.toggleShuffle();
    player.playTracks(tracks, Math.floor(Math.random() * tracks.length));
  }
</script>

<div class="collection">
  <header class="hero">
    {#if artwork}
      <img class="backdrop" src={artwork} alt="" aria-hidden="true" referrerpolicy="no-referrer" />
    {/if}
    <div class="hero-content">
      <Artwork src={artwork} size="208px" radius="var(--radius-card)" {fallback} eager />
      <div class="info">
        <span class="kind">{kind}</span>
        <h1 title={name}>{name}</h1>
        <div class="meta">
          {#if owner}<span class="owner">{owner}</span>{/if}
          <span>{i18n.plural(i18n.t.plural.tracks, tracks.length)}</span>
        </div>
        {#if provider}
          <span class="source">
            {i18n.t.collection.importedFrom}
            <ProviderBadge {provider} />
          </span>
        {/if}
      </div>
    </div>
  </header>

  <div class="actions">
    <button
      class="play"
      aria-label={playingHere ? i18n.t.player.pause : i18n.t.player.play}
      disabled={tracks.length === 0}
      onclick={playAll}
    >
      <PlayPauseIcon playing={playingHere} />
    </button>
    <button
      class="icon-button large"
      aria-label={i18n.t.collection.shuffle}
      title={i18n.t.collection.shuffle}
      disabled={tracks.length === 0}
      onclick={shufflePlay}
    >
      <Shuffle />
    </button>
    {#if onremove}
      <button class="icon-button large" aria-label={i18n.t.collection.remove} title={i18n.t.collection.remove} onclick={onremove}>
        <Trash />
      </button>
    {/if}
  </div>

  {#if tracks.length === 0}
    <div class="empty">
      <h2>{emptyTitle}</h2>
      {#if emptyBody}<p>{emptyBody}</p>{/if}
    </div>
  {:else}
    <div class="tracks">
      <TrackList {tracks} showAlbum={hasAlbums} />
    </div>
  {/if}
</div>

<style>
  .collection {
    padding-bottom: 40px;
  }

  .hero {
    position: relative;
    overflow: hidden;
    padding: 56px 24px 24px;
  }

  /* A blurred copy of the cover tints the header without sampling pixels. */
  .backdrop {
    position: absolute;
    inset: -40px;
    width: calc(100% + 80px);
    height: calc(100% + 80px);
    object-fit: cover;
    filter: blur(60px) saturate(1.4);
    opacity: 0.45;
    pointer-events: none;
    /* Own compositor layer: the blur is rasterized once instead of on every scroll frame. */
    will-change: transform;
  }

  .hero::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, transparent 30%, var(--surface));
    pointer-events: none;
  }

  .hero-content {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: flex-end;
    gap: 24px;
  }

  .hero-content > :global(.artwork) {
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.45);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .kind {
    font-size: 13px;
    font-weight: 600;
  }

  h1 {
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    font-size: clamp(28px, 4.2vw, 64px);
    font-weight: 800;
    line-height: 1.08;
    letter-spacing: -0.03em;
    padding-bottom: 4px;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--text-muted);
    font-size: 14px;
  }

  .owner {
    color: var(--text);
    font-weight: 600;
  }

  .owner::after {
    content: "·";
    margin-left: 6px;
    color: var(--text-muted);
  }

  .source {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-faint);
    font-size: 12px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px 24px;
  }

  .play {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 22px;
    transition:
      transform var(--fast) var(--ease),
      background-color var(--fast) var(--ease);
  }

  .play:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 88%, white);
  }

  .play:active:not(:disabled) {
    transform: scale(0.94);
  }

  .large {
    width: 44px;
    height: 44px;
    font-size: 26px;
  }

  .tracks {
    padding: 0 12px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 24px;
  }

  .empty h2 {
    font-size: 22px;
    font-weight: 700;
  }

  .empty p {
    color: var(--text-muted);
  }
</style>

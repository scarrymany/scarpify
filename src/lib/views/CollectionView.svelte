<script lang="ts">
  import { tick } from "svelte";
  import Shuffle from "phosphor-svelte/lib/Shuffle";
  import DotsThree from "phosphor-svelte/lib/DotsThree";
  import PencilSimple from "phosphor-svelte/lib/PencilSimple";
  import Artwork from "$lib/components/Artwork.svelte";
  import CollectionCover from "$lib/components/CollectionCover.svelte";
  import PlayPauseIcon from "$lib/components/PlayPauseIcon.svelte";
  import ProviderBadge from "$lib/components/ProviderBadge.svelte";
  import TrackList from "$lib/components/TrackList.svelte";
  import { changeCover, collectionActions } from "$lib/collectionActions";
  import { i18n } from "$lib/i18n/index.svelte";
  import { library } from "$lib/state/library.svelte";
  import { menu } from "$lib/state/menu.svelte";
  import { player } from "$lib/state/player.svelte";
  import { ui } from "$lib/state/ui.svelte";
  import { trackKey, type SavedCollection, type Track } from "$lib/types";

  /** Either a saved playlist/album, or the liked tracks described by the other props. */
  interface Props {
    collection?: SavedCollection;
    title?: string;
    tracks?: Track[];
    emptyTitle?: string;
    emptyBody?: string;
  }

  let { collection, title = "", tracks: plainTracks = [], emptyTitle = "", emptyBody }: Props = $props();

  const t = $derived(i18n.t);
  const isUser = $derived(collection?.origin === "user");
  const tracks = $derived(collection?.tracks ?? plainTracks);
  const name = $derived(collection?.name ?? title);
  const kind = $derived(collection?.kind === "album" ? t.collection.album : t.collection.playlist);
  const backdrop = $derived(collection?.customArtwork ?? collection?.artwork ?? tracks[0]?.artwork ?? null);

  const keys = $derived(new Set(tracks.map(trackKey)));
  // Spotify playlist embeds carry no album names; an empty column only wastes width.
  const hasAlbums = $derived(tracks.some((track) => track.album));
  const playingHere = $derived(
    player.status === "playing" && player.current !== null && keys.has(trackKey(player.current)),
  );

  let titleInput = $state<HTMLInputElement>();
  let draftName = $state("");
  const renaming = $derived(isUser && collection !== undefined && ui.renaming === collection.id);

  $effect(() => {
    if (!renaming) return;
    draftName = name;
    void tick().then(() => {
      titleInput?.focus();
      titleInput?.select();
    });
  });

  function finishRename(save: boolean) {
    if (!renaming || !collection) return;
    if (save) library.rename(collection.id, draftName);
    ui.renaming = null;
  }

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
    {#if backdrop}
      <img class="backdrop" src={backdrop} alt="" aria-hidden="true" referrerpolicy="no-referrer" />
    {/if}
    <div class="hero-content">
      {#if collection}
        <button class="cover" aria-label={t.playlist.changeCover} title={t.playlist.changeCover} onclick={() => changeCover(collection)}>
          <CollectionCover {collection} size="208px" radius="var(--radius-card)" eager />
          <span class="cover-overlay"><PencilSimple /><span>{t.playlist.changeCover}</span></span>
        </button>
      {:else}
        <Artwork src={null} size="208px" radius="var(--radius-card)" fallback="heart" eager />
      {/if}
      <div class="info">
        <span class="kind">{kind}</span>
        {#if renaming}
          <input
            bind:this={titleInput}
            bind:value={draftName}
            class="title-input"
            maxlength="100"
            aria-label={t.playlist.rename}
            onkeydown={(event) => {
              if (event.key === "Enter") finishRename(true);
              if (event.key === "Escape") finishRename(false);
            }}
            onblur={() => finishRename(true)}
          />
        {:else if isUser && collection}
          {@const id = collection.id}
          <button class="title-button" title={t.playlist.rename} onclick={() => (ui.renaming = id)}>
            <h1>{name}</h1>
          </button>
        {:else}
          <h1 title={name}>{name}</h1>
        {/if}
        <div class="meta">
          {#if collection?.owner}<span class="owner">{collection.owner}</span>{/if}
          <span>{i18n.plural(t.plural.tracks, tracks.length)}</span>
        </div>
        {#if collection?.provider}
          <span class="source">
            {t.collection.importedFrom}
            <ProviderBadge provider={collection.provider} />
          </span>
        {/if}
      </div>
    </div>
  </header>

  <div class="actions">
    <button
      class="play"
      aria-label={playingHere ? t.player.pause : t.player.play}
      disabled={tracks.length === 0}
      onclick={playAll}
    >
      <PlayPauseIcon playing={playingHere} />
    </button>
    <button
      class="icon-button large"
      aria-label={t.collection.shuffle}
      title={t.collection.shuffle}
      disabled={tracks.length === 0}
      onclick={shufflePlay}
    >
      <Shuffle />
    </button>
    {#if collection}
      {@const saved = collection}
      <button
        class="icon-button large"
        aria-label={t.playlist.more}
        title={t.playlist.more}
        onclick={(event) => menu.showAt(event.currentTarget, collectionActions(saved))}
      >
        <DotsThree weight="bold" />
      </button>
    {/if}
  </div>

  {#if tracks.length === 0}
    <div class="empty">
      <h2>{isUser ? t.playlist.emptyTitle : collection ? t.collection.empty : emptyTitle}</h2>
      {#if isUser}<p>{t.playlist.emptyBody}</p>{:else if emptyBody}<p>{emptyBody}</p>{/if}
    </div>
  {:else}
    <div class="tracks">
      <TrackList {tracks} showAlbum={hasAlbums} playlistId={isUser ? collection?.id : undefined} />
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

  .hero-content > :global(.artwork),
  .cover {
    flex: none;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.45);
  }

  .cover {
    position: relative;
    border-radius: var(--radius-card);
    overflow: hidden;
  }

  .cover-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: rgb(0 0 0 / 0.55);
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    opacity: 0;
    transition: opacity var(--base) var(--ease);
  }

  .cover-overlay :global(svg) {
    font-size: 36px;
  }

  .cover:hover .cover-overlay,
  .cover:focus-visible .cover-overlay {
    opacity: 1;
  }

  .title-button {
    min-width: 0;
    text-align: left;
    cursor: text;
  }

  .title-input {
    width: 100%;
    min-width: 0;
    padding: 0 0 4px;
    border: 0;
    border-bottom: 2px solid var(--accent);
    outline: 0;
    background: none;
    font-size: clamp(28px, 4.2vw, 64px);
    font-weight: 800;
    line-height: 1.08;
    letter-spacing: -0.03em;
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

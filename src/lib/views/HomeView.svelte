<script lang="ts">
  import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
  import Card from "$lib/components/Card.svelte";
  import Shelf from "$lib/components/Shelf.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { collectionActions } from "$lib/collectionActions";
  import { library } from "$lib/state/library.svelte";
  import { menu } from "$lib/state/menu.svelte";
  import { nav } from "$lib/state/nav.svelte";
  import { player } from "$lib/state/player.svelte";
  import { trackKey, type Track } from "$lib/types";

  interface Props {
    onsearch: () => void;
    onimport: () => void;
  }

  let { onsearch, onimport }: Props = $props();

  const greeting = $derived.by(() => {
    const hour = new Date().getHours();
    if (hour < 5) return i18n.t.home.night;
    if (hour < 12) return i18n.t.home.morning;
    if (hour < 18) return i18n.t.home.afternoon;
    if (hour < 23) return i18n.t.home.evening;
    return i18n.t.home.night;
  });

  const isEmpty = $derived(
    library.recent.length === 0 && library.liked.length === 0 && library.collections.length === 0,
  );

  function isPlaying(track: Track): boolean {
    return player.status === "playing" && player.current !== null && trackKey(player.current) === trackKey(track);
  }
</script>

<div class="home">
  <h1>{greeting}</h1>

  {#if isEmpty}
    <div class="empty">
      <h2>{i18n.t.home.emptyTitle}</h2>
      <p>{i18n.t.home.emptyBody}</p>
      <div class="actions">
        <button class="pill-button primary" onclick={onsearch}>
          <MagnifyingGlass weight="bold" />{i18n.t.home.searchAction}
        </button>
        <button class="pill-button secondary" onclick={onimport}>{i18n.t.importer.action}</button>
      </div>
    </div>
  {:else}
    {#if library.recent.length > 0}
      <Shelf title={i18n.t.home.recent}>
        {#each library.recent as track, index (trackKey(track))}
          <Card
            title={track.title}
            subtitle={track.artists.join(", ")}
            artwork={track.artwork}
            provider={track.provider}
            playing={isPlaying(track)}
            onplay={() => (isPlaying(track) ? player.toggle() : player.playTracks(library.recent, index))}
          />
        {/each}
      </Shelf>
    {/if}

    <Shelf title={i18n.t.home.playlists}>
      <Card
        title={i18n.t.nav.liked}
        subtitle={i18n.plural(i18n.t.plural.tracks, library.liked.length)}
        artwork={null}
        fallback="heart"
        onopen={() => nav.go({ name: "liked" })}
        onplay={() => player.playTracks(library.liked)}
      />
      {#each library.collections as collection (collection.id)}
        <Card
          title={collection.name}
          subtitle={i18n.plural(i18n.t.plural.tracks, collection.tracks.length)}
          artwork={collection.customArtwork ?? collection.artwork ?? collection.tracks[0]?.artwork ?? null}
          provider={collection.provider ?? undefined}
          oncontextmenu={(event) => menu.show(event, collectionActions(collection))}
          onopen={() => nav.go({ name: "collection", id: collection.id })}
          onplay={() => player.playTracks(collection.tracks)}
        />
      {/each}
    </Shelf>
  {/if}
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 32px;
    padding: 24px 12px 40px;
  }

  h1 {
    padding: 0 12px;
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    max-width: 520px;
    padding: 8px 12px;
  }

  .empty h2 {
    font-size: 22px;
    font-weight: 700;
  }

  .empty p {
    color: var(--text-muted);
    line-height: 1.55;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }
</style>

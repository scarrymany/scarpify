<script lang="ts">
  import Artwork from "./Artwork.svelte";
  import type { SavedCollection } from "$lib/types";

  interface Props {
    collection: SavedCollection;
    size: string;
    radius?: string;
    eager?: boolean;
  }

  let { collection, size, radius = "var(--radius-small)", eager = false }: Props = $props();

  const MOSAIC_TILES = 4;

  const cover = $derived(collection.customArtwork ?? collection.artwork);
  /** Playlists made in the app have no cover of their own until one is picked. */
  const mosaic = $derived.by(() => {
    if (cover) return [];
    const artworks = [...new Set(collection.tracks.map((t) => t.artwork).filter((a): a is string => !!a))];
    return artworks.length >= MOSAIC_TILES ? artworks.slice(0, MOSAIC_TILES) : artworks.slice(0, 1);
  });
</script>

{#if mosaic.length === MOSAIC_TILES}
  <div class="mosaic" style:width={size} style:height={size} style:border-radius={radius}>
    {#each mosaic as tile (tile)}
      <Artwork src={tile} size="100%" radius="0" {eager} />
    {/each}
  </div>
{:else}
  <Artwork src={cover ?? mosaic[0] ?? null} {size} {radius} {eager} />
{/if}

<style>
  .mosaic {
    display: grid;
    flex: none;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    overflow: hidden;
  }
</style>

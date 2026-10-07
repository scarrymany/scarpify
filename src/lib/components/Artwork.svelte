<script lang="ts">
  import MusicNotes from "phosphor-svelte/lib/MusicNotes";
  import Heart from "phosphor-svelte/lib/Heart";

  interface Props {
    src: string | null;
    size?: string;
    radius?: string;
    /** Placeholder glyph when there is no image. */
    fallback?: "note" | "heart";
    eager?: boolean;
  }

  let { src, size = "48px", radius = "var(--radius-small)", fallback = "note", eager = false }: Props = $props();

  // Keyed by URL so a reused component never shows the previous image's state.
  let loadedSrc = $state<string | null>(null);
  let failedSrc = $state<string | null>(null);

  const loaded = $derived(src !== null && loadedSrc === src);
  const failed = $derived(src !== null && failedSrc === src);
</script>

<div class="artwork" class:heart={fallback === "heart"} style:width={size} style:height={size} style:border-radius={radius}>
  {#if src && !failed}
    <img
      {src}
      alt=""
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      referrerpolicy="no-referrer"
      draggable="false"
      class:loaded
      onload={() => (loadedSrc = src)}
      onerror={() => (failedSrc = src)}
    />
  {/if}
  {#if !src || failed || !loaded}
    <span class="placeholder" aria-hidden="true">
      {#if fallback === "heart"}
        <Heart weight="fill" />
      {:else}
        <MusicNotes />
      {/if}
    </span>
  {/if}
</div>

<style>
  .artwork {
    position: relative;
    flex: none;
    overflow: hidden;
    background: var(--surface-active);
    container-type: size;
  }

  .heart {
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 85%, black), color-mix(in srgb, var(--accent) 45%, var(--surface)));
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity var(--slow) var(--ease);
  }

  img.loaded {
    opacity: 1;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--text-faint);
    font-size: 36cqmin;
  }

  .heart .placeholder {
    color: rgb(255 255 255 / 0.92);
  }
</style>

<script lang="ts">
  import Artwork from "./Artwork.svelte";
  import PlayPauseIcon from "./PlayPauseIcon.svelte";
  import ProviderBadge from "./ProviderBadge.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import type { Provider } from "$lib/types";

  interface Props {
    title: string;
    subtitle: string;
    artwork: string | null;
    provider?: Provider;
    fallback?: "note" | "heart";
    playing?: boolean;
    onplay: () => void;
    /** Card body click; defaults to playing. */
    onopen?: () => void;
    oncontextmenu?: (event: MouseEvent) => void;
  }

  let { title, subtitle, artwork, provider, fallback = "note", playing = false, onplay, onopen, oncontextmenu }: Props =
    $props();
</script>

<article class="card" class:playing {oncontextmenu}>
  <button class="open" aria-label={title} onclick={onopen ?? onplay}></button>
  <div class="cover">
    <Artwork src={artwork} size="100%" radius="var(--radius-card)" {fallback} />
    <button
      class="play"
      aria-label={playing ? i18n.t.player.pause : i18n.t.player.play}
      onclick={onplay}
    >
      <PlayPauseIcon {playing} />
    </button>
  </div>
  <h3 class="title">{title}</h3>
  <p class="subtitle">{subtitle}</p>
  {#if provider}
    <ProviderBadge {provider} />
  {/if}
</article>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 12px;
    border-radius: var(--radius-card);
    transition: background-color var(--base) var(--ease);
  }

  .card:hover,
  .card:focus-within {
    background: var(--surface-hover);
  }

  .open {
    position: absolute;
    inset: 0;
    border-radius: inherit;
  }

  .cover {
    position: relative;
    aspect-ratio: 1;
    margin-bottom: 6px;
    pointer-events: none;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.28);
    border-radius: var(--radius-card);
  }

  .play {
    position: absolute;
    right: 8px;
    bottom: 8px;
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 20px;
    box-shadow: 0 8px 20px rgb(0 0 0 / 0.35);
    pointer-events: auto;
    opacity: 0;
    transition:
      opacity var(--base) var(--ease),
      transform var(--fast) var(--ease);
  }

  .card:hover .play,
  .card:focus-within .play,
  .card.playing .play {
    opacity: 1;
  }

  .play:active {
    transform: scale(0.94);
  }

  .title,
  .subtitle {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    pointer-events: none;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
  }

  .playing .title {
    color: var(--accent);
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 13px;
  }
</style>

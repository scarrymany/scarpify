<script lang="ts">
  import X from "phosphor-svelte/lib/X";
  import Artwork from "./Artwork.svelte";
  import ProviderBadge from "./ProviderBadge.svelte";
  import { i18n } from "$lib/i18n/index.svelte";
  import { player } from "$lib/state/player.svelte";
  import { trackKey, type Track } from "$lib/types";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
</script>

{#snippet item(track: Track, index: number, active: boolean)}
  <button class="item" class:active ondblclick={() => player.jumpTo(index)} onclick={() => active && player.toggle()}>
    <Artwork src={track.artwork} size="44px" />
    <span class="text">
      <span class="title">{track.title}</span>
      <span class="artists">{track.artists.join(", ")}</span>
      <ProviderBadge provider={track.provider} />
    </span>
  </button>
{/snippet}

<aside class="queue">
  <header>
    <h2>{i18n.t.queue.title}</h2>
    <button class="icon-button" aria-label={i18n.t.window.close} onclick={onclose}><X /></button>
  </header>

  <div class="scroll">
    {#if player.current}
      <h3>{i18n.t.queue.nowPlaying}</h3>
      {@render item(player.current, player.index, true)}

      <div class="next-header">
        <h3>{i18n.t.queue.next}</h3>
        {#if player.upcoming.length > 0}
          <button class="link" onclick={() => player.clearUpcoming()}>{i18n.t.queue.clear}</button>
        {/if}
      </div>
      {#each player.upcoming as track, offset (trackKey(track) + offset)}
        {@render item(track, player.index + 1 + offset, false)}
      {:else}
        <p class="empty">{i18n.t.queue.empty}</p>
      {/each}
    {:else}
      <p class="empty">{i18n.t.queue.empty}</p>
    {/if}
  </div>
</aside>

<style>
  .queue {
    grid-area: queue;
    display: flex;
    flex-direction: column;
    width: var(--queue-width);
    min-height: 0;
    border-radius: var(--radius-panel);
    background: var(--surface);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 12px 8px 20px;
  }

  h2 {
    font-size: 16px;
    font-weight: 700;
  }

  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 16px;
  }

  h3 {
    padding: 12px 12px 8px;
    font-size: 14px;
    font-weight: 600;
  }

  .next-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 12px;
  }

  .link {
    color: var(--text-muted);
    font-size: 12.5px;
    font-weight: 600;
  }

  .link:hover {
    color: var(--text);
  }

  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 12px;
    border-radius: var(--radius-card);
    text-align: left;
    transition: background-color var(--fast) var(--ease);
  }

  .item:hover {
    background: var(--surface-hover);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .title,
  .artists {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-weight: 500;
  }

  .active .title {
    color: var(--accent);
  }

  .artists {
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .empty {
    padding: 12px;
    color: var(--text-faint);
  }
</style>

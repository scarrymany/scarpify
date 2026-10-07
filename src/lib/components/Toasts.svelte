<script lang="ts">
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
  import WarningCircle from "phosphor-svelte/lib/WarningCircle";
  import { motion } from "$lib/motion";
  import { toasts } from "$lib/state/toasts.svelte";
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each toasts.items as toast (toast.id)}
    <button
      class="toast"
      class:error={toast.tone === "error"}
      animate:flip={motion(220)}
      transition:fly={motion(220, { y: 12 })}
      onclick={() => toasts.dismiss(toast.id)}
    >
      {#if toast.tone === "error"}<WarningCircle weight="fill" />{/if}
      <span class="message">{toast.message}</span>
    </button>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: calc(var(--playerbar-height) + 16px);
    z-index: var(--layer-toast);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    translate: -50% 0;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(560px, calc(100vw - 32px));
    padding: 10px 16px;
    border-radius: var(--radius-card);
    background: var(--text);
    color: var(--bg);
    font-weight: 500;
    text-align: left;
    box-shadow: var(--shadow);
    pointer-events: auto;
  }

  .message {
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  .toast :global(svg) {
    flex: none;
    font-size: 18px;
  }

  /* The toast inverts the theme, so it borrows the opposite theme's danger color. */
  .toast.error :global(svg) {
    color: #c62828;
  }

  :global(:root[data-theme="light"]) .toast.error :global(svg) {
    color: #f2777a;
  }
</style>

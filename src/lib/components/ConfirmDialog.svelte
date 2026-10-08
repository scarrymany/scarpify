<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { i18n } from "$lib/i18n/index.svelte";
  import { motion } from "$lib/motion";
  import { ui } from "$lib/state/ui.svelte";

  let confirmButton = $state<HTMLButtonElement>();

  $effect(() => {
    if (ui.confirmRequest) confirmButton?.focus();
  });

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && ui.confirmRequest) ui.answer(false);
  }
</script>

<svelte:window {onkeydown} />

{#if ui.confirmRequest}
  {@const request = ui.confirmRequest}
  <div class="backdrop" transition:fade={motion(180)} onclick={() => ui.answer(false)} aria-hidden="true"></div>
  <div
    class="dialog"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    aria-describedby="confirm-body"
    transition:scale={motion(220, { start: 0.97 })}
  >
    <h2 id="confirm-title">{request.title}</h2>
    <p id="confirm-body">{request.body}</p>
    <div class="actions">
      <button class="pill-button secondary" onclick={() => ui.answer(false)}>{i18n.t.importer.cancel}</button>
      <button bind:this={confirmButton} class="pill-button danger" onclick={() => ui.answer(true)}>
        {request.confirmLabel}
      </button>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: var(--layer-dialog);
    background: rgb(0 0 0 / 0.55);
  }

  .dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    z-index: var(--layer-dialog);
    width: min(420px, calc(100vw - 32px));
    padding: 24px;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    box-shadow: var(--shadow);
    translate: -50% -50%;
  }

  h2 {
    font-size: 19px;
    font-weight: 700;
    overflow-wrap: anywhere;
  }

  p {
    margin-top: 8px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }

  .danger {
    background: var(--danger);
    color: #fff;
  }

  .danger:hover {
    background: color-mix(in srgb, var(--danger) 88%, black);
  }
</style>

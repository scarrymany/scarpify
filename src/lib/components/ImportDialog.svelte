<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import ProviderBadge from "./ProviderBadge.svelte";
  import { errorMessage } from "$lib/api";
  import { format, i18n } from "$lib/i18n/index.svelte";
  import { library } from "$lib/state/library.svelte";
  import { nav } from "$lib/state/nav.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { motion } from "$lib/motion";
  import type { Provider } from "$lib/types";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  const PROVIDERS: Provider[] = ["spotify", "soundcloud", "youtube"];

  /** Mirrors the backend's link checks so obvious mistakes are caught before a request. */
  const LINK_PATTERNS: Record<Provider, RegExp> = {
    spotify: /^(https?:\/\/open\.spotify\.com\/(intl-[\w-]+\/)?(playlist|album)\/\w+|spotify:(playlist|album):\w+)/,
    soundcloud: /^https?:\/\/(www\.|m\.)?soundcloud\.com\/[^/]+\/sets\/[^/?#]+/,
    youtube: /^https?:\/\/([\w-]+\.)?(youtube\.com|youtu\.be)\/.*([?&]list=[\w-]+|\/browse\/MPREb[\w-]+)/,
  };

  let link = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let input: HTMLInputElement;

  const detected = $derived(PROVIDERS.find((p) => LINK_PATTERNS[p].test(link.trim())) ?? null);

  $effect(() => input.focus());

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const value = link.trim();
    if (!detected) {
      error = i18n.t.importer.invalid;
      return;
    }

    busy = true;
    error = null;
    try {
      const saved = await library.importPlaylist(value);
      toasts.show(format(i18n.t.importer.done, { name: saved.name }));
      nav.go({ name: "collection", id: saved.id });
      onclose();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      busy = false;
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && !busy) onclose();
  }
</script>

<svelte:window {onkeydown} />

<div class="backdrop" transition:fade={motion(180)} onclick={() => !busy && onclose()} aria-hidden="true"></div>
<div class="dialog" role="dialog" aria-modal="true" aria-labelledby="import-title" transition:scale={motion(260, { start: 0.97 })}>
  <div class="providers">
    {#each PROVIDERS as provider (provider)}
      <span class="provider" class:dimmed={detected !== null && detected !== provider}>
        <ProviderBadge {provider} />
      </span>
    {/each}
  </div>
  <h2 id="import-title">{i18n.t.importer.title}</h2>
  <p class="body">{i18n.t.importer.body}</p>

  <form onsubmit={submit}>
    <label for="import-link" class="sr-only">{i18n.t.importer.title}</label>
    <input
      id="import-link"
      bind:this={input}
      bind:value={link}
      oninput={() => (error = null)}
      type="url"
      spellcheck="false"
      autocomplete="off"
      placeholder={i18n.t.importer.placeholder}
      aria-invalid={error !== null}
      aria-describedby={error ? "import-error" : undefined}
      disabled={busy}
    />
    {#if error}
      <p id="import-error" class="error">{error}</p>
    {/if}
    <div class="actions">
      <button type="button" class="pill-button secondary" disabled={busy} onclick={onclose}>{i18n.t.importer.cancel}</button>
      <button type="submit" class="pill-button primary" disabled={busy || link.trim() === ""}>
        {busy ? i18n.t.importer.working : i18n.t.importer.submit}
      </button>
    </div>
  </form>
</div>

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
    width: min(500px, calc(100vw - 32px));
    padding: 28px;
    border-radius: var(--radius-panel);
    background: var(--surface-raised);
    box-shadow: var(--shadow);
    translate: -50% -50%;
  }

  .providers {
    display: flex;
    gap: 16px;
    margin-bottom: 18px;
  }

  .provider {
    transition: opacity var(--base) var(--ease);
  }

  .provider :global(.badge) {
    font-size: 13px;
    color: var(--text-muted);
  }

  .provider :global(svg) {
    width: 16px;
    height: 16px;
  }

  .provider.dimmed {
    opacity: 0.3;
  }

  h2 {
    font-size: 20px;
    font-weight: 700;
  }

  .body {
    margin-top: 8px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 20px;
  }

  input {
    height: 44px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-card);
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--line);
    outline: 0;
    transition: box-shadow var(--fast) var(--ease);
  }

  input:focus {
    box-shadow: inset 0 0 0 2px var(--text);
  }

  input[aria-invalid="true"] {
    box-shadow: inset 0 0 0 2px var(--danger);
  }

  input::placeholder {
    color: var(--text-faint);
  }

  .error {
    color: var(--danger);
    font-size: 13px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 12px;
  }
</style>

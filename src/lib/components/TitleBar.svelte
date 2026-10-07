<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import CaretLeft from "phosphor-svelte/lib/CaretLeft";
  import CaretRight from "phosphor-svelte/lib/CaretRight";
  import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
  import X from "phosphor-svelte/lib/X";
  import Minus from "phosphor-svelte/lib/Minus";
  import Square from "phosphor-svelte/lib/Square";
  import Copy from "phosphor-svelte/lib/Copy";
  import { i18n } from "$lib/i18n/index.svelte";
  import { nav } from "$lib/state/nav.svelte";
  import { search } from "$lib/state/search.svelte";

  const appWindow = getCurrentWindow();
  let maximized = $state(false);
  let input: HTMLInputElement;

  onMount(() => {
    const sync = async () => (maximized = await appWindow.isMaximized());
    void sync();
    const unlisten = appWindow.onResized(sync);
    return () => void unlisten.then((stop) => stop());
  });

  function onSearchInput(event: Event) {
    search.setQuery((event.currentTarget as HTMLInputElement).value);
    nav.go({ name: "search" });
  }

  function onSearchKey(event: KeyboardEvent) {
    if (event.key === "Enter") void search.run();
    if (event.key === "Escape") {
      search.setQuery("");
      input.blur();
    }
  }

  export function focusSearch() {
    input.focus();
    input.select();
  }
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="history" data-tauri-drag-region>
    <button class="icon-button round" aria-label={i18n.t.nav.back} title={i18n.t.nav.back} disabled={!nav.canBack} onclick={() => nav.back()}>
      <CaretLeft weight="bold" />
    </button>
    <button class="icon-button round" aria-label={i18n.t.nav.forward} title={i18n.t.nav.forward} disabled={!nav.canForward} onclick={() => nav.forward()}>
      <CaretRight weight="bold" />
    </button>
  </div>

  <label class="search" class:active={nav.current.name === "search"}>
    <MagnifyingGlass />
    <span class="sr-only">{i18n.t.nav.search}</span>
    <input
      bind:this={input}
      type="search"
      spellcheck="false"
      autocomplete="off"
      placeholder={i18n.t.search.placeholder}
      value={search.query}
      oninput={onSearchInput}
      onkeydown={onSearchKey}
      onfocus={() => nav.go({ name: "search" })}
    />
    {#if search.query}
      <button class="clear" aria-label={i18n.t.search.clear} title={i18n.t.search.clear} onclick={() => search.setQuery("")}>
        <X />
      </button>
    {/if}
  </label>

  <div class="window-controls">
    <button aria-label={i18n.t.window.minimize} title={i18n.t.window.minimize} onclick={() => appWindow.minimize()}>
      <Minus />
    </button>
    <button
      aria-label={maximized ? i18n.t.window.restore : i18n.t.window.maximize}
      title={maximized ? i18n.t.window.restore : i18n.t.window.maximize}
      onclick={() => appWindow.toggleMaximize()}
    >
      {#if maximized}<Copy />{:else}<Square />{/if}
    </button>
    <button class="close" aria-label={i18n.t.window.close} title={i18n.t.window.close} onclick={() => appWindow.close()}>
      <X />
    </button>
  </div>
</header>

<style>
  .titlebar {
    grid-area: titlebar;
    display: grid;
    grid-template-columns: 1fr minmax(240px, 480px) 1fr;
    align-items: center;
    gap: 16px;
    height: var(--titlebar-height);
    padding-left: 12px;
  }

  .history {
    display: flex;
    gap: 8px;
  }

  .round {
    background: var(--surface);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 40px;
    padding: 0 12px 0 16px;
    border-radius: 999px;
    background: var(--surface-raised);
    color: var(--text-muted);
    font-size: 18px;
    box-shadow: inset 0 0 0 1px var(--line);
    transition: box-shadow var(--fast) var(--ease);
  }

  .search:hover {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--text) 18%, transparent);
  }

  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--text);
  }

  input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    font-size: 14px;
  }

  input::placeholder {
    color: var(--text-faint);
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    display: grid;
    place-items: center;
    color: var(--text-muted);
    font-size: 16px;
  }

  .clear:hover {
    color: var(--text);
  }

  .window-controls {
    display: flex;
    justify-self: end;
    align-self: stretch;
  }

  .window-controls button {
    display: grid;
    place-items: center;
    width: 46px;
    color: var(--text-muted);
    font-size: 14px;
    transition: background-color var(--fast) var(--ease);
  }

  .window-controls button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .window-controls .close:hover {
    background: #c42b1c;
    color: #fff;
  }
</style>

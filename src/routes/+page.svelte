<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import PlayerBar from "$lib/components/PlayerBar.svelte";
  import QueuePanel from "$lib/components/QueuePanel.svelte";
  import ImportDialog from "$lib/components/ImportDialog.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import HomeView from "$lib/views/HomeView.svelte";
  import SearchView from "$lib/views/SearchView.svelte";
  import CollectionView from "$lib/views/CollectionView.svelte";
  import SettingsView from "$lib/views/SettingsView.svelte";
  import { errorMessage } from "$lib/api";
  import { format, i18n } from "$lib/i18n/index.svelte";
  import { library } from "$lib/state/library.svelte";
  import { nav } from "$lib/state/nav.svelte";
  import { player } from "$lib/state/player.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { motion } from "$lib/motion";

  let titleBar: TitleBar;
  let queueOpen = $state(false);
  let importOpen = $state(false);

  const route = $derived(nav.current);
  const routeKey = $derived(route.name === "collection" ? `collection:${route.id}` : route.name);
  const collection = $derived(route.name === "collection" ? library.collection(route.id) : undefined);

  onMount(() => {
    player.init().catch((error) => toasts.error(format(i18n.t.errors.generic, { error: errorMessage(error) })));
  });

  $effect(() => {
    if (route.name === "collection" && !collection) {
      nav.forget((r) => r.name === "collection" && r.id === route.id);
    }
  });

  function removeCollection(id: string) {
    library.removeCollection(id);
    nav.forget((r) => r.name === "collection" && r.id === id);
  }

  function isTyping(target: EventTarget | null): boolean {
    return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
  }

  function onkeydown(event: KeyboardEvent) {
    if ((event.ctrlKey && (event.key === "l" || event.key === "f")) || (event.key === "/" && !isTyping(event.target))) {
      event.preventDefault();
      titleBar.focusSearch();
      return;
    }
    if (event.altKey && event.key === "ArrowLeft") nav.back();
    else if (event.altKey && event.key === "ArrowRight") nav.forward();
    else if (event.key === "MediaPlayPause") player.toggle();
    else if (event.key === "MediaTrackNext") player.next();
    else if (event.key === "MediaTrackPrevious") player.previous();
    else if (isTyping(event.target)) return;
    else if (event.code === "Space" && !(event.target instanceof HTMLButtonElement)) {
      event.preventDefault();
      player.toggle();
    } else if (event.ctrlKey && event.key === "ArrowRight") player.next();
    else if (event.ctrlKey && event.key === "ArrowLeft") player.previous();
  }

  /** Mouse back and forward buttons navigate like in a browser. */
  function onmouseup(event: MouseEvent) {
    if (event.button === 3) nav.back();
    if (event.button === 4) nav.forward();
  }
</script>

<svelte:window {onkeydown} {onmouseup} />

<div class="app" class:with-queue={queueOpen} oncontextmenu={(e) => e.preventDefault()} role="presentation">
  <TitleBar bind:this={titleBar} />
  <Sidebar onimport={() => (importOpen = true)} />

  <main class="main">
    {#key routeKey}
      <div class="view" in:fade={motion(220)}>
        {#if route.name === "home"}
          <HomeView onsearch={() => titleBar.focusSearch()} onimport={() => (importOpen = true)} />
        {:else if route.name === "search"}
          <SearchView />
        {:else if route.name === "liked"}
          <CollectionView
            kind={i18n.t.collection.playlist}
            name={i18n.t.nav.liked}
            artwork={null}
            fallback="heart"
            tracks={library.liked}
            emptyTitle={i18n.t.collection.likedEmptyTitle}
            emptyBody={i18n.t.collection.likedEmptyBody}
          />
        {:else if route.name === "collection" && collection}
          <CollectionView
            kind={collection.kind === "album" ? i18n.t.collection.album : i18n.t.collection.playlist}
            name={collection.name}
            owner={collection.owner}
            artwork={collection.artwork}
            provider={collection.provider}
            tracks={collection.tracks}
            emptyTitle={i18n.t.collection.empty}
            onremove={() => removeCollection(collection.id)}
          />
        {:else if route.name === "settings"}
          <SettingsView />
        {/if}
      </div>
    {/key}
  </main>

  <!-- Kept mounted so the column can animate closed instead of snapping. -->
  <div class="queue-slot" class:open={queueOpen} inert={!queueOpen} aria-hidden={!queueOpen}>
    <QueuePanel onclose={() => (queueOpen = false)} />
  </div>

  <PlayerBar {queueOpen} ontogglequeue={() => (queueOpen = !queueOpen)} />
</div>

{#if importOpen}
  <ImportDialog onclose={() => (importOpen = false)} />
{/if}

<Toasts />

<style>
  .app {
    display: grid;
    grid-template-areas:
      "titlebar titlebar titlebar"
      "sidebar main queue"
      "player player player";
    /* Gaps live inside the columns, so a closed queue leaves no extra gutter. */
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr) 0px;
    grid-template-rows: var(--titlebar-height) minmax(0, 1fr) var(--playerbar-height);
    height: 100vh;
    padding: 0 8px;
    transition: grid-template-columns var(--slow) var(--ease);
  }

  .app.with-queue {
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr) calc(var(--queue-width) + 8px);
  }

  .main {
    grid-area: main;
    margin-left: 8px;
    overflow: hidden;
    border-radius: var(--radius-panel);
    background: var(--surface);
  }

  .view {
    height: 100%;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .queue-slot {
    grid-area: queue;
    display: flex;
    min-width: 0;
    min-height: 0;
    padding-left: 8px;
    overflow: hidden;
    opacity: 0;
    transition: opacity var(--base) var(--ease);
  }

  .queue-slot.open {
    opacity: 1;
  }

  .queue-slot > :global(*) {
    flex: none;
  }
</style>

<script lang="ts">
  import House from "phosphor-svelte/lib/House";
  import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
  import Books from "phosphor-svelte/lib/Books";
  import Plus from "phosphor-svelte/lib/Plus";
  import Gear from "phosphor-svelte/lib/Gear";
  import MusicNotesPlus from "phosphor-svelte/lib/MusicNotesPlus";
  import Link from "phosphor-svelte/lib/Link";
  import PushPin from "phosphor-svelte/lib/PushPin";
  import Artwork from "./Artwork.svelte";
  import CollectionCover from "./CollectionCover.svelte";
  import ProviderBadge from "./ProviderBadge.svelte";
  import { errorMessage } from "$lib/api";
  import { collectionActions } from "$lib/collectionActions";
  import { format, i18n } from "$lib/i18n/index.svelte";
  import { sortable } from "$lib/sortable";
  import { library } from "$lib/state/library.svelte";
  import { menu } from "$lib/state/menu.svelte";
  import { nav, type Route } from "$lib/state/nav.svelte";
  import { toasts } from "$lib/state/toasts.svelte";
  import { ui } from "$lib/state/ui.svelte";

  interface Props {
    onimport: () => void;
  }

  let { onimport }: Props = $props();

  function isCurrent(route: Route): boolean {
    const current = nav.current;
    if (current.name !== route.name) return false;
    return current.name !== "collection" || current.id === (route as typeof current).id;
  }

  async function createPlaylist() {
    try {
      const playlist = await library.createPlaylist();
      nav.go({ name: "collection", id: playlist.id });
      ui.renaming = playlist.id;
    } catch (error) {
      toasts.error(format(i18n.t.errors.generic, { error: errorMessage(error) }));
    }
  }

  function openAddMenu(event: MouseEvent) {
    menu.showAt(event.currentTarget as HTMLElement, [
      { label: i18n.t.playlist.create, icon: MusicNotesPlus, action: () => void createPlaylist() },
      { label: i18n.t.importer.action, icon: Link, action: onimport },
    ]);
  }
</script>

<aside class="sidebar">
  <nav class="panel primary">
    <div class="wordmark">SCARPIFY</div>
    <button class="nav-item" class:current={isCurrent({ name: "home" })} onclick={() => nav.go({ name: "home" })}>
      <House weight={isCurrent({ name: "home" }) ? "fill" : "regular"} />
      {i18n.t.nav.home}
    </button>
    <button class="nav-item" class:current={isCurrent({ name: "search" })} onclick={() => nav.go({ name: "search" })}>
      <MagnifyingGlass weight={isCurrent({ name: "search" }) ? "bold" : "regular"} />
      {i18n.t.nav.search}
    </button>
  </nav>

  <section class="panel library">
    <header class="library-header">
      <h2><Books weight="fill" />{i18n.t.nav.library}</h2>
      <button class="icon-button" aria-label={i18n.t.playlist.create} title={i18n.t.playlist.create} onclick={openAddMenu}>
        <Plus weight="bold" />
      </button>
    </header>

    <div class="items">
      <button class="entry" class:current={isCurrent({ name: "liked" })} onclick={() => nav.go({ name: "liked" })}>
        <Artwork src={null} fallback="heart" size="48px" />
        <span class="entry-text">
          <span class="entry-title">{i18n.t.nav.liked}</span>
          <span class="entry-sub">{i18n.plural(i18n.t.plural.tracks, library.liked.length)}</span>
        </span>
      </button>

      <div class="collections" use:sortable={{ onmove: (from, to) => library.move(from, to) }}>
        {#each library.collections as collection (collection.id)}
          {@const route = { name: "collection", id: collection.id } as const}
          <button
            class="entry"
            class:current={isCurrent(route)}
            data-sortable
            onclick={() => nav.go(route)}
            oncontextmenu={(event) => menu.show(event, collectionActions(collection))}
          >
            <CollectionCover {collection} size="48px" />
            <span class="entry-text">
              <span class="entry-title">{collection.name}</span>
              <span class="entry-sub">
                {#if collection.pinned}<span class="pin"><PushPin weight="fill" /></span>{/if}
                {#if collection.provider}<ProviderBadge provider={collection.provider} variant="icon" />{/if}
                {i18n.plural(i18n.t.plural.tracks, collection.tracks.length)}
              </span>
            </span>
          </button>
        {/each}
      </div>
    </div>

    <button class="nav-item settings" class:current={isCurrent({ name: "settings" })} onclick={() => nav.go({ name: "settings" })}>
      <Gear weight={isCurrent({ name: "settings" }) ? "fill" : "regular"} />
      {i18n.t.nav.settings}
    </button>
  </section>
</aside>

<style>
  .sidebar {
    grid-area: sidebar;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }

  .panel {
    border-radius: var(--radius-panel);
    background: var(--surface);
  }

  .primary {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 16px 12px 12px;
  }

  .wordmark {
    padding: 0 12px 12px;
    font-size: 17px;
    font-weight: 800;
    letter-spacing: 0.14em;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 42px;
    padding: 0 12px;
    border-radius: var(--radius-card);
    color: var(--text-muted);
    font-size: 15px;
    font-weight: 600;
    text-align: left;
    transition: color var(--fast) var(--ease);
  }

  .nav-item :global(svg) {
    font-size: 22px;
  }

  .nav-item:hover,
  .nav-item.current {
    color: var(--text);
  }

  .library {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 8px;
  }

  .library-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 8px 8px 12px;
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 14px;
    color: var(--text-muted);
    font-size: 15px;
    font-weight: 600;
  }

  h2 :global(svg) {
    font-size: 22px;
  }

  .items {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 4px;
  }

  .collections {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .pin {
    display: grid;
    color: var(--accent);
    font-size: 13px;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border-radius: var(--radius-card);
    text-align: left;
    transition:
      background-color var(--fast) var(--ease),
      transform var(--base) var(--ease);
  }

  .entry:hover {
    background: var(--surface-hover);
  }

  .entry.current {
    background: var(--surface-active);
  }

  .entry-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .entry-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .current .entry-title {
    color: var(--accent);
  }

  .entry-sub {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .settings {
    margin-top: 8px;
  }
</style>

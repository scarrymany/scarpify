<script lang="ts">
  import { tick } from "svelte";
  import CaretRight from "phosphor-svelte/lib/CaretRight";
  import { menu, type MenuItem } from "$lib/state/menu.svelte";

  /** Gap between the menu and the window edge. */
  const EDGE_PX = 8;

  let panel = $state<HTMLDivElement>();
  let position = $state({ left: 0, top: 0 });
  let openSubmenu = $state<number | null>(null);
  let submenuLeft = $state(true);

  // Measure after render, then keep the whole menu inside the window.
  $effect(() => {
    const current = menu.current;
    openSubmenu = null;
    if (!current) return;
    position = { left: current.x, top: current.y };
    void tick().then(() => {
      if (!panel) return;
      const { width, height } = panel.getBoundingClientRect();
      position = {
        left: Math.max(EDGE_PX, Math.min(current.x, window.innerWidth - width - EDGE_PX)),
        top: Math.max(EDGE_PX, Math.min(current.y, window.innerHeight - height - EDGE_PX)),
      };
      submenuLeft = position.left + width * 2 + EDGE_PX > window.innerWidth;
    });
  });

  function run(item: MenuItem) {
    if (item.submenu) return;
    menu.close();
    item.action?.();
  }

  function onpointerdown(event: PointerEvent) {
    if (menu.current && panel && !panel.contains(event.target as Node)) menu.close();
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && menu.current) menu.close();
  }
</script>

<svelte:window
  onpointerdowncapture={onpointerdown}
  {onkeydown}
  onwheel={() => menu.close()}
  onresize={() => menu.close()}
  onblur={() => menu.close()}
/>

{#snippet list(items: MenuItem[], nested: boolean)}
  <div class="menu" class:nested class:left={nested && submenuLeft} role="menu">
    {#each items as item, index (item.label + index)}
      <div class="entry" onpointerenter={() => !nested && (openSubmenu = item.submenu ? index : null)} role="none">
        <button class="item" class:danger={item.danger} role="menuitem" onclick={() => run(item)}>
          {#if item.icon}<item.icon />{/if}
          <span class="label">{item.label}</span>
          {#if item.submenu}<span class="caret"><CaretRight /></span>{/if}
        </button>
        {#if item.submenu && openSubmenu === index}
          {@render list(item.submenu, true)}
        {/if}
      </div>
    {/each}
  </div>
{/snippet}

{#if menu.current}
  <div
    bind:this={panel}
    class="context"
    style:left="{position.left}px"
    style:top="{position.top}px"
    oncontextmenu={(event) => event.preventDefault()}
    role="presentation"
  >
    {@render list(menu.current.items, false)}
  </div>
{/if}

<style>
  .context {
    position: fixed;
    z-index: var(--layer-dialog);
    animation: appear var(--base) var(--ease);
    transform-origin: top left;
  }

  .menu {
    display: flex;
    flex-direction: column;
    min-width: 220px;
    max-width: 320px;
    padding: 4px;
    border-radius: var(--radius-card);
    background: var(--surface-raised);
    box-shadow:
      var(--shadow),
      inset 0 0 0 1px var(--line);
  }

  .menu.nested {
    position: absolute;
    top: -4px;
    left: calc(100% + 4px);
    max-height: 360px;
    overflow-y: auto;
    animation: appear var(--fast) var(--ease);
  }

  .menu.nested.left {
    right: calc(100% + 4px);
    left: auto;
  }

  .entry {
    position: relative;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    height: 38px;
    padding: 0 12px;
    border-radius: var(--radius-small);
    color: var(--text);
    font-size: 13.5px;
    text-align: left;
    transition: background-color var(--fast) var(--ease);
  }

  .item:hover,
  .entry:has(.nested) > .item {
    background: var(--surface-hover);
  }

  .item :global(svg) {
    flex: none;
    color: var(--text-muted);
    font-size: 17px;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .caret {
    display: grid;
  }

  .danger,
  .danger :global(svg) {
    color: var(--danger);
  }

  @keyframes appear {
    from {
      opacity: 0;
      transform: scale(0.97);
    }
  }
</style>

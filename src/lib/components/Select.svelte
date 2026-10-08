<script lang="ts" generics="T">
  import CaretDown from "phosphor-svelte/lib/CaretDown";
  import Check from "phosphor-svelte/lib/Check";

  interface Option {
    value: T;
    label: string;
  }

  interface Props {
    options: Option[];
    value: T;
    label: string;
    onchange: (value: T) => void;
    /** Called before the list opens, e.g. to refresh options. */
    onopen?: () => void;
  }

  let { options, value, label, onchange, onopen }: Props = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement>();

  const selected = $derived(options.find((option) => option.value === value) ?? options[0]);

  function toggle() {
    if (!open) onopen?.();
    open = !open;
  }

  function choose(option: Option) {
    open = false;
    if (option.value !== value) onchange(option.value);
  }

  function onpointerdown(event: PointerEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") open = false;
  }
</script>

<svelte:window onpointerdowncapture={onpointerdown} {onkeydown} />

<div class="select" bind:this={root}>
  <button class="trigger" class:open aria-haspopup="listbox" aria-expanded={open} aria-label={label} onclick={toggle}>
    <span class="current">{selected?.label}</span>
    <span class="caret"><CaretDown weight="bold" /></span>
  </button>

  {#if open}
    <div class="list" role="listbox" aria-label={label}>
      {#each options as option (option.value)}
        <button
          class="option"
          role="option"
          aria-selected={option.value === value}
          onclick={() => choose(option)}
        >
          <span class="option-label">{option.label}</span>
          {#if option.value === value}<Check weight="bold" />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .select {
    position: relative;
    min-width: 0;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 300px;
    max-width: 100%;
    height: 38px;
    padding: 0 12px 0 16px;
    border-radius: 999px;
    background: var(--surface-hover);
    font-weight: 500;
    transition: background-color var(--fast) var(--ease);
  }

  .trigger:hover,
  .trigger.open {
    background: var(--surface-active);
  }

  .current {
    flex: 1;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .caret {
    display: grid;
    color: var(--text-muted);
    transition: transform var(--base) var(--ease);
  }

  .open .caret {
    transform: rotate(180deg);
  }

  .list {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: var(--layer-dialog);
    display: flex;
    flex-direction: column;
    width: max(100%, 300px);
    max-height: 280px;
    padding: 4px;
    overflow-y: auto;
    border-radius: var(--radius-card);
    background: var(--surface-raised);
    box-shadow:
      var(--shadow),
      inset 0 0 0 1px var(--line);
    transform-origin: top right;
    animation: drop var(--base) var(--ease);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 38px;
    padding: 8px 12px;
    border-radius: var(--radius-small);
    text-align: left;
    transition: background-color var(--fast) var(--ease);
  }

  .option:hover {
    background: var(--surface-hover);
  }

  .option[aria-selected="true"] {
    color: var(--accent);
  }

  .option-label {
    flex: 1;
  }

  @keyframes drop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
</style>

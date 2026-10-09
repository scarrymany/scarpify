<script lang="ts">
  interface Props {
    value: number;
    max: number;
    label: string;
    /** Fired continuously while dragging; `commit` fires once on release. */
    oninput?: (value: number) => void;
    oncommit: (value: number) => void;
    step?: number;
    disabled?: boolean;
    /** Value advances on its own (playback): glide between updates instead of jumping. */
    flowing?: boolean;
  }

  let { value, max, label, oninput, oncommit, step = max / 100, disabled = false, flowing = false }: Props = $props();

  let track: HTMLDivElement;
  let dragValue = $state<number | null>(null);

  const shown = $derived(dragValue ?? value);
  // A non-finite ratio would make `scaleX` invalid and paint the whole rail as filled.
  const ratio = $derived(max > 0 && Number.isFinite(shown) ? Math.min(1, Math.max(0, shown / max)) : 0);

  function valueAt(clientX: number): number {
    const rect = track.getBoundingClientRect();
    const fraction = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    return fraction * max;
  }

  function onpointerdown(event: PointerEvent) {
    if (disabled || event.button !== 0) return;
    track.setPointerCapture(event.pointerId);
    dragValue = valueAt(event.clientX);
    oninput?.(dragValue);
  }

  function onpointermove(event: PointerEvent) {
    if (dragValue === null) return;
    dragValue = valueAt(event.clientX);
    oninput?.(dragValue);
  }

  function onpointerup() {
    if (dragValue === null) return;
    const final = dragValue;
    dragValue = null;
    oncommit(final);
  }

  function onkeydown(event: KeyboardEvent) {
    const delta = { ArrowRight: step, ArrowUp: step, ArrowLeft: -step, ArrowDown: -step }[event.key];
    if (delta === undefined || disabled) return;
    event.preventDefault();
    oncommit(Math.min(max, Math.max(0, value + delta)));
  }
</script>

<div
  bind:this={track}
  class="slider"
  class:dragging={dragValue !== null}
  class:flowing
  class:disabled
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={Math.round(max)}
  aria-valuenow={Math.round(shown)}
  aria-disabled={disabled}
  style:--ratio={ratio}
  {onpointerdown}
  {onpointermove}
  {onpointerup}
  onpointercancel={onpointerup}
  {onkeydown}
>
  <div class="rail">
    <div class="fill"></div>
  </div>
  <div class="thumb"></div>
</div>

<style>
  .slider {
    position: relative;
    flex: 1;
    height: 16px;
    display: flex;
    align-items: center;
    touch-action: none;
  }

  .slider.disabled {
    pointer-events: none;
    opacity: 0.5;
  }

  .rail {
    position: relative;
    width: 100%;
    height: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--text) 18%, transparent);
    overflow: hidden;
  }

  .fill {
    position: absolute;
    inset: 0;
    background: var(--accent);
    transform-origin: left;
    transform: scaleX(var(--ratio));
  }

  .thumb {
    position: absolute;
    left: calc(var(--ratio) * 100%);
    width: 12px;
    height: 12px;
    margin-left: -6px;
    border-radius: 999px;
    background: var(--text);
    box-shadow: 0 2px 6px rgb(0 0 0 / 0.35);
    opacity: 0;
    transform: scale(0.6);
    transition:
      opacity var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }

  /* Matches the engine's 250 ms position reports, so the bar never stops between them. */
  .flowing:not(.dragging) .fill {
    transition: transform 260ms linear;
  }

  .flowing:not(.dragging) .thumb {
    transition:
      left 260ms linear,
      opacity var(--fast) var(--ease),
      transform var(--fast) var(--ease);
  }

  .slider:hover .thumb,
  .slider:focus-visible .thumb,
  .dragging .thumb {
    opacity: 1;
    transform: scale(1);
  }
</style>

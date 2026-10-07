<script lang="ts">
  interface Props {
    playing: boolean;
  }

  let { playing }: Props = $props();

  // Each shape has the same number of points so the browser can interpolate `d`:
  // the triangle splits into two halves that slide apart into the pause bars.
  const SHAPES = {
    play: ["M7 4.5 L13 8.25 L13 15.75 L7 19.5 Z", "M13 8.25 L19 12 L19 12 L13 15.75 Z"],
    pause: ["M6 5 L10 5 L10 19 L6 19 Z", "M14 5 L18 5 L18 19 L14 19 Z"],
  };

  const shape = $derived(playing ? SHAPES.pause : SHAPES.play);
</script>

<svg viewBox="0 0 24 24" aria-hidden="true">
  {#each shape as d, i (i)}
    <path style:d={`path("${d}")`} />
  {/each}
</svg>

<style>
  svg {
    width: 1em;
    height: 1em;
    overflow: visible;
  }

  path {
    fill: currentColor;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linejoin: round;
    transition: d var(--base) var(--ease);
  }
</style>

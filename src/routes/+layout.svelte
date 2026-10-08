<script lang="ts">
  import "@fontsource-variable/onest";
  import "../app.css";
  import type { Snippet } from "svelte";
  import { settings } from "$lib/state/settings.svelte";

  let { children }: { children: Snippet } = $props();

  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  let systemDark = $state(darkQuery.matches);

  /** Palette currently on screen; `null` until the first paint, which must not animate. */
  let appliedPalette: string | null = null;

  $effect(() => {
    const onChange = (event: MediaQueryListEvent) => (systemDark = event.matches);
    darkQuery.addEventListener("change", onChange);
    return () => darkQuery.removeEventListener("change", onChange);
  });

  $effect(() => {
    const root = document.documentElement;
    root.dataset.motion = settings.animations ? "on" : "off";
    root.lang = settings.locale;
  });

  $effect(() => {
    const theme = settings.theme === "system" ? (systemDark ? "dark" : "light") : settings.theme;
    const accent = settings.accentColor;
    const palette = `${theme}|${accent}`;
    if (palette === appliedPalette) return;

    const animate = appliedPalette !== null && settings.animations && !reducedMotion.matches;
    appliedPalette = palette;
    switchPalette(theme, accent, animate);
  });

  /**
   * Swaps colors as one cross-faded frame. Per-element color transitions are paused meanwhile,
   * otherwise each control would fade on its own schedule and the switch would look torn.
   */
  function switchPalette(theme: string, accent: string, animate: boolean) {
    const root = document.documentElement;
    const apply = () => {
      root.dataset.theme = theme;
      root.style.setProperty("--accent", accent);
    };

    root.dataset.switching = "";
    const done = () => delete root.dataset.switching;

    if (animate && document.startViewTransition) {
      document.startViewTransition(apply).finished.finally(done);
      return;
    }
    apply();
    // Two frames: the first paints the new colors, the second re-enables transitions.
    requestAnimationFrame(() => requestAnimationFrame(done));
  }
</script>

{@render children()}

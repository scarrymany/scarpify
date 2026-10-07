<script lang="ts">
  import "@fontsource-variable/onest";
  import "../app.css";
  import type { Snippet } from "svelte";
  import { settings } from "$lib/state/settings.svelte";

  let { children }: { children: Snippet } = $props();

  const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
  let systemDark = $state(darkQuery.matches);

  $effect(() => {
    const onChange = (event: MediaQueryListEvent) => (systemDark = event.matches);
    darkQuery.addEventListener("change", onChange);
    return () => darkQuery.removeEventListener("change", onChange);
  });

  $effect(() => {
    const root = document.documentElement;
    const theme = settings.theme === "system" ? (systemDark ? "dark" : "light") : settings.theme;
    root.dataset.theme = theme;
    root.dataset.motion = settings.animations ? "on" : "off";
    root.lang = settings.locale;
    root.style.setProperty("--accent", settings.accentColor);
  });
</script>

{@render children()}

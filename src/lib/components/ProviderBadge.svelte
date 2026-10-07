<script lang="ts">
  import { siSoundcloud, siSpotify, siYoutubemusic } from "simple-icons";
  import { i18n } from "$lib/i18n/index.svelte";
  import type { Provider } from "$lib/types";

  interface Props {
    provider: Provider;
    /** `label` shows the logo with the service name, `icon` only the logo. */
    variant?: "label" | "icon";
  }

  let { provider, variant = "label" }: Props = $props();

  const LOGOS = {
    youtube: siYoutubemusic,
    soundcloud: siSoundcloud,
    spotify: siSpotify,
  } as const;

  const name = $derived(i18n.t.providers[provider]);
  const hint = $derived(provider === "spotify" ? `${name}. ${i18n.t.track.viaYoutube}` : name);
</script>

<span class="badge {variant}" style:--brand="var(--{provider})" title={hint}>
  <svg viewBox="0 0 24 24" aria-hidden="true"><path d={LOGOS[provider].path} /></svg>
  {#if variant === "label"}
    <span class="name">{name}</span>
  {:else}
    <span class="sr-only">{name}</span>
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    color: var(--text-faint);
    font-size: 11.5px;
    font-weight: 500;
    line-height: 1;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }

  svg {
    flex: none;
    width: 12px;
    height: 12px;
    fill: var(--brand);
  }

  .icon svg {
    width: 14px;
    height: 14px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>

<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
  import Check from "phosphor-svelte/lib/Check";
  import { glide } from "$lib/glide";
  import { i18n } from "$lib/i18n/index.svelte";
  import { ACCENTS, LOCALES, settings, type Theme } from "$lib/state/settings.svelte";

  const REPOSITORY_URL = "https://github.com/scarrymany/scarpify";

  let version = $state("");

  onMount(() => {
    void getVersion().then((v) => (version = v));
  });

  const themes = $derived<{ id: Theme; label: string }[]>([
    { id: "dark", label: i18n.t.settings.themeDark },
    { id: "light", label: i18n.t.settings.themeLight },
    { id: "system", label: i18n.t.settings.themeSystem },
  ]);

  function update(change: () => void) {
    change();
    settings.persist();
  }
</script>

<div class="settings">
  <h1>{i18n.t.settings.title}</h1>

  <section>
    <h2>{i18n.t.settings.language}</h2>
    <div class="segmented" role="radiogroup" aria-label={i18n.t.settings.language} use:glide>
      <span class="thumb" data-glide aria-hidden="true"></span>
      {#each LOCALES as locale (locale.id)}
        <button
          role="radio"
          aria-checked={settings.locale === locale.id}
          lang={locale.id}
          onclick={() => update(() => (settings.locale = locale.id))}
        >
          {locale.name}
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h2>{i18n.t.settings.appearance}</h2>

    <div class="field">
      <span class="label">{i18n.t.settings.theme}</span>
      <div class="segmented" role="radiogroup" aria-label={i18n.t.settings.theme} use:glide>
        <span class="thumb" data-glide aria-hidden="true"></span>
        {#each themes as theme (theme.id)}
          <button
            role="radio"
            aria-checked={settings.theme === theme.id}
            onclick={() => update(() => (settings.theme = theme.id))}
          >
            {theme.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="field">
      <span class="label">{i18n.t.settings.accent}</span>
      <div class="swatches" role="radiogroup" aria-label={i18n.t.settings.accent} use:glide>
        <span class="ring" data-glide aria-hidden="true"></span>
        {#each ACCENTS as accent (accent.id)}
          <button
            class="swatch"
            role="radio"
            aria-checked={settings.accent === accent.id}
            aria-label={accent.id}
            style:--swatch={accent.color}
            onclick={() => update(() => (settings.accent = accent.id))}
          >
            <span class="check"><Check weight="bold" /></span>
          </button>
        {/each}
      </div>
    </div>

    <div class="field">
      <span class="label-block">
        <span class="label">{i18n.t.settings.motion}</span>
        <span class="hint">{i18n.t.settings.motionHint}</span>
      </span>
      <button
        class="switch"
        role="switch"
        aria-checked={settings.animations}
        aria-label={i18n.t.settings.motion}
        onclick={() => update(() => (settings.animations = !settings.animations))}
      >
        <span class="knob"></span>
      </button>
    </div>

    <div class="field">
      <span class="label-block">
        <span class="label">{i18n.t.settings.discord}</span>
        <span class="hint">{i18n.t.settings.discordHint}</span>
      </span>
      <button
        class="switch"
        role="switch"
        aria-checked={settings.discordPresence}
        aria-label={i18n.t.settings.discord}
        onclick={() => update(() => (settings.discordPresence = !settings.discordPresence))}
      >
        <span class="knob"></span>
      </button>
    </div>
  </section>

  <section>
    <h2>{i18n.t.settings.about}</h2>
    <p class="about">
      <span class="product">SCARPIFY {version}</span>
      {i18n.t.settings.aboutBody}
    </p>
    <button class="pill-button secondary" onclick={() => openUrl(REPOSITORY_URL)}>
      {i18n.t.settings.sourceCode}<ArrowSquareOut />
    </button>
  </section>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 32px;
    max-width: 720px;
    padding: 24px 24px 48px;
  }

  h1 {
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  section {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 16px;
  }

  h2 {
    font-size: 16px;
    font-weight: 700;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    width: 100%;
  }

  .label {
    color: var(--text);
  }

  .label-block {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .hint {
    color: var(--text-muted);
    font-size: 13px;
  }

  .segmented {
    position: relative;
    display: inline-flex;
    padding: 3px;
    border-radius: 999px;
    background: var(--surface-hover);
  }

  /* One thumb slides under the options instead of each option toggling its own fill. */
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 0;
    width: var(--glide-w, 0);
    border-radius: 999px;
    background: var(--surface-raised);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
    transform: translateX(var(--glide-x, 0));
  }

  .segmented:global([data-glide-ready]) > .thumb {
    transition:
      transform var(--slow) var(--ease),
      width var(--slow) var(--ease);
  }

  .segmented button {
    position: relative;
    height: 32px;
    padding: 0 16px;
    border-radius: 999px;
    color: var(--text-muted);
    font-weight: 500;
    transition: color var(--base) var(--ease);
  }

  .segmented button:hover,
  .segmented button[aria-checked="true"] {
    color: var(--text);
  }

  .swatches {
    position: relative;
    display: flex;
    gap: 10px;
  }

  .ring {
    position: absolute;
    top: -4px;
    left: -4px;
    width: calc(var(--glide-w, 0px) + 8px);
    height: calc(100% + 8px);
    border: 2px solid var(--accent);
    border-radius: 999px;
    transform: translateX(var(--glide-x, 0));
    pointer-events: none;
  }

  .swatches:global([data-glide-ready]) > .ring {
    transition: transform var(--slow) var(--ease);
  }

  .swatch {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 999px;
    background: var(--swatch);
    color: #16110f;
    font-size: 15px;
    transition: transform var(--fast) var(--ease);
  }

  .swatch:active {
    transform: scale(0.92);
  }

  .check {
    display: grid;
    opacity: 0;
    transform: scale(0.6);
    transition:
      opacity var(--base) var(--ease),
      transform var(--base) var(--ease);
  }

  .swatch[aria-checked="true"] .check {
    opacity: 1;
    transform: scale(1);
  }

  .switch {
    position: relative;
    flex: none;
    width: 44px;
    height: 24px;
    border-radius: 999px;
    background: var(--surface-active);
    transition: background-color var(--base) var(--ease);
  }

  .switch[aria-checked="true"] {
    background: var(--accent);
  }

  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 999px;
    background: var(--text);
    transition: transform var(--base) var(--ease);
  }

  .switch[aria-checked="true"] .knob {
    background: var(--accent-contrast);
    transform: translateX(20px);
  }

  .about {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--text-muted);
    max-width: 60ch;
  }

  .product {
    color: var(--text);
    font-weight: 600;
  }
</style>

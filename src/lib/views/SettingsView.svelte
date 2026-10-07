<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import ArrowSquareOut from "phosphor-svelte/lib/ArrowSquareOut";
  import Check from "phosphor-svelte/lib/Check";
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
    <div class="segmented" role="radiogroup" aria-label={i18n.t.settings.language}>
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
      <div class="segmented" role="radiogroup" aria-label={i18n.t.settings.theme}>
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
      <div class="swatches" role="radiogroup" aria-label={i18n.t.settings.accent}>
        {#each ACCENTS as accent (accent.id)}
          <button
            class="swatch"
            role="radio"
            aria-checked={settings.accent === accent.id}
            aria-label={accent.id}
            style:--swatch={accent.color}
            onclick={() => update(() => (settings.accent = accent.id))}
          >
            {#if settings.accent === accent.id}<Check weight="bold" />{/if}
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
    display: inline-flex;
    padding: 3px;
    border-radius: 999px;
    background: var(--surface-hover);
  }

  .segmented button {
    height: 32px;
    padding: 0 16px;
    border-radius: 999px;
    color: var(--text-muted);
    font-weight: 500;
    transition:
      background-color var(--base) var(--ease),
      color var(--base) var(--ease);
  }

  .segmented button:hover {
    color: var(--text);
  }

  .segmented button[aria-checked="true"] {
    background: var(--surface-raised);
    color: var(--text);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
  }

  .swatches {
    display: flex;
    gap: 10px;
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

  .swatch[aria-checked="true"] {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--swatch);
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

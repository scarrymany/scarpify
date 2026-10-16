import { api, type TerminalPrefs } from "$lib/api";
import { player } from "$lib/state/player.svelte";
import { ACCENTS, LOCALES, settings, type AccentId, type Locale } from "$lib/state/settings.svelte";

const THEMES: TerminalPrefs["theme"][] = ["dark", "light", "system"];
/** Volume drags change settings many times a second; one write after they settle is enough. */
const SYNC_DELAY_MS = 500;

let syncTimer: ReturnType<typeof setTimeout> | undefined;

/** Reads every shared setting, so an `$effect` calling it re-runs when any of them changes. */
export function currentPrefs(): TerminalPrefs {
  return {
    locale: settings.locale,
    theme: settings.theme,
    accent: settings.accent,
    animations: settings.animations,
    volume: player.volume,
    muted: player.muted,
    audioDevice: settings.audioDevice,
    discordPresence: settings.discordPresence,
  };
}

/** Mirrors the window's settings for the terminal mode, which may start without the window. */
export function syncTerminalPrefs(prefs: TerminalPrefs): void {
  clearTimeout(syncTimer);
  syncTimer = setTimeout(() => void api.syncTerminalPrefs(prefs).catch(() => {}), SYNC_DELAY_MS);
}

/** Switches to the terminal mode; `false` means Windows Terminal has to be downloaded first. */
export function openTerminal(): Promise<boolean> {
  return api.openTerminalMode(currentPrefs());
}

/** Takes over what was changed in the terminal mode when it switched back to the window. */
export async function applyTerminalHandoff(): Promise<void> {
  const prefs = await api.terminalHandoff();
  if (!prefs) return;

  if (LOCALES.some((l) => l.id === prefs.locale)) settings.locale = prefs.locale as Locale;
  if (THEMES.includes(prefs.theme)) settings.theme = prefs.theme;
  if (ACCENTS.some((a) => a.id === prefs.accent)) settings.accent = prefs.accent as AccentId;
  settings.animations = prefs.animations;
  settings.discordPresence = prefs.discordPresence;
  settings.audioDevice = prefs.audioDevice;
  settings.persist();

  player.setVolume(prefs.volume);
  if (prefs.muted !== player.muted) player.toggleMute();
}

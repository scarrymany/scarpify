import { load, save } from "./storage";

export type Locale = "en" | "ru" | "uk";
export type Theme = "dark" | "light" | "system";

export const LOCALES: { id: Locale; name: string }[] = [
  { id: "uk", name: "Українська" },
  { id: "ru", name: "Русский" },
  { id: "en", name: "English" },
];

export const ACCENTS = [
  { id: "ember", color: "#ec6a45" },
  { id: "mint", color: "#3fbf8a" },
  { id: "azure", color: "#4f8cf0" },
  { id: "rose", color: "#e0577d" },
  { id: "amber", color: "#e3a33c" },
  { id: "iris", color: "#8b7ff0" },
] as const;

export type AccentId = (typeof ACCENTS)[number]["id"];

interface Persisted {
  locale: Locale;
  theme: Theme;
  accent: AccentId;
  animations: boolean;
  discordPresence: boolean;
  /** Output device id; `null` follows the system default. */
  audioDevice: string | null;
}

const KEY = "settings";

function detectLocale(): Locale {
  const language = navigator.language.toLowerCase();
  if (language.startsWith("uk")) return "uk";
  if (language.startsWith("ru")) return "ru";
  return "en";
}

const defaults: Persisted = {
  locale: detectLocale(),
  theme: "dark",
  accent: "ember",
  animations: true,
  discordPresence: true,
  audioDevice: null,
};

class Settings {
  locale = $state<Locale>(defaults.locale);
  theme = $state<Theme>(defaults.theme);
  accent = $state<AccentId>(defaults.accent);
  animations = $state(defaults.animations);
  discordPresence = $state(defaults.discordPresence);
  audioDevice = $state<string | null>(defaults.audioDevice);

  constructor() {
    const stored = { ...defaults, ...load<Partial<Persisted>>(KEY, {}) };
    this.locale = LOCALES.some((l) => l.id === stored.locale) ? stored.locale : defaults.locale;
    this.theme = stored.theme;
    this.accent = ACCENTS.some((a) => a.id === stored.accent) ? stored.accent : defaults.accent;
    this.animations = stored.animations;
    this.discordPresence = stored.discordPresence;
    this.audioDevice = stored.audioDevice;
  }

  get accentColor(): string {
    return ACCENTS.find((a) => a.id === this.accent)?.color ?? ACCENTS[0].color;
  }

  persist(): void {
    save<Persisted>(KEY, {
      locale: this.locale,
      theme: this.theme,
      accent: this.accent,
      animations: this.animations,
      discordPresence: this.discordPresence,
      audioDevice: this.audioDevice,
    });
  }
}

export const settings = new Settings();

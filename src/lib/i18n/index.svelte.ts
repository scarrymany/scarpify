import { settings, type Locale } from "$lib/state/settings.svelte";
import { en, type Dictionary } from "./en";
import { ru } from "./ru";
import { uk } from "./uk";

const dictionaries: Record<Locale, Dictionary> = { en, ru, uk };

type PluralForms = Dictionary["plural"]["tracks"];

class I18n {
  get t(): Dictionary {
    return dictionaries[settings.locale];
  }

  get locale(): Locale {
    return settings.locale;
  }

  plural(forms: PluralForms, count: number): string {
    const rule = new Intl.PluralRules(settings.locale).select(count);
    return format(forms[rule] ?? forms.other, { count });
  }
}

export const i18n = new I18n();

export function format(template: string, params: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    key in params ? String(params[key]) : match,
  );
}

export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return "-:--";
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`
    : `${minutes}:${seconds}`;
}

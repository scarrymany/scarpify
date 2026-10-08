import { afterEach, describe, expect, it } from "vitest";
import { settings } from "$lib/state/settings.svelte";
import { en } from "./en";
import { formatDuration, i18n } from "./index.svelte";
import { ru } from "./ru";
import { uk } from "./uk";

/** Every string path in a dictionary, e.g. `player.play`; plural forms differ per language. */
function keys(value: object, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix + key;
    if (typeof child !== "object" || path.startsWith("plural.")) return [path];
    return keys(child, `${path}.`);
  });
}

function placeholders(text: string): string[] {
  return [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}

function flatten(value: object, prefix = ""): Record<string, string> {
  return Object.fromEntries(
    Object.entries(value).flatMap(([key, child]) =>
      typeof child === "object" ? Object.entries(flatten(child, `${prefix}${key}.`)) : [[`${prefix}${key}`, child]],
    ),
  );
}

const original = settings.locale;
afterEach(() => {
  settings.locale = original;
});

describe("translations", () => {
  it.each([
    ["ru", ru],
    ["uk", uk],
  ])("%s has exactly the English keys", (_, dictionary) => {
    expect(new Set(keys(dictionary))).toEqual(new Set(keys(en)));
  });

  it.each([
    ["ru", ru],
    ["uk", uk],
  ])("%s keeps every placeholder", (_, dictionary) => {
    const english = flatten(en);
    for (const [key, text] of Object.entries(flatten(dictionary))) {
      if (key.startsWith("plural.")) continue;
      expect(placeholders(text), key).toEqual(placeholders(english[key]));
    }
  });

  it.each([
    ["ru", [[1, "1 трек"], [3, "3 трека"], [5, "5 треков"], [21, "21 трек"], [12, "12 треков"]]],
    ["uk", [[1, "1 трек"], [3, "3 треки"], [5, "5 треків"], [22, "22 треки"], [11, "11 треків"]]],
    ["en", [[1, "1 track"], [2, "2 tracks"], [0, "0 tracks"]]],
  ] as const)("pluralizes track counts in %s", (locale, cases) => {
    settings.locale = locale;
    for (const [count, expected] of cases) {
      expect(i18n.plural(i18n.t.plural.tracks, count)).toBe(expected);
    }
  });

  it("has no em or en dashes in any visible string", () => {
    for (const dictionary of [en, ru, uk]) {
      for (const [key, text] of Object.entries(flatten(dictionary))) {
        expect(text, key).not.toMatch(/[–—]/);
      }
    }
  });
});

describe("formatDuration", () => {
  it("formats minutes, hours and invalid input", () => {
    expect(formatDuration(65_000)).toBe("1:05");
    expect(formatDuration(3_725_000)).toBe("1:02:05");
    expect(formatDuration(null)).toBe("-:--");
    expect(formatDuration(Number.NaN)).toBe("-:--");
  });
});

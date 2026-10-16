import data from "./en.json";

// The dictionaries are JSON so the terminal mode (Rust) reads the very same strings.

type PluralForms = { other: string } & Partial<Record<Intl.LDMLPluralRule, string>>;

export type Dictionary = Omit<typeof data, "plural"> & { plural: { tracks: PluralForms } };

export const en: Dictionary = data;

import { api, errorMessage } from "$lib/api";
import type { ProviderResult } from "$lib/types";

export const SEARCH_PROVIDERS = ["youtube", "soundcloud"] as const;
export type SearchProvider = (typeof SEARCH_PROVIDERS)[number];

const DEBOUNCE_MS = 250;

const searchers: Record<SearchProvider, (query: string) => Promise<ProviderResult>> = {
  youtube: api.searchYoutube,
  soundcloud: api.searchSoundcloud,
};

/** Results for one provider; `null` until that provider has answered the current query. */
type Results = Record<SearchProvider, ProviderResult | null>;

const EMPTY: Results = { youtube: null, soundcloud: null };

class Search {
  query = $state("");
  results = $state<Results>({ ...EMPTY });
  /** Query the shown results belong to, so the UI never mixes old results with new input. */
  resultsFor = $state("");
  pending = $state<Record<SearchProvider, boolean>>({ youtube: false, soundcloud: false });

  loading = $derived(this.pending.youtube || this.pending.soundcloud);
  hasAny = $derived(SEARCH_PROVIDERS.some((p) => this.results[p] !== null));

  #timer: ReturnType<typeof setTimeout> | undefined;
  #requestId = 0;

  setQuery(query: string): void {
    this.query = query;
    clearTimeout(this.#timer);
    if (query.trim() === "") {
      this.#requestId++;
      this.results = { ...EMPTY };
      this.resultsFor = "";
      this.pending = { youtube: false, soundcloud: false };
      return;
    }
    this.#timer = setTimeout(() => this.run(), DEBOUNCE_MS);
  }

  /** Providers answer independently; whichever is faster shows up first. */
  run(): void {
    clearTimeout(this.#timer);
    const query = this.query.trim();
    if (query === "" || (query === this.resultsFor && !this.loading)) return;

    const requestId = ++this.#requestId;
    this.pending = { youtube: true, soundcloud: true };

    for (const provider of SEARCH_PROVIDERS) {
      searchers[provider](query)
        .catch((error): ProviderResult => ({ tracks: [], error: errorMessage(error) }))
        .then((result) => {
          if (requestId !== this.#requestId) return;
          if (this.resultsFor !== query) {
            this.results = { ...EMPTY };
            this.resultsFor = query;
          }
          this.results[provider] = result;
          this.pending[provider] = false;
        });
    }
  }
}

export const search = new Search();

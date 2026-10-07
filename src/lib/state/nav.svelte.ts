export type Route =
  | { name: "home" }
  | { name: "search" }
  | { name: "liked" }
  | { name: "collection"; id: string }
  | { name: "settings" };

function sameRoute(a: Route, b: Route): boolean {
  return a.name === b.name && (a.name !== "collection" || a.id === (b as typeof a).id);
}

/** Browser-like history so the back and forward buttons behave the way users expect. */
class Navigation {
  #stack = $state<Route[]>([{ name: "home" }]);
  #index = $state(0);

  current = $derived(this.#stack[this.#index]);
  canBack = $derived(this.#index > 0);
  canForward = $derived(this.#index < this.#stack.length - 1);

  go(route: Route): void {
    if (sameRoute(route, this.current)) return;
    this.#stack = [...this.#stack.slice(0, this.#index + 1), route];
    this.#index = this.#stack.length - 1;
  }

  back(): void {
    if (this.canBack) this.#index -= 1;
  }

  forward(): void {
    if (this.canForward) this.#index += 1;
  }

  /** Drops routes that point to something deleted, e.g. a removed playlist. */
  forget(predicate: (route: Route) => boolean): void {
    const current = this.current;
    const kept = this.#stack.filter((r) => !predicate(r));
    this.#stack = kept.length > 0 ? kept : [{ name: "home" }];
    const index = this.#stack.indexOf(current);
    this.#index = index >= 0 ? index : this.#stack.length - 1;
  }
}

export const nav = new Navigation();

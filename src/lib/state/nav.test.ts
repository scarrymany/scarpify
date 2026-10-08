import { beforeEach, describe, expect, it, vi } from "vitest";

type Nav = (typeof import("./nav.svelte"))["nav"];

let nav: Nav;

beforeEach(async () => {
  vi.resetModules();
  ({ nav } = await import("./nav.svelte"));
});

describe("navigation history", () => {
  it("moves back and forward like a browser", () => {
    nav.go({ name: "search" });
    nav.go({ name: "settings" });
    nav.back();
    expect(nav.current.name).toBe("search");
    nav.forward();
    expect(nav.current.name).toBe("settings");
    expect(nav.canForward).toBe(false);
  });

  it("drops forward history after a new navigation", () => {
    nav.go({ name: "search" });
    nav.back();
    nav.go({ name: "liked" });
    expect(nav.canForward).toBe(false);
  });

  it("ignores navigating to the current route", () => {
    nav.go({ name: "home" });
    expect(nav.canBack).toBe(false);
  });

  it("forgets a deleted playlist and lands on a valid route", () => {
    nav.go({ name: "collection", id: "a" });
    nav.go({ name: "collection", id: "b" });
    nav.forget((r) => r.name === "collection" && r.id === "b");
    expect(nav.current).toEqual({ name: "collection", id: "a" });
    nav.back();
    expect(nav.current.name).toBe("home");
  });
});

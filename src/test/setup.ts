import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// jsdom has no media queries; tests run as a dark-theme desktop without reduced motion.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: query.includes("dark"),
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
  }),
});

// jsdom lacks the Web Animations API that Svelte transitions run on; finish them at once.
Element.prototype.animate = function () {
  const animation = {
    onfinish: null as (() => void) | null,
    currentTime: 0,
    finished: Promise.resolve(),
    cancel() {},
    play() {},
    pause() {},
  };
  queueMicrotask(() => animation.onfinish?.());
  return animation as unknown as Animation;
};

vi.mock("@tauri-apps/api/core", async () => ({ invoke: (await import("./tauri")).invoke }));
vi.mock("@tauri-apps/api/event", async () => ({ listen: (await import("./tauri")).listen }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn(async () => "0.0.0-test") }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    isMaximized: async () => false,
    onResized: async () => () => {},
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
  }),
}));

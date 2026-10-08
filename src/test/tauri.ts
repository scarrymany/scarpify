import { vi, type Mock } from "vitest";

interface Bridge {
  invoke: Mock<(command: string, args?: unknown) => Promise<unknown>>;
  listen: Mock<(event: string, handler: (event: { payload: unknown }) => void) => Promise<() => void>>;
}

const globals = globalThis as typeof globalThis & { __tauriBridge?: Bridge };

/**
 * Tauri bridge doubles. Stored on `globalThis` so they survive `vi.resetModules()`:
 * tests that reload singletons still talk to the same spies.
 */
export const bridge: Bridge = (globals.__tauriBridge ??= {
  invoke: vi.fn(async () => undefined),
  listen: vi.fn(async () => () => {}),
});

export const { invoke, listen } = bridge;

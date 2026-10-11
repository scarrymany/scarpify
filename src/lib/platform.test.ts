import { describe, expect, it, vi } from "vitest";

describe("native platform capabilities", () => {
  it("does not confuse a narrow Linux window with Android", async () => {
    vi.stubEnv("VITE_NATIVE_PLATFORM", "linux");
    vi.resetModules();
    const platform = await import("./platform");
    expect(platform.supportsDesktopWindow).toBe(true);
    expect(platform.supportsDiscordPresence).toBe(true);
    vi.unstubAllEnvs();
  });
  it("does not invoke unsupported desktop integrations on Android", async () => {
    vi.stubEnv("VITE_NATIVE_PLATFORM", "android");
    vi.resetModules();
    const platform = await import("./platform");
    expect(platform.isAndroid).toBe(true);
    expect(platform.supportsDesktopWindow).toBe(false);
    expect(platform.supportsDiscordPresence).toBe(false);
    vi.unstubAllEnvs();
  });
});

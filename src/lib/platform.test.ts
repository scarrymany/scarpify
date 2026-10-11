import { describe, expect, it } from "vitest";
import { capabilities } from "./platform";

describe("platform UI capabilities", () => {
  it("does not expose desktop commands on Android", () => {
    expect(capabilities("Mozilla/5.0 (Linux; Android 15) wv")).toEqual({ android: true, windowControls: false, discordPresence: false, outputSelection: false });
  });
  it.each(["Windows NT 10.0", "X11; Linux x86_64"])("preserves desktop capabilities on %s", ua => {
    expect(capabilities(ua)).toEqual({ android: false, windowControls: true, discordPresence: true, outputSelection: true });
  });
});

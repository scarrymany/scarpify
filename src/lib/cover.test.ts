import { afterEach, describe, expect, it, vi } from "vitest";
import { pickCover } from "./cover";

afterEach(() => { vi.restoreAllMocks(); document.body.innerHTML = ""; });

describe("native cover chooser lifecycle", () => {
  it("keeps the chooser input attached until WebKit reports cancellation", async () => {
    let chooser: HTMLInputElement | undefined;
    vi.spyOn(HTMLInputElement.prototype, "click").mockImplementation(function () {
      chooser = this;
      expect(this.isConnected).toBe(true);
    });
    const result = pickCover();
    expect(chooser?.isConnected).toBe(true);
    chooser!.dispatchEvent(new Event("cancel"));
    expect(await result).toBeNull();
    expect(chooser?.isConnected).toBe(false);
  });

  it("cleans up when the chooser returns no file", async () => {
    vi.spyOn(HTMLInputElement.prototype, "click").mockImplementation(function () {
      this.dispatchEvent(new Event("change"));
    });
    expect(await pickCover()).toBeNull();
    expect(document.querySelector('input[type="file"]')).toBeNull();
  });

  it("cleans up after chooser failure", async () => {
    vi.spyOn(HTMLInputElement.prototype, "click").mockImplementation(() => { throw new Error("chooser unavailable"); });
    await expect(pickCover()).rejects.toThrow("chooser unavailable");
    expect(document.querySelector('input[type="file"]')).toBeNull();
  });
});

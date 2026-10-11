/** Native target is supplied by the Tauri CLI, independent of window size or user agent. */
export const platform = import.meta.env.VITE_NATIVE_PLATFORM ?? "browser";
export const isAndroid = platform === "android";
export const supportsDesktopWindow = !isAndroid;
export const supportsDiscordPresence = !isAndroid;

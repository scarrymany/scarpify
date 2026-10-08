// Tauri serves a static SPA, there is no server to render on.
export const ssr = false;
export const prerender = true;

export async function load() {
  // `npm run dev` opened in a regular browser has no Tauri bridge; fake it for UI work.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    await import("$lib/dev/browser-preview");
  }
}

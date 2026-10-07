const PREFIX = "scarpify:";

/** Storage can be unavailable or hold data from an older version; both fall back to defaults. */
export function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

export function save<T>(key: string, value: T): void {
  try {
    localStorage.setItem(PREFIX + key, JSON.stringify(value));
  } catch {
    // Quota or privacy errors only cost persistence, never the running session.
  }
}

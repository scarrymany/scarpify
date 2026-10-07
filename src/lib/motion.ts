import { cubicOut } from "svelte/easing";
import { settings } from "$lib/state/settings.svelte";

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

/** Transition params that collapse to instant when animations are off or reduced. */
export function motion<T extends object>(duration: number, params?: T): T & { duration: number; easing: typeof cubicOut } {
  const enabled = settings.animations && !reducedMotion.matches;
  return { easing: cubicOut, ...(params as T), duration: enabled ? duration : 0 };
}

import type { Component } from "svelte";

export interface MenuItem {
  label: string;
  icon?: Component;
  danger?: boolean;
  action?: () => void;
  submenu?: MenuItem[];
}

interface OpenMenu {
  x: number;
  y: number;
  items: MenuItem[];
}

/** One context menu for the whole app; opening another replaces it. */
class ContextMenu {
  current = $state<OpenMenu | null>(null);

  show(event: MouseEvent, items: MenuItem[]): void {
    event.preventDefault();
    event.stopPropagation();
    this.current = { x: event.clientX, y: event.clientY, items };
  }

  /** Opens below an anchor element, e.g. a "more" button. */
  showAt(anchor: HTMLElement, items: MenuItem[]): void {
    const rect = anchor.getBoundingClientRect();
    this.current = { x: rect.left, y: rect.bottom + 6, items };
  }

  close(): void {
    this.current = null;
  }
}

export const menu = new ContextMenu();

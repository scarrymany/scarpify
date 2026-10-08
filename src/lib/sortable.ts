/**
 * Drag-to-reorder for vertical lists, used by the sidebar, the queue and user playlists.
 *
 * Items are the container's descendants marked `data-sortable` (in list order). A press
 * only becomes a drag after a few pixels of movement, so clicks and double-clicks keep
 * working. The dragged item follows the pointer while the others slide aside with CSS
 * transforms; the DOM is reordered once, by `onmove`, when the item is dropped.
 */

export interface SortableOptions {
  onmove: (from: number, to: number) => void;
  disabled?: boolean;
}

const DRAG_THRESHOLD_PX = 5;
const AUTOSCROLL_EDGE_PX = 48;
const AUTOSCROLL_MAX_STEP_PX = 14;
/** Targets that keep their own pointer behaviour instead of starting a drag. */
const NO_DRAG = "input, textarea, [role=slider], [contenteditable]";

interface Drag {
  item: HTMLElement;
  items: HTMLElement[];
  rects: DOMRect[];
  from: number;
  to: number;
  startY: number;
  pointerY: number;
  scroller: HTMLElement | null;
  startScroll: number;
  /** Distance between neighbouring item tops: item height plus any gap. */
  slot: number;
  started: boolean;
}

function scrollParent(element: HTMLElement): HTMLElement | null {
  for (let node = element.parentElement; node; node = node.parentElement) {
    const overflow = getComputedStyle(node).overflowY;
    if (overflow === "auto" || overflow === "scroll") return node;
  }
  return null;
}

export function sortable(container: HTMLElement, initial: SortableOptions) {
  let options = initial;
  let drag: Drag | null = null;
  let frame = 0;

  const itemsOf = () => [...container.querySelectorAll<HTMLElement>("[data-sortable]")];

  function onpointerdown(event: PointerEvent) {
    if (options.disabled || event.button !== 0) return;
    const target = event.target as Element;
    if (target.closest(NO_DRAG)) return;
    const item = target.closest<HTMLElement>("[data-sortable]");
    if (!item || !container.contains(item)) return;

    const items = itemsOf();
    const from = items.indexOf(item);
    const scroller = scrollParent(container);
    drag = {
      item,
      items,
      rects: [],
      from,
      to: from,
      startY: event.clientY,
      pointerY: event.clientY,
      scroller,
      startScroll: scroller?.scrollTop ?? 0,
      slot: 0,
      started: false,
    };
    window.addEventListener("pointermove", onpointermove);
    window.addEventListener("pointerup", onpointerup, { once: true });
    window.addEventListener("pointercancel", onpointerup, { once: true });
  }

  function begin(state: Drag) {
    state.started = true;
    state.rects = state.items.map((item) => item.getBoundingClientRect());
    const next = state.rects[state.from + 1] ?? state.rects[state.from - 1];
    const own = state.rects[state.from];
    state.slot = next ? Math.abs(next.top - own.top) : own.height;
    state.item.classList.add("dragging");
    container.classList.add("sorting");
    frame = requestAnimationFrame(autoscroll);
  }

  function onpointermove(event: PointerEvent) {
    if (!drag) return;
    drag.pointerY = event.clientY;
    if (!drag.started) {
      if (Math.abs(event.clientY - drag.startY) < DRAG_THRESHOLD_PX) return;
      begin(drag);
    }
    layout(drag);
  }

  /** Positions the dragged item under the pointer and slides the others into place. */
  function layout(state: Drag) {
    const scrolled = (state.scroller?.scrollTop ?? 0) - state.startScroll;
    const dy = state.pointerY - state.startY + scrolled;
    const own = state.rects[state.from];
    const center = own.top + own.height / 2 + dy - scrolled;

    let to = state.from;
    state.rects.forEach((rect, index) => {
      const mid = rect.top - scrolled + rect.height / 2;
      if (index < state.from && center < mid) to = Math.min(to, index);
      if (index > state.from && center > mid) to = Math.max(to, index);
    });
    state.to = to;

    state.items.forEach((item, index) => {
      if (index === state.from) {
        item.style.transform = `translateY(${dy}px)`;
        return;
      }
      let shift = 0;
      if (index >= to && index < state.from) shift = state.slot;
      if (index <= to && index > state.from) shift = -state.slot;
      item.style.transform = shift ? `translateY(${shift}px)` : "";
    });
  }

  /** Scrolls the list while the pointer rests near its top or bottom edge. */
  function autoscroll() {
    if (!drag?.started) return;
    const scroller = drag.scroller;
    if (scroller) {
      const bounds = scroller.getBoundingClientRect();
      const up = bounds.top + AUTOSCROLL_EDGE_PX - drag.pointerY;
      const down = drag.pointerY - (bounds.bottom - AUTOSCROLL_EDGE_PX);
      const step = up > 0 ? -up : down > 0 ? down : 0;
      if (step !== 0) {
        scroller.scrollTop += Math.max(-1, Math.min(1, step / AUTOSCROLL_EDGE_PX)) * AUTOSCROLL_MAX_STEP_PX;
        layout(drag);
      }
    }
    frame = requestAnimationFrame(autoscroll);
  }

  function onpointerup() {
    window.removeEventListener("pointermove", onpointermove);
    window.removeEventListener("pointerup", onpointerup);
    window.removeEventListener("pointercancel", onpointerup);
    cancelAnimationFrame(frame);
    const state = drag;
    drag = null;
    if (!state?.started) return;

    // The release would also click whatever is under the pointer.
    window.addEventListener("click", swallowClick, { capture: true, once: true });
    setTimeout(() => window.removeEventListener("click", swallowClick, { capture: true }), 0);

    // Drop without animating back: the reorder below puts every item in its final place.
    container.classList.add("sort-dropping");
    for (const item of state.items) item.style.transform = "";
    state.item.classList.remove("dragging");
    container.classList.remove("sorting");
    if (state.to !== state.from) options.onmove(state.from, state.to);
    requestAnimationFrame(() => requestAnimationFrame(() => container.classList.remove("sort-dropping")));
  }

  function swallowClick(event: MouseEvent) {
    event.stopPropagation();
    event.preventDefault();
  }

  container.addEventListener("pointerdown", onpointerdown);

  return {
    update(next: SortableOptions) {
      options = next;
    },
    destroy() {
      container.removeEventListener("pointerdown", onpointerdown);
      window.removeEventListener("pointermove", onpointermove);
      window.removeEventListener("pointerup", onpointerup);
      cancelAnimationFrame(frame);
    },
  };
}

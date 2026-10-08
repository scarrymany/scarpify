/**
 * Svelte action for selection indicators that slide between options.
 *
 * It tracks the descendant marked `aria-checked="true"` and exposes its box to CSS as
 * `--glide-x` and `--glide-w` on the container, so a single indicator element can move with
 * `transform` instead of each option toggling its own background. Positions follow option
 * size changes too, e.g. labels changing width when the interface language switches.
 */
export function glide(container: HTMLElement) {
  let ready = false;

  const update = () => {
    const active = container.querySelector<HTMLElement>('[aria-checked="true"]');
    if (!active) return;
    container.style.setProperty("--glide-x", `${active.offsetLeft}px`);
    container.style.setProperty("--glide-w", `${active.offsetWidth}px`);

    // The first placement must not animate in from the container's corner.
    if (!ready) {
      ready = true;
      requestAnimationFrame(() => (container.dataset.glideReady = ""));
    }
  };

  const mutations = new MutationObserver(update);
  mutations.observe(container, { subtree: true, attributes: true, attributeFilter: ["aria-checked"] });

  const resizes = new ResizeObserver(update);
  resizes.observe(container);
  for (const child of container.children) resizes.observe(child);

  update();

  return {
    destroy() {
      mutations.disconnect();
      resizes.disconnect();
    },
  };
}

<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    children: Snippet;
  }

  let { title, children }: Props = $props();
</script>

<section class="shelf">
  <h2>{title}</h2>
  <div class="row">
    {@render children()}
  </div>
</section>

<style>
  .shelf {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h2 {
    padding: 0 12px;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  /* One responsive row: extra cards collapse to zero height instead of wrapping. */
  .row {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
    grid-template-rows: 1fr;
    grid-auto-rows: 0;
    overflow: hidden;
  }
</style>

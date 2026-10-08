<script lang="ts">
  import Heart from "phosphor-svelte/lib/Heart";
  import { i18n } from "$lib/i18n/index.svelte";
  import { library } from "$lib/state/library.svelte";
  import type { Track } from "$lib/types";

  interface Props {
    track: Track;
  }

  let { track }: Props = $props();

  const liked = $derived(library.isLiked(track));
  const label = $derived(liked ? i18n.t.track.unlike : i18n.t.track.like);

  /** Set only by a click, so hearts that are already liked do not pop when a list renders. */
  let bounce = $state<"like" | "unlike" | null>(null);

  function toggle() {
    bounce = liked ? "unlike" : "like";
    library.toggleLike(track);
  }
</script>

<button
  class="like"
  class:liked
  aria-pressed={liked}
  aria-label={label}
  title={label}
  onclick={toggle}
>
  <span class="heart" data-bounce={bounce} onanimationend={() => (bounce = null)}>
    <span class="outline"><Heart /></span>
    <span class="fill"><Heart weight="fill" /></span>
  </span>
</button>

<style>
  .like {
    display: inline-grid;
    place-items: center;
    width: 32px;
    height: 32px;
    color: var(--text-muted);
    font-size: 18px;
    transition: color var(--base) var(--ease);
  }

  .like:hover {
    color: var(--text);
  }

  .like.liked {
    color: var(--accent);
  }

  .heart {
    position: relative;
    display: grid;
  }

  .outline,
  .fill {
    display: grid;
    grid-area: 1 / 1;
  }

  /* The fill grows out of the heart's centre with a slight overshoot. */
  .fill {
    transform: scale(0);
    opacity: 0;
    transition:
      transform var(--slow) cubic-bezier(0.34, 1.56, 0.64, 1),
      opacity var(--fast) var(--ease);
  }

  .liked .fill {
    transform: scale(1);
    opacity: 1;
  }

  .heart[data-bounce="like"] {
    animation: like-bounce var(--slow) var(--ease);
  }

  .heart[data-bounce="unlike"] {
    animation: unlike-bounce var(--base) var(--ease);
  }

  @keyframes like-bounce {
    0% {
      transform: scale(1);
    }
    30% {
      transform: scale(0.82);
    }
    65% {
      transform: scale(1.18);
    }
    100% {
      transform: scale(1);
    }
  }

  @keyframes unlike-bounce {
    0% {
      transform: scale(1);
    }
    45% {
      transform: scale(0.86);
    }
    100% {
      transform: scale(1);
    }
  }
</style>

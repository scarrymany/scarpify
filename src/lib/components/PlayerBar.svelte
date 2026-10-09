<script lang="ts">
  import SkipBack from "phosphor-svelte/lib/SkipBack";
  import SkipForward from "phosphor-svelte/lib/SkipForward";
  import Shuffle from "phosphor-svelte/lib/Shuffle";
  import Repeat from "phosphor-svelte/lib/Repeat";
  import RepeatOnce from "phosphor-svelte/lib/RepeatOnce";
  import Queue from "phosphor-svelte/lib/Queue";
  import SpeakerHigh from "phosphor-svelte/lib/SpeakerHigh";
  import SpeakerLow from "phosphor-svelte/lib/SpeakerLow";
  import SpeakerSlash from "phosphor-svelte/lib/SpeakerSlash";
  import Artwork from "./Artwork.svelte";
  import LikeButton from "./LikeButton.svelte";
  import PlayPauseIcon from "./PlayPauseIcon.svelte";
  import ProviderBadge from "./ProviderBadge.svelte";
  import Slider from "./Slider.svelte";
  import { formatDuration, i18n } from "$lib/i18n/index.svelte";
  import { player } from "$lib/state/player.svelte";

  interface Props {
    queueOpen: boolean;
    ontogglequeue: () => void;
  }

  let { queueOpen, ontogglequeue }: Props = $props();

  /** While scrubbing, the time label follows the thumb instead of playback. */
  let scrubMs = $state<number | null>(null);

  const track = $derived(player.current);
  const shownPosition = $derived(scrubMs ?? player.positionMs);
  const silent = $derived(player.muted || player.volume === 0);
  const repeatLabel = $derived(player.repeat === "one" ? i18n.t.player.repeatOne : i18n.t.player.repeat);
</script>

<footer class="playerbar">
  <div class="now">
    {#if track}
      <Artwork src={track.artwork} size="56px" />
      <div class="meta">
        <span class="title" title={track.title}>{track.title}</span>
        <span class="artists">{track.artists.join(", ")}</span>
        <ProviderBadge provider={track.provider} />
      </div>
      <LikeButton {track} />
    {:else}
      <span class="idle">{i18n.t.player.nothing}</span>
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      <button
        class="icon-button"
        aria-pressed={player.shuffle}
        aria-label={i18n.t.player.shuffle}
        title={i18n.t.player.shuffle}
        onclick={() => player.toggleShuffle()}
      >
        <Shuffle />
      </button>
      <button class="icon-button" aria-label={i18n.t.player.previous} title={i18n.t.player.previous} disabled={!track} onclick={() => player.previous()}>
        <SkipBack weight="fill" />
      </button>
      <button
        class="main-button"
        aria-label={player.status === "playing" ? i18n.t.player.pause : i18n.t.player.play}
        title={player.status === "loading" ? i18n.t.player.loading : undefined}
        disabled={!track}
        onclick={() => player.toggle()}
      >
        <!-- Loading keeps the triangle, so skipping never flashes the pause bars. -->
        <PlayPauseIcon playing={player.status === "playing"} />
      </button>
      <button class="icon-button" aria-label={i18n.t.player.next} title={i18n.t.player.next} disabled={!track} onclick={() => player.next()}>
        <SkipForward weight="fill" />
      </button>
      <button
        class="icon-button"
        aria-pressed={player.repeat !== "off"}
        aria-label={repeatLabel}
        title={repeatLabel}
        onclick={() => player.cycleRepeat()}
      >
        {#if player.repeat === "one"}<RepeatOnce />{:else}<Repeat />{/if}
      </button>
    </div>

    <div class="progress">
      <span class="time">{formatDuration(track ? shownPosition : null)}</span>
      <Slider
        value={player.positionMs}
        max={player.durationMs}
        step={5000}
        label={i18n.t.track.duration}
        disabled={!track || player.durationMs === 0 || player.status === "loading"}
        flowing={player.status === "playing"}
        oninput={(v) => (scrubMs = v)}
        oncommit={(v) => {
          scrubMs = null;
          player.seek(v);
        }}
      />
      <span class="time">{formatDuration(track?.durationMs)}</span>
    </div>
  </div>

  <div class="extras">
    <button
      class="icon-button"
      aria-pressed={queueOpen}
      aria-label={i18n.t.player.queue}
      title={i18n.t.player.queue}
      onclick={ontogglequeue}
    >
      <Queue />
    </button>
    <button
      class="icon-button"
      aria-label={silent ? i18n.t.player.unmute : i18n.t.player.mute}
      title={silent ? i18n.t.player.unmute : i18n.t.player.mute}
      onclick={() => player.toggleMute()}
    >
      {#if silent}<SpeakerSlash />{:else if player.volume < 0.5}<SpeakerLow />{:else}<SpeakerHigh />{/if}
    </button>
    <div class="volume">
      <Slider
        value={player.muted ? 0 : player.volume}
        max={1}
        step={0.05}
        label={i18n.t.player.volume}
        oninput={(v) => player.setVolume(v)}
        oncommit={(v) => player.setVolume(v)}
      />
    </div>
  </div>
</footer>

<style>
  .playerbar {
    grid-area: player;
    display: grid;
    grid-template-columns: minmax(180px, 1fr) minmax(320px, 680px) minmax(180px, 1fr);
    align-items: center;
    gap: 16px;
    height: var(--playerbar-height);
    padding: 0 8px;
  }

  .now {
    display: flex;
    align-items: center;
    gap: 14px;
    min-width: 0;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .title,
  .artists {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-weight: 600;
  }

  .artists {
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .idle {
    color: var(--text-faint);
    font-size: 13px;
  }

  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .main-button {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 999px;
    background: var(--text);
    color: var(--bg);
    font-size: 16px;
    transition:
      transform var(--fast) var(--ease),
      background-color var(--fast) var(--ease);
  }

  .main-button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--text) 86%, var(--bg));
  }

  .main-button:active:not(:disabled) {
    transform: scale(0.94);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
  }

  .time {
    min-width: 40px;
    color: var(--text-muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .extras {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
  }

  .volume {
    display: flex;
    width: 112px;
  }
</style>

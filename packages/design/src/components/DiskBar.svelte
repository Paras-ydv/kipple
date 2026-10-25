<script lang="ts">
  import { formatBytes, formatDisk } from '../format.ts';
  import { kippleSwatch } from '../tokens.ts';
  import Dot from './Dot.svelte';

  interface Props {
    volume: string;
    capacity: number;
    free: number;
    /** kipple bytes per category, largest first. */
    parts: { key: string; color: string; bytes: number }[];
  }

  let { volume, capacity, free, parts }: Props = $props();

  const kipple = $derived(parts.reduce((s, p) => s + p.bytes, 0));
  const yours = $derived(Math.max(0, capacity - free - kipple));
  const pct = (b: number) => `${((b / capacity) * 100).toFixed(2)}%`;
</script>

<div class="disk k-glass">
  <div class="head">
    <span class="name">{volume}</span>
    <span class="free">{formatDisk(free)} free of {formatDisk(capacity)}</span>
  </div>
  <div
    class="bar"
    role="img"
    aria-label="{formatDisk(yours)} of your files, {formatBytes(kipple)} of kipple, {formatDisk(free)} free"
  >
    <span style:width={pct(yours)} style:background="rgba(255,255,255,.3)"></span>
    {#each parts as p (p.key)}<span style:width={pct(p.bytes)} style:background={p.color}></span>{/each}
  </div>
  <div class="legend">
    <span><Dot color="rgba(255,255,255,.4)" />Your files {formatDisk(yours)}</span>
    <span><Dot color="" background={kippleSwatch} />kipple {formatBytes(kipple)}</span>
    <span><Dot color="rgba(255,255,255,.4)" outline />Free {formatDisk(free)}</span>
  </div>
</div>

<style>
  .disk {
    border-radius: var(--k-r-panel);
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }
  .name {
    font-weight: 600;
  }
  .free {
    color: var(--k-text-3);
  }
  .bar {
    height: 14px;
    border-radius: var(--k-r-bar);
    background: var(--k-track);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.35);
    display: flex;
    overflow: hidden;
    gap: 1px;
  }
  .bar span {
    display: block;
    flex: none;
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    font-size: 12px;
    color: var(--k-text-3);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>

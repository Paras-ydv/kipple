<script lang="ts">
  // A figure with the dark stage background that stacks float on (docs/UI.md §3.1).
  import type { Snippet } from 'svelte';

  interface Props {
    /** SVG coordinate space. */
    width: number;
    height: number;
    radius?: number;
    /** Accessible description of the picture. */
    label: string;
    /** Floor line (y in SVG units) for the subtle reflection plane. */
    floorY?: number;
    shadow?: 'lg' | 'sm' | 'none';
    /** SVG content (stacks). */
    children: Snippet;
    /** HTML overlay positioned in the same coordinate space (chips, labels). */
    overlay?: Snippet;
    class?: string;
    /** Fill the parent instead of using a fixed size. */
    fluid?: boolean;
  }

  let {
    width,
    height,
    radius = 22,
    label,
    floorY,
    shadow = 'lg',
    children,
    overlay,
    class: className = '',
    fluid = false,
  }: Props = $props();
</script>

<figure
  class="stage {className} shadow-{shadow}"
  class:fluid
  style:border-radius="{radius}px"
  style:aspect-ratio="{width} / {height}"
  style:width={fluid ? '100%' : `${width}px`}
  style:--stage-floor={floorY !== undefined ? `${(floorY / height) * 100}%` : '78%'}
>
  <svg viewBox="0 0 {width} {height}" role="img" aria-label={label} preserveAspectRatio="xMidYMid meet">
    {@render children()}
  </svg>
  {#if overlay}
    <div class="overlay" style:--sx="{100 / width}%" style:--sy="{100 / height}%">
      {@render overlay()}
    </div>
  {/if}
  <div class="rim" aria-hidden="true" style:border-radius="{radius}px"></div>
</figure>

<style>
  .stage {
    position: relative;
    margin: 0;
    overflow: hidden;
    flex: none;
    container-type: inline-size;
    background:
      radial-gradient(60% 42% at 50% 36%, rgba(70, 110, 255, 0.16), rgba(70, 110, 255, 0) 70%),
      radial-gradient(70% 28% at 50% var(--stage-floor), rgba(255, 132, 84, 0.1), rgba(255, 132, 84, 0) 70%),
      linear-gradient(180deg, #0b0e1c 0%, #06080f 58%, #05060b 100%);
  }
  .stage::before {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    top: var(--stage-floor);
    bottom: 0;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0) 60%);
    border-top: 1px solid rgba(255, 255, 255, 0.04);
    pointer-events: none;
  }
  .fluid {
    flex: 1 1 auto;
  }
  .shadow-lg {
    box-shadow: var(--k-shadow-figure);
  }
  .shadow-sm {
    box-shadow: var(--k-shadow-figure-sm);
  }
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    overflow: visible;
  }
  .overlay {
    position: absolute;
    inset: 0;
  }
  .rim {
    position: absolute;
    inset: 0;
    pointer-events: none;
    box-shadow: var(--k-rim-figure);
  }
</style>

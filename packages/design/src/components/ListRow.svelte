<script lang="ts">
  import type { Snippet } from 'svelte';
  import Dot from './Dot.svelte';

  interface Props {
    color?: string;
    dotSize?: number;
    title: string;
    subtitle?: string;
    height?: number;
    /** Divider position. */
    border?: 'top' | 'bottom' | 'none';
    fontSize?: number;
    trailing?: Snippet;
    leading?: Snippet;
    onclick?: () => void;
    /** Accessible description for clickable rows. */
    label?: string;
  }

  let {
    color,
    dotSize = 8,
    title,
    subtitle,
    height = 36,
    border = 'top',
    fontSize = 13,
    trailing,
    leading,
    onclick,
    label,
  }: Props = $props();
</script>

{#snippet content()}
  {@render leading?.()}
  {#if color}<Dot {color} size={dotSize} />{/if}
  <span class="text">
    <span class="title">{title}</span>
    {#if subtitle}<span class="subtitle">{subtitle}</span>{/if}
  </span>
  {@render trailing?.()}
{/snippet}

{#if onclick}
  <button
    type="button"
    class="row border-{border}"
    style:height="{height}px"
    style:font-size="{fontSize}px"
    aria-label={label}
    {onclick}>{@render content()}</button
  >
{:else}
  <div class="row border-{border}" style:height="{height}px" style:font-size="{fontSize}px">
    {@render content()}
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    padding: 0;
    background: transparent;
    border: 0;
    color: inherit;
    text-align: left;
  }
  button.row {
    cursor: pointer;
    border-radius: 0;
  }
  button.row:hover .title {
    color: #fff;
    text-decoration: underline;
    text-decoration-color: rgba(255, 255, 255, 0.35);
    text-underline-offset: 3px;
  }
  .border-top {
    border-top: 1px solid var(--k-line);
  }
  .border-bottom {
    border-bottom: 1px solid var(--k-line-soft);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .subtitle {
    font-size: 11px;
    color: var(--k-text-4);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>

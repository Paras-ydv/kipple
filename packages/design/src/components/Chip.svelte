<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    radius?: number;
    padding?: string;
    children: Snippet;
  }

  let { radius = 14, padding = '0 11px 0 9px', children }: Props = $props();
</script>

<span class="chip k-chip" style:border-radius="{radius}px" style:padding>{@render children()}</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
    font-size: 12px;
  }
</style>

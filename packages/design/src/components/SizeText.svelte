<script lang="ts">
  import { formatBytes, formatDelta, splitBytes } from '../format.ts';

  interface Props {
    bytes: number;
    /** Signed growth delta ("+6.2 GB"). */
    delta?: boolean;
    /** Render number and unit as separate spans (display numbers). */
    split?: boolean;
    class?: string;
  }

  let { bytes, delta = false, split = false, class: className = '' }: Props = $props();
  const parts = $derived(splitBytes(bytes));
</script>

{#if split}
  <span class="size {className}" aria-label={formatBytes(bytes)}
    ><span class="value" aria-hidden="true">{parts.value}</span><span class="unit" aria-hidden="true"
      >{parts.unit}</span
    ></span
  >
{:else}
  <span class="size {className}">{delta ? formatDelta(bytes) : formatBytes(bytes)}</span>
{/if}

<style>
  .size {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>

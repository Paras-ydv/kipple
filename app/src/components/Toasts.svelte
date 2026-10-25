<script lang="ts">
  import { fly } from 'svelte/transition';
  import { ui } from '../lib/stores/ui.svelte.ts';
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each ui.toasts as t (t.id)}
    <div class="toast k-chip" transition:fly={{ y: 8, duration: 150 }}>{t.text}</div>
  {/each}
</div>

<style>
  .toasts {
    position: absolute;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    z-index: 60;
    pointer-events: none;
  }
  .toast {
    padding: 9px 16px;
    border-radius: 18px;
    font-size: 13px;
    max-width: 480px;
  }
</style>

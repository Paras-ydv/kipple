<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    subtitle?: string;
    children?: Snippet;
  }

  let { title, subtitle, children }: Props = $props();
</script>

<header class="toolbar" data-tauri-drag-region>
  <div class="titles" data-tauri-drag-region>
    <h1 class="title">{title}</h1>
    {#if subtitle}<span class="subtitle">{subtitle}</span>{/if}
  </div>
  {#if children}<div class="controls">{@render children()}</div>{/if}
</header>

<style>
  .toolbar {
    height: 44px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .titles {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .title {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
    line-height: 1.3;
  }
  .subtitle {
    font-size: 11px;
    color: var(--k-text-4);
    white-space: nowrap;
  }
  .controls {
    display: flex;
    gap: 10px;
    align-items: center;
  }
</style>

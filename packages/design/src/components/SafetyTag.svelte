<script lang="ts">
  import { safety, type SafetyLevel } from '../tokens.ts';

  interface Props {
    level: SafetyLevel;
    /** Fixed 58 px column width (Projects detail). */
    fixed?: boolean;
  }

  let { level, fixed = false }: Props = $props();
  const t = $derived(safety[level]);
</script>

<!-- Safety is a word plus a shape: Safe plain, Review a leading ring, Keep a leading bar (docs/UI.md §2.3). -->
<span class="tag {level}" class:fixed style:background={t.bg} style:color={t.fg}>
  {#if level === 'review'}<span class="mark ring" aria-hidden="true"></span>{/if}
  {#if level === 'keep'}<span class="mark bar" aria-hidden="true"></span>{/if}
  {t.word}
</span>

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    height: 20px;
    padding: 0 8px;
    border-radius: var(--k-r-row);
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    flex: none;
  }
  .fixed {
    width: 58px;
    padding: 0;
  }
  .mark {
    display: inline-block;
    flex: none;
  }
  .ring {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
  }
  .bar {
    width: 2px;
    height: 8px;
    border-radius: 1px;
    background: currentColor;
  }
</style>

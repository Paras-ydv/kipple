<script lang="ts">
  import { flags, type FlagKind } from '../tokens.ts';

  interface Props {
    kind: FlagKind;
    text: string;
  }

  let { kind, text }: Props = $props();
  const t = $derived(flags[kind]);
</script>

<span class="flag" style:background={t.bg} style:color={t.fg}>{text}</span>

<style>
  .flag {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: var(--k-r-row);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    flex: none;
  }
</style>

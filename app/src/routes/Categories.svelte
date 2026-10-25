<script lang="ts">
  // Categories (docs/UI.md §3.8): every item of one category.
  import { Dot, GlassPanel, SafetyTag, SearchField, Segmented, categoryColor, categoryLabel, formatAgo, formatBytes, plural } from '@kipple/design';
  import Toolbar from '../components/Toolbar.svelte';
  import { router } from '../lib/router.svelte.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';

  interface Props {
    id?: string;
  }
  let { id }: Props = $props();
  let search = $state('');
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (ui.searchRequest) input?.focus();
  });

  const cats = $derived(data.categories);
  const current = $derived(cats.find((c) => c.category === id)?.category ?? cats[0]?.category ?? 'container');
  const items = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return data.items
      .filter((i) => i.category === current)
      .filter((i) => !q || i.title.toLowerCase().includes(q) || (i.project?.name ?? '').toLowerCase().includes(q))
      .sort((a, b) => b.size.reclaimable - a.size.reclaimable);
  });
  const summary = $derived(cats.find((c) => c.category === current));
  const shown = $derived(items.slice(0, 300));
</script>

<Toolbar
  title={categoryLabel(current)}
  subtitle={summary ? `${plural(summary.items, 'item')}, ${formatBytes(summary.reclaimable)}` : ''}
>
  <SearchField label="Search items" bind:value={search} bind:input />
</Toolbar>

<div class="tabs">
  <Segmented
    label="Categories"
    value={current}
    onchange={(c) => router.go({ name: 'categories', id: c }, { replace: true })}
    segments={cats.map((c) => ({ value: c.category, label: categoryLabel(c.category), detail: formatBytes(c.reclaimable) }))}
  />
</div>

<GlassPanel tag="section" radius={20} padding="6px 18px" class="list" aria-label="{categoryLabel(current)} items">
  <ul>
    {#each shown as i (i.id)}
      {@const locked = i.safety === 'keep'}
      <li>
        <input
          type="checkbox"
          checked={planStore.has(i.id)}
          disabled={locked}
          aria-label="Add {i.title} to the plan"
          onchange={(e) => ((e.currentTarget as HTMLInputElement).checked ? planStore.add([i.id]) : planStore.remove([i.id]))}
        />
        <Dot color={categoryColor(i.category)} />
        <button type="button" class="name" onclick={() => ui.openExplain(i.title, [i])}>
          <span class="t">{i.title}</span>
          <span class="s">{[i.project?.name, i.last_used ? formatAgo(i.last_used.at).toLowerCase() : ''].filter(Boolean).join(', ')}</span>
        </button>
        <span class="size">{formatBytes(i.size.reclaimable)}</span>
        <SafetyTag level={i.safety} fixed />
      </li>
    {/each}
  </ul>
  {#if items.length > shown.length}<p class="more">Showing the largest 300 of {items.length}.</p>{/if}
  {#if !items.length}<p class="more">Nothing matches.</p>{/if}
</GlassPanel>

<style>
  .tabs {
    overflow-x: auto;
    flex: none;
  }
  :global(.list) {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 46px;
    border-bottom: 1px solid var(--k-line-soft);
  }
  input {
    width: 16px;
    height: 16px;
    accent-color: var(--k-accent);
    margin: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    border: 0;
    background: transparent;
    padding: 0;
    text-align: left;
  }
  .t {
    font-size: 13px;
    font-weight: 600;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name:hover .t {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .s {
    font-size: 11px;
    color: var(--k-text-4);
  }
  .size {
    width: 72px;
    text-align: right;
    font-size: 13px;
    color: var(--k-text-2);
  }
  .more {
    font-size: 12px;
    color: var(--k-text-4);
    margin: 10px 0;
  }
</style>

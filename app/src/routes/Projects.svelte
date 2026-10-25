<script lang="ts">
  // Projects (docs/UI.md §3.2): a shelf of stacks and the selected project's items.
  import type { ProjectSummary } from '@kipple/client';
  import {
    Button,
    Dot,
    GlassPanel,
    SafetyTag,
    SearchField,
    Segmented,
    StackSlabs,
    Stage,
    categoryColor,
    formatAge,
    formatBytes,
    layoutStack,
    sharedScale,
    tildePath,
    type StackLayer,
  } from '@kipple/design';
  import Toolbar from '../components/Toolbar.svelte';
  import { groupItems, platformFinder } from '../lib/derive.ts';
  import { reveal } from '../lib/native.ts';
  import { router } from '../lib/router.svelte.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';

  interface Props {
    id?: string;
  }
  let { id }: Props = $props();

  type Filter = 'all' | 'untouched' | 'deleted';
  let filter: Filter = $state('all');
  let search = $state('');
  let searchInput: HTMLInputElement | undefined = $state();

  const FIG_W = 1030;
  const FIG_H = 429;
  const FLOOR = 300;
  const STACK_W = 92;
  const DAY = 86_400_000;
  const now = Date.now();

  $effect(() => {
    if (ui.searchRequest) searchInput?.focus();
  });

  const ageDays = (p: ProjectSummary) => (p.last_used ? (now - Date.parse(p.last_used.at)) / DAY : Infinity);
  const isDull = (p: ProjectSummary) => !p.exists || ageDays(p) >= 180;

  const all = $derived(data.projects);
  const totalBytes = $derived(all.reduce((s, p) => s + p.total.reclaimable, 0));
  const deletedCount = $derived(all.filter((p) => !p.exists).length);

  const list = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return all.filter(
      (p) =>
        (filter === 'all' || (filter === 'untouched' ? ageDays(p) >= 30 : !p.exists)) &&
        (!q || p.name.toLowerCase().includes(q) || p.root.toLowerCase().includes(q)),
    );
  });
  const shelf = $derived(list.slice(0, 7));

  const selected = $derived(list.find((p) => p.id === id) ?? shelf[0]);

  const layouts = $derived.by(() => {
    const layers = shelf.map((p) =>
      Object.entries(p.by_category)
        .filter(([, b]) => (b ?? 0) > 0)
        .map(([c, b]): StackLayer => ({ key: c, color: categoryColor(c), value: b ?? 0 })),
    );
    const scale = sharedScale(
      shelf.map((p) => p.total.reclaimable),
      layers.map((l) => l.length),
      STACK_W,
      210,
    );
    return shelf.map((p, i) => {
      const cx = 170 + 115 * i;
      return { p, cx, layout: layoutStack(layers[i] ?? [], { cx, w: STACK_W, y0: FLOOR - 210, y1: FLOOR, scale }) };
    });
  });

  const items = $derived(selected ? data.items.filter((i) => i.project?.id === selected.id && i.safety !== 'keep') : []);
  const groups = $derived(groupItems(items));
  const safeIds = $derived(items.filter((i) => i.safety === 'safe').map((i) => i.id));
  const safeBytes = $derived(selected?.safety_mix.safe ?? 0);

  function ageText(p: ProjectSummary): string {
    if (!p.exists) return 'Folder deleted';
    if (!p.last_used) return '';
    const d = ageDays(p);
    return d < 1 ? 'Today' : `${formatAge(p.last_used.at)} ago`;
  }

  function lastLine(p: ProjectSummary): string {
    if (!p.exists) return 'The folder is gone. Only tool data is left.';
    if (!p.last_used) return '';
    const d = ageDays(p);
    const what = p.last_used.source === 'git_commit' ? 'Last commit' : 'Last change';
    return d < 1 ? `${what} today` : `${what} ${formatAge(p.last_used.at)} ago`;
  }

  function select(p: ProjectSummary) {
    router.go({ name: 'projects', id: p.id }, { replace: true });
  }

  function onShelfKey(e: KeyboardEvent) {
    if (!selected) return;
    const i = shelf.findIndex((p) => p.id === selected.id);
    let next = -1;
    if (e.key === 'ArrowRight') next = Math.min(shelf.length - 1, i + 1);
    else if (e.key === 'ArrowLeft') next = Math.max(0, i - 1);
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = shelf.length - 1;
    if (next < 0) return;
    e.preventDefault();
    const p = shelf[next];
    if (p) {
      select(p);
      requestAnimationFrame(() => document.getElementById(`stack-${p.id}`)?.focus());
    }
  }

  function addToPlan() {
    planStore.add(safeIds);
    router.go({ name: 'review' });
  }
</script>

<Toolbar title="Projects" subtitle="{all.length} projects holding {formatBytes(totalBytes)}">
  <Segmented
    label="Filter projects"
    bind:value={filter}
    segments={[
      { value: 'all', label: 'All' },
      { value: 'untouched', label: 'Untouched 30+ days' },
      { value: 'deleted', label: 'Folder deleted', detail: deletedCount ? String(deletedCount) : undefined },
    ]}
  />
  <SearchField label="Search projects" bind:value={search} bind:input={searchInput} />
</Toolbar>

{#if !shelf.length}
  <div class="empty k-glass" role="status">
    {search ? `No project matches “${search}”.` : 'No projects here.'}
  </div>
{:else}
  <Stage
    width={FIG_W}
    height={FIG_H}
    fluid
    shadow="sm"
    floorY={FLOOR + 8}
    label="{shelf.length} stacks of glass slabs, one per project, as tall as the space each takes. {shelf.filter(isDull).length} are dull because they have not been touched in months."
    class="shelf"
  >
    {#each layouts as { p, layout } (p.id)}
      <StackSlabs {layout} dull={isDull(p)} />
    {/each}
    {#snippet overlay()}
      <div class="stacks" role="listbox" aria-label="Projects" tabindex="-1" onkeydown={onShelfKey}>
        {#each layouts as { p, cx } (p.id)}
          {@const sel = p.id === selected?.id}
          <button
            type="button"
            id="stack-{p.id}"
            role="option"
            aria-selected={sel}
            tabindex={sel ? 0 : -1}
            class="stack-btn"
            class:sel
            style:left="calc({cx - 64} * var(--sx))"
            style:width="calc(128 * var(--sx))"
            aria-label="{p.name}, {formatBytes(p.total.reclaimable)}, {ageText(p)}"
            onclick={() => select(p)}
          >
            <span class="frame" aria-hidden="true"></span>
            <span class="caption" aria-hidden="true">
              <span class="c-name">{p.name}</span>
              <span class="c-size">{formatBytes(p.total.reclaimable)}</span>
              <span class="c-age" class:dusty={isDull(p)}>{ageText(p)}</span>
            </span>
          </button>
        {/each}
      </div>
    {/snippet}
  </Stage>

  {#if selected}
    <GlassPanel tag="section" radius={20} padding="18px 20px" class="detail" aria-labelledby="pd-name">
      <div class="who">
        <h2 id="pd-name">{selected.name}</h2>
        <span class="path k-mono">{tildePath(selected.root)}</span>
        <span class="last" class:dusty={isDull(selected)}>{lastLine(selected)}</span>
        <span class="total">{formatBytes(selected.total.reclaimable)}</span>
      </div>
      <ul class="items" aria-label="What {selected.name} holds">
        {#each groups as g (g.key)}
          <li>
            <button type="button" onclick={() => ui.openExplain(g.title, g.items)} aria-label="{g.title}, {formatBytes(g.bytes)}, {g.safety}. Explain.">
              <Dot color={g.color} />
              <span class="i-name">{g.title}</span>
              <span class="i-size">{formatBytes(g.bytes)}</span>
              <SafetyTag level={g.safety} fixed />
            </button>
          </li>
        {/each}
      </ul>
      <div class="acts">
        {#if safeBytes > 0}
          <Button variant="tint" size="md" block onclick={addToPlan}>Add {formatBytes(safeBytes)} to plan</Button>
        {:else}
          <Button variant="tint" size="md" block disabled>Nothing safe here</Button>
        {/if}
        <Button variant="clear" size="md" block onclick={() => reveal(selected.root, platformFinder())} disabled={!selected.exists}>
          {platformFinder()}
        </Button>
      </div>
    </GlassPanel>
  {/if}
{/if}

<style>
  .empty {
    border-radius: 20px;
    padding: 24px;
    font-size: 13px;
    color: var(--k-text-3);
  }
  :global(.shelf) {
    max-width: 1030px;
  }
  .stacks {
    position: absolute;
    inset: 0;
  }
  .stack-btn {
    position: absolute;
    top: calc(64 * var(--sy));
    bottom: calc(18 * var(--sy));
    border: 0;
    padding: 0;
    background: transparent;
    cursor: pointer;
    border-radius: 18px;
  }
  .stack-btn:focus-visible {
    outline: none;
  }
  .stack-btn:focus-visible .frame {
    outline: 2px solid var(--k-focus);
    outline-offset: 4px;
  }
  .frame {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    /* The frame spans y 64 to 300 of the 429 figure; the button spans 64 to 411. */
    height: calc(236 / 347 * 100%);
    border-radius: 18px;
    border: 1.5px solid transparent;
    transition:
      border-color var(--k-dur-response) ease-out,
      box-shadow var(--k-dur-response) ease-out;
  }
  .stack-btn:hover .frame {
    border-color: rgba(255, 255, 255, 0.18);
  }
  .sel .frame,
  .sel:hover .frame {
    border-color: rgba(127, 184, 255, 0.85);
    box-shadow:
      0 0 0 4px rgba(127, 184, 255, 0.16),
      0 0 40px rgba(80, 150, 255, 0.35);
  }
  .caption {
    position: absolute;
    left: 50%;
    bottom: 0;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    text-align: center;
    white-space: nowrap;
  }
  .c-name {
    font-size: 13px;
    font-weight: 650;
  }
  .sel .c-name {
    color: var(--k-accent-text);
  }
  .c-size {
    font-size: 12px;
    color: var(--k-text-2);
  }
  .c-age {
    font-size: 11px;
    color: var(--k-text-4);
  }
  .dusty {
    color: var(--k-growth);
  }

  :global(.detail) {
    flex: 1;
    display: flex;
    gap: 28px;
    min-height: 0;
  }
  .who {
    width: 220px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .path {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .last {
    font-size: 13px;
    margin-top: 2px;
    color: var(--k-text-3);
  }
  .last.dusty {
    color: var(--k-growth);
  }
  .total {
    margin-top: auto;
    font-family: var(--k-font-display);
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    column-gap: 24px;
    align-content: start;
    overflow: auto;
  }
  .items button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    border: 0;
    border-bottom: 1px solid var(--k-line-soft);
    background: transparent;
    font-size: 13px;
    text-align: left;
    padding: 0;
  }
  .items button:hover .i-name {
    text-decoration: underline;
    text-decoration-color: rgba(255, 255, 255, 0.35);
    text-underline-offset: 3px;
  }
  .i-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .i-size {
    color: rgba(255, 255, 255, 0.75);
  }
  .acts {
    width: 196px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    justify-content: flex-end;
  }
</style>

<script lang="ts">
  // Overview (docs/UI.md §3.1).
  import {
    Button,
    Chip,
    DiskBar,
    Dot,
    Icon,
    StackSlabs,
    Stage,
    categories as catTokens,
    categoryColor,
    categoryLabel,
    formatAgo,
    formatBytes,
    formatCount,
    formatDelta,
    formatDuration,
    layoutStack,
    sharedScale,
    splitBytes,
    type CategoryId,
  } from '@kipple/design';
  import Toolbar from '../components/Toolbar.svelte';
  import { growth, reviewPhrase } from '../lib/derive.ts';
  import { router } from '../lib/router.svelte.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';

  const FIG_W = 552;
  const FIG_H = 690;
  const CX = 190;
  const SLAB_W = 220;
  const Y0 = 120;
  const Y1 = 580;

  const top6 = $derived(data.categories.slice(0, 6));
  let lastTotal = $state(0);
  $effect(() => {
    if (!data.scanning && data.total) lastTotal = data.total;
  });

  const layout = $derived.by(() => {
    const layers = top6.map((c) => ({
      key: c.category,
      color: categoryColor(c.category),
      value: c.reclaimable,
      label: categoryLabel(c.category),
    }));
    if (data.scanning) {
      const target = Math.max(lastTotal, data.total, 50e9);
      const scale = sharedScale([target], [Math.max(6, layers.length)], SLAB_W, Y1 - Y0);
      return layoutStack(layers, { cx: CX, w: SLAB_W, y0: Y0, y1: Y1, scale });
    }
    return layoutStack(layers, { cx: CX, w: SLAB_W, y0: Y0, y1: Y1 });
  });

  const total = $derived(splitBytes(data.total));
  const safe = $derived(data.report?.summary.by_safety.safe ?? 0);

  const safeLine = $derived.by(() => {
    const list = reviewPhrase(data.report?.summary.categories ?? []);
    if (safe === 0) return `Nothing here is marked safe. It's mostly ${list}, so you decide.`;
    if (!list) return `All of it, ${formatBytes(safe)}, is safe to remove right now.`;
    return `${formatBytes(safe)} is safe to remove right now. The rest is ${list}, so you decide.`;
  });

  const scanInfo = $derived(data.report?.scan);
  const now = new Date();
  const week = $derived(growth(data.history, now.getTime() - 7 * 86_400_000));

  const scanLine = $derived.by(() => {
    if (data.scanning) {
      return data.scanFiles > 0 ? `Scanning. ${formatCount(data.scanFiles)} files so far.` : 'Scanning.';
    }
    if (!scanInfo?.finished_at) return 'Not scanned yet.';
    return `Scanned ${formatAgo(scanInfo.finished_at).toLowerCase()}. ${formatCount(scanInfo.entries_seen)} files in ${formatDuration(scanInfo.duration_ms)}.`;
  });

  const diskParts = $derived(
    data.categories.map((c) => ({ key: c.category, color: categoryColor(c.category), bytes: c.reclaimable })),
  );

  const stackLabel = $derived(
    `A floating stack of ${top6.length} glass slabs, one per kind of kipple, sized by how much space each takes: ` +
      top6.map((c) => `${categoryLabel(c.category)} ${formatBytes(c.reclaimable)}`).join(', '),
  );

  function reclaim() {
    planStore.pickSafe();
    router.go({ name: 'review' });
  }

  function openCategory(id: string) {
    router.go({ name: 'categories', id });
  }
</script>

<Toolbar title="Overview" subtitle={data.volume?.name ?? ''}>
  <div class="group k-clear">
    <button type="button" aria-label="Search" title="Search (/)" onclick={() => { router.go({ name: 'projects' }); ui.searchRequest++; }}>
      <Icon name="search" size={16} stroke="#fff" />
    </button>
    <button type="button" aria-label="Filter by category" title="Categories" onclick={() => router.go({ name: 'categories' })}>
      <Icon name="filter" size={16} stroke="#fff" />
    </button>
    <button type="button" aria-label="Share card" title="Share card" onclick={() => (ui.shareOpen = true)}>
      <Icon name="share" size={16} stroke="#fff" />
    </button>
  </div>
  <button type="button" class="scan k-clear" onclick={() => (data.scanning ? data.cancelScan() : data.scan())} aria-keyshortcuts="Meta+R">
    <span class="spin" class:on={data.scanning}><Icon name="refresh" size={15} stroke="#fff" strokeWidth={2.2} /></span>
    {data.scanning ? 'Stop scan' : 'Scan'}
  </button>
</Toolbar>

<div class="body">
  <Stage width={FIG_W} height={FIG_H} label={stackLabel} floorY={Y1 + 20} class="figure">
    <StackSlabs {layout} grow={data.scanning} />
    {#snippet overlay()}
      {#each layout.slabs as s (s.key)}
        <button
          type="button"
          class="label"
          style:left="calc({s.cx + s.w / 2 + 12} * var(--sx))"
          style:top="calc({s.cy} * var(--sy))"
          onclick={() => openCategory(s.key)}
          aria-label="{s.label}, {formatBytes(s.value)}. Show items."
        >
          <span class="pin" aria-hidden="true"></span>
          <span class="leader" aria-hidden="true"></span>
          <Chip>
            <Dot color={s.color} />
            <span class="chip-name">{catTokens[s.key as CategoryId]?.label ?? s.label}</span>
            <span class="chip-size">{formatBytes(s.value)}</span>
          </Chip>
        </button>
      {/each}
    {/snippet}
  </Stage>

  <section class="summary" aria-labelledby="ov-total">
    <div class="intro">
      <span class="scanline" aria-live="polite">{scanLine}</span>
      <div class="hero" id="ov-total" role="heading" aria-level="2" aria-label="{formatBytes(data.total)} of kipple on this Mac">
        <span class="num">{total.value}</span>
        <span class="unit">{total.unit}</span>
      </div>
      <span class="subhead">of kipple on this Mac</span>
      {#if !data.scanning}
        {#if data.total > 0}
          <p class="para">
            {safeLine}
          </p>
        {:else}
          <p class="para">Nothing to reclaim. kipple checked {formatCount(scanInfo?.entries_seen ?? 0)} files.</p>
        {/if}
      {/if}
    </div>

    {#if !data.scanning && safe > 0}
      <div class="buttons">
        <Button variant="tint" size="lg" pad={22} onclick={reclaim} weight={650}>Reclaim {formatBytes(safe)}</Button>
        <Button variant="clear" size="lg" pad={20} href="#/projects" weight={550}>Review everything</Button>
      </div>
    {/if}

    {#if data.error}
      <div class="error k-glass" role="alert">{data.error}</div>
    {/if}

    {#if data.volume}
      <DiskBar volume={data.volume.name} capacity={data.volume.capacity} free={data.volume.free} parts={diskParts} />
    {/if}

    {#if week.rows.length}
      <div class="growth">
        <div class="growth-head">
          <h2>Grew this week</h2>
          <span class="delta">{formatDelta(week.total)}</span>
        </div>
        <ul>
          {#each week.rows.slice(0, 3) as g (g.key)}
            <li>
              <Dot color={g.color} />
              <span class="g-name">{g.label}</span>
              <span class="g-delta">{formatDelta(g.delta)}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </section>
</div>

<style>
  .group {
    display: flex;
    height: 36px;
    border-radius: 18px;
    padding: 0 4px;
    align-items: center;
  }
  .group button {
    width: 36px;
    height: 30px;
    border: 0;
    border-radius: 15px;
    background: transparent;
    display: grid;
    place-items: center;
  }
  .group button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .scan {
    height: 36px;
    border-radius: 18px;
    padding: 0 16px 0 12px;
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 13px;
    font-weight: 600;
  }
  .spin.on {
    animation: spin 1s linear infinite;
  }
  .spin {
    display: grid;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .body {
    flex: 1;
    display: flex;
    gap: 36px;
    padding-top: 0;
    min-height: 0;
  }
  .body :global(.figure) {
    height: 100%;
    max-height: 690px;
    width: auto !important;
  }
  .label {
    position: absolute;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    border: 0;
    padding: 0;
    background: transparent;
    cursor: pointer;
  }
  .label:hover :global(.chip) {
    border-color: rgba(255, 255, 255, 0.4);
  }
  .pin {
    width: 7px;
    height: 7px;
    border-radius: 4px;
    background: #fff;
    box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.18);
  }
  .leader {
    width: 12px;
    height: 1px;
    background: rgba(255, 255, 255, 0.55);
  }
  .label :global(.chip) {
    height: 28px;
  }
  .chip-name {
    font-weight: 600;
  }
  .chip-size {
    color: rgba(255, 255, 255, 0.7);
  }

  .summary {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 22px;
    padding-top: 18px;
    overflow: auto;
  }
  .intro {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .scanline {
    font-size: 13px;
    color: var(--k-text-3);
  }
  .hero {
    display: flex;
    align-items: baseline;
    gap: 10px;
    font-family: var(--k-font-display);
  }
  .num {
    font-size: var(--k-fs-hero);
    line-height: 0.9;
    font-weight: 700;
    letter-spacing: -0.05em;
  }
  .unit {
    font-size: var(--k-fs-hero-unit);
    font-weight: 600;
    letter-spacing: -0.02em;
    color: var(--k-text-2);
  }
  .subhead {
    font-size: var(--k-fs-subhead);
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--k-text-sub);
  }
  .para {
    margin: 4px 0 0;
    font-size: 15px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.66);
    max-width: 40ch;
  }
  .buttons {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }
  .error {
    border-radius: 16px;
    padding: 12px 16px;
    font-size: 13px;
  }
  .growth {
    display: flex;
    flex-direction: column;
  }
  .growth-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding-bottom: 8px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }
  .delta {
    font-size: 15px;
    font-weight: 650;
    color: var(--k-growth);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    border-top: 1px solid var(--k-line);
    font-size: 13px;
  }
  .g-name {
    flex: 1;
  }
  .g-delta {
    color: var(--k-text-2);
  }
</style>

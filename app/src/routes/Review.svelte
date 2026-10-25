<script lang="ts">
  // Review plan, "Reclaim" (docs/UI.md §3.4).
  import type { Category } from '@kipple/client';
  import { client } from '../lib/client.ts';
  import {
    Button,
    Chip,
    Dot,
    GlassPanel,
    StackSlabs,
    Stage,
    Switch,
    categoryColor,
    formatBytes,
    formatDay,
    layoutStack,
    sharedScale,
    type StackLayer,
  } from '@kipple/design';
  import { entryBytes } from '@kipple/client';
  import Toolbar from '../components/Toolbar.svelte';
  import { groupPlanEntries } from '../lib/derive.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';

  const FIG_W = 600;
  const FIG_H = 690;
  const FLOOR = 610;
  const W = 150;
  const LEFT = 120;
  const RIGHT = 480;

  $effect(() => {
    planStore.ensure();
  });

  const plan = $derived(planStore.plan);
  const totals = $derived(plan?.totals);
  const groups = $derived(plan ? groupPlanEntries(plan.entries) : { removed: [], quarantined: [] });
  const rows = $derived(groups.removed.length + groups.quarantined.length);
  const allSafe = $derived(!!plan && plan.entries.every((e) => e.item.safety === 'safe'));
  const subtitle = $derived(
    !plan
      ? 'Nothing selected'
      : planStore.pickedByKipple && allSafe
        ? `${rows} safe items, picked by kipple`
        : `${rows} ${rows === 1 ? 'item' : 'items'} you picked`,
  );

  // Before and after stacks.
  const result = $derived(planStore.phase === 'done' ? planStore.result : null);
  const today = $derived((result && planStore.before ? planStore.before.categories : data.categories).slice(0, 6));
  const removedByCat = $derived.by(() => {
    const m = new Map<Category, number>();
    for (const e of plan?.entries ?? []) m.set(e.item.category, (m.get(e.item.category) ?? 0) + entryBytes(e.item, e.action));
    return m;
  });
  const total = $derived(result && planStore.before ? planStore.before.total : data.total);
  const after = $derived(result ? data.total : Math.max(0, total - (totals?.reclaimable ?? 0)));
  const stacks = $derived.by(() => {
    const before: StackLayer[] = today.map((c) => ({ key: c.category, color: categoryColor(c.category), value: c.reclaimable }));
    const rest: StackLayer[] = today
      .map((c) => ({
        key: c.category,
        color: categoryColor(c.category),
        value: result
          ? (data.categories.find((d) => d.category === c.category)?.reclaimable ?? 0)
          : Math.max(0, c.reclaimable - (removedByCat.get(c.category) ?? 0)),
      }))
      .filter((l) => l.value > 1e8);
    const scale = sharedScale([total, after], [before.length, Math.max(1, rest.length)], W, FLOOR - 120);
    return {
      before: layoutStack(before, { cx: LEFT, w: W, y0: 120, y1: FLOOR, scale }),
      after: layoutStack(rest, { cx: RIGHT, w: W, y0: 120, y1: FLOOR, scale }),
    };
  });
  const reviewLeft = $derived(Math.max(0, (data.report?.summary.by_safety.review ?? 0) - (totals?.by_safety.review ?? 0)));
  const keepDays = $derived(data.report?.next?.keep_days ?? 7);
  const freedNow = $derived(totals ? totals.freed_now + (planStore.skipQuarantine ? totals.held_in_quarantine : 0) : 0);
  const held = $derived(totals && !planStore.skipQuarantine ? totals.held_in_quarantine : 0);

  // Result view.
  const releaseDay = $derived(formatDay(new Date(Date.now() + keepDays * 86_400_000)));
  let undoing = $state(false);

  async function undo() {
    if (!result) return;
    undoing = true;
    const ids = result.entries.map((e) => e.quarantine_id).filter((x): x is string => !!x);
    for (const id of ids) await client.restore(id);
    await data.refresh();
    undoing = false;
    ui.toast(`Restored ${ids.length === 1 ? 'one item' : `${ids.length} items`} from quarantine.`);
    planStore.dismissResult();
  }

  const skipped = $derived(result?.entries.filter((e) => e.outcome.status !== 'done') ?? []);
</script>

<Toolbar title="Review plan" subtitle={result ? 'Done' : subtitle}>
  {#if !result}
    <Button variant="clear" size="md" weight={550} onclick={() => (ui.selectionOpen = true)} height={36} pad={16}>
      Edit selection
    </Button>
  {/if}
</Toolbar>

<div class="body">
  <Stage
    width={FIG_W}
    height={FIG_H}
    shadow="sm"
    floorY={FLOOR + 10}
    class="stage"
    label="Two stacks of glass slabs. Left: today, {formatBytes(total)}. Right: after this plan, {formatBytes(after)}."
  >
    <StackSlabs layout={stacks.before} />
    <StackSlabs layout={stacks.after} />
    {#snippet overlay()}
      <div class="tag" style:left="calc({LEFT} * var(--sx))" style:top="calc({stacks.before.top - 70} * var(--sy))">
        <Chip radius={16} padding="7px 14px">
          <span class="tag-in"><span class="cap">{result ? 'Before' : 'Today'}</span><span class="val">{formatBytes(total)}</span></span>
        </Chip>
      </div>
      <div class="tag" style:left="calc({RIGHT} * var(--sx))" style:top="calc({stacks.after.top - 70} * var(--sy))">
        <Chip radius={16} padding="7px 14px">
          <span class="tag-in"><span class="cap">{result ? 'Now' : 'After this plan'}</span><span class="val">{formatBytes(after)}</span></span>
        </Chip>
      </div>
      {#if totals || result}
        <div class="arrow" style:left="calc(300 * var(--sx))" style:top="calc(330 * var(--sy))">
          <svg width="54" height="14" viewBox="0 0 54 14" fill="none" stroke="rgba(255,255,255,.7)" stroke-width="1.6" stroke-linecap="round" aria-hidden="true">
            <path d="M2 7h48M44 2l6 5-6 5" />
          </svg>
          <Chip radius={14} padding="6px 12px"><span class="out">{formatBytes(result ? result.freed_now + result.held_in_quarantine : (totals?.reclaimable ?? 0))} out</span></Chip>
        </div>
      {/if}
    {/snippet}
  </Stage>

  <section class="plan" aria-labelledby="rv-total">
    {#if result}
      <div class="lead">
        <span class="kicker">This plan reclaimed</span>
        <span class="big" id="rv-total">{formatBytes(result.freed_now + result.held_in_quarantine)}</span>
      </div>
      <p class="done-line">
        {formatBytes(result.freed_now)} is free now.
        {#if result.held_in_quarantine > 0}{formatBytes(result.held_in_quarantine)} is in quarantine until {releaseDay}.{/if}
      </p>
      {#if skipped.length}
        <GlassPanel radius={18} padding="12px 16px">
          <p class="skipped-title">{skipped.length === 1 ? 'One item was' : `${skipped.length} items were`} left alone</p>
          <ul class="skipped">
            {#each skipped as e (e.item_id)}
              <li>{e.title}: {e.outcome.status === 'skipped' ? e.outcome.reason : e.outcome.status === 'failed' ? e.outcome.error : ''}</li>
            {/each}
          </ul>
        </GlassPanel>
      {/if}
      <div class="bottom">
        <Button variant="tint" size="xl" block onclick={() => (ui.shareOpen = true)}>Make a share card</Button>
        {#if result.held_in_quarantine > 0}
          <Button variant="clear" size="md" block onclick={undo} disabled={undoing}>{undoing ? 'Restoring' : 'Undo'}</Button>
        {/if}
        <Button variant="plain" size="sm" block onclick={() => planStore.dismissResult()}>Make another plan</Button>
      </div>
    {:else if !plan}
      <div class="lead">
        <span class="kicker">This plan reclaims</span>
        <span class="big" id="rv-total">0 GB</span>
      </div>
      <p class="done-line">Nothing is selected. Pick safe items, or add items from Projects and AI history.</p>
      <div class="bottom">
        <Button variant="tint" size="xl" block onclick={() => planStore.pickSafe()}>Pick the safe items</Button>
        <Button variant="clear" size="md" block onclick={() => planStore.restoreSaved() || ui.toast('There is no saved plan.')}>Open the saved plan</Button>
      </div>
    {:else}
      <div class="lead">
        <span class="kicker">This plan reclaims</span>
        <span class="big" id="rv-total">{formatBytes(totals?.reclaimable ?? 0)}</span>
      </div>
      <div class="lines">
        {#if freedNow > 0}
          <div class="line">
            <Dot color="var(--k-positive)" />
            <span><strong>{formatBytes(freedNow)} is freed right away</strong> <span class="dim">{planStore.skipQuarantine && held === 0 && totals && totals.held_in_quarantine > 0 ? 'with no undo.' : 'by the tools that made it. They download it again when needed.'}</span></span>
          </div>
        {/if}
        {#if held > 0}
          <div class="line">
            <Dot color="var(--k-accent-text)" />
            <span><strong>{formatBytes(held)} waits in quarantine for {keepDays} days</strong> <span class="dim">so you can undo it, then it is deleted.</span></span>
          </div>
        {/if}
      </div>

      <GlassPanel radius={18} padding="4px 16px" class="groups">
        {#each [{ title: 'Removed by the tool that made it', list: groups.removed }, { title: 'Moved to quarantine', list: groups.quarantined }].filter((g) => g.list.length) as g, gi (g.title)}
          <div class="group" class:last={gi === 1 || !groups.quarantined.length}>
            <div class="g-head"><span>{g.title}</span><span>{formatBytes(g.list.reduce((s, x) => s + x.bytes, 0))}</span></div>
            {#each g.list.slice(0, 3) as r (r.key)}
              <button type="button" class="g-row" onclick={() => ui.openExplain(r.title, r.items)}>
                <Dot color={r.color} size={6} />
                <span class="g-name">{r.title}</span>
                {#if r.safety === 'review'}<span class="k-visually-hidden">Review</span><span class="rv" aria-hidden="true">Review</span>{/if}
                <span>{formatBytes(r.bytes)}</span>
              </button>
            {/each}
            {#if g.list.length > 3}
              <span class="more">and {g.list.length - 3} more: {g.list.slice(3).map((r) => r.short).join(', ')}</span>
            {/if}
          </div>
        {/each}
      </GlassPanel>

      {#if totals && totals.held_in_quarantine > 0}
        <label class="skip">
          <span>
            <span class="skip-title">Skip quarantine</span><br />
            <span class="dim4">Free all {formatBytes(totals.reclaimable)} now. No undo.</span>
          </span>
          <Switch label="Skip quarantine" bind:checked={planStore.skipQuarantine} />
        </label>
      {/if}

      <div class="bottom">
        {#if planStore.phase === 'running'}
          <div class="running k-clear" role="status" aria-live="polite">
            Reclaiming {planStore.progress.index} of {planStore.progress.total}. {planStore.progress.title}
          </div>
          <Button variant="clear" size="md" block onclick={() => planStore.stop()}>Stop after this item</Button>
        {:else}
          <Button variant="tint" size="xl" block onclick={() => planStore.reclaim()} disabled={planStore.building}>
            Reclaim {formatBytes(totals?.reclaimable ?? 0)}
          </Button>
          <Button variant="clear" size="md" block onclick={() => planStore.save()}>Save plan for later</Button>
        {/if}
        {#if reviewLeft > 0}
          <a class="foot" href="#/ai">
            {formatBytes(reviewLeft)} needs your decision: database volumes, emulators, models and chat history.
          </a>
        {/if}
      </div>
    {/if}
  </section>
</div>

<style>
  .body {
    flex: 1;
    display: flex;
    gap: 28px;
    min-height: 0;
  }
  .body :global(.stage) {
    height: 100%;
    max-height: 690px;
    width: auto !important;
  }
  .tag {
    position: absolute;
    transform: translateX(-50%);
  }
  .tag-in {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  .cap {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.65);
  }
  .val {
    font-size: 17px;
    font-weight: 700;
  }
  .arrow {
    position: absolute;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .out {
    font-size: 13px;
    font-weight: 700;
    color: var(--k-positive);
  }
  .plan {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-top: 8px;
    overflow: auto;
  }
  .lead {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .kicker {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.6);
  }
  .big {
    font-family: var(--k-font-display);
    font-size: 64px;
    line-height: 1;
    font-weight: 700;
    letter-spacing: -0.045em;
  }
  .lines {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
    line-height: 1.45;
  }
  .line {
    display: flex;
    gap: 10px;
  }
  .line :global(.dot) {
    margin-top: 5px;
  }
  strong {
    font-weight: 650;
  }
  .dim {
    color: var(--k-text-3);
  }
  .dim4 {
    color: var(--k-text-4);
  }
  :global(.groups) {
    display: flex;
    flex-direction: column;
  }
  .group {
    padding: 12px 0 10px;
    border-bottom: 1px solid var(--k-line-soft);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .group.last {
    border-bottom-color: transparent;
  }
  .g-head {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 650;
  }
  .g-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--k-text-2);
    border: 0;
    padding: 0;
    background: transparent;
    text-align: left;
  }
  .g-row:hover .g-name {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .g-name {
    flex: 1;
  }
  .rv {
    font-size: 10px;
    font-weight: 600;
    color: var(--k-review-fg);
  }
  .more {
    font-size: 12px;
    color: var(--k-text-4);
  }
  .skip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    cursor: pointer;
  }
  .skip-title {
    font-weight: 600;
  }
  .bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .running {
    height: 48px;
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    padding: 0 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .foot {
    font-size: 12px;
    color: var(--k-text-4);
    text-align: center;
    line-height: 1.4;
  }
  .done-line {
    margin: 0;
    font-size: 15px;
    line-height: 1.5;
    color: var(--k-text-2);
  }
  .skipped-title {
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 650;
  }
  .skipped {
    margin: 0;
    padding-left: 16px;
    font-size: 12px;
    color: var(--k-text-3);
  }
</style>

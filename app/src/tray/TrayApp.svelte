<script lang="ts">
  // Menu bar popover (docs/UI.md §3.7). In Tauri this is its own window under the
  // tray icon; in the browser preview it is drawn under a fake menu bar.
  import { KippleError, type Report, type SizePoint } from '@kipple/client';
  import { Button, Dot, StackSlabs, Wordmark, categoryColor, formatAgo, formatBytes, formatDelta, layoutStack } from '@kipple/design';
  import { client, native } from '../lib/client.ts';
  import { growth } from '../lib/derive.ts';
  import { hideTray, openMain } from '../lib/native.ts';

  let report = $state<Report | null>(null);
  let history = $state<SizePoint[]>([]);
  let volume = $state('');
  let failed = $state(false);
  let noScan = $state(false);
  let scanning = $state(false);

  async function load() {
    try {
      const [r, h, v] = await Promise.all([client.report(), client.history({}), client.volume()]);
      report = r;
      history = h;
      volume = v.name;
      failed = false;
      noScan = false;
    } catch (err) {
      if (KippleError.from(err).code === 'no_scan') noScan = true;
      else failed = true;
    }
  }

  $effect(() => {
    void load();
  });

  const total = $derived(report?.summary.reclaimable ?? 0);
  const safe = $derived(report?.summary.by_safety.safe ?? 0);
  const week = $derived(growth(history, Date.now() - 7 * 86_400_000));
  const layout = $derived(
    layoutStack(
      (report?.summary.categories ?? []).slice(0, 6).map((c) => ({ key: c.category, color: categoryColor(c.category), value: c.reclaimable })),
      { cx: 52, w: 76, y0: 14, y1: 126 },
    ),
  );
  const checked = $derived(report?.scan.finished_at ? `Checked ${formatAgo(report.scan.finished_at).toLowerCase()}` : '');

  async function scanNow() {
    scanning = true;
    try {
      await client.scan({});
      await load();
    } catch {
      failed = true;
    }
    scanning = false;
  }

  async function go(hash: string) {
    await openMain(hash);
    await hideTray();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') void hideTray();
  }

  const now = new Date();
  const clock = `${now.toLocaleDateString('en-GB', { weekday: 'short' })} ${now.getDate()} ${now.toLocaleDateString('en-GB', { month: 'short' })}  ${now.toLocaleTimeString('en-GB', { hour: 'numeric', minute: '2-digit' })}`;
</script>

<svelte:window {onkeydown} />

<div class="stage" class:native>
  {#if !native}
    <div class="menubar" aria-hidden="true">
      <div class="menus"><b>Finder</b><span>File</span><span>Edit</span><span>View</span><span>Go</span><span>Window</span><span>Help</span></div>
      <div class="status">
        <span class="pill k-pill">
          <svg width="10" height="16" viewBox="0 0 10 16"><rect x="0.5" y="1.5" width="9" height="2.6" rx="1" fill="#fff" opacity="0.7" /><rect x="0.5" y="5.6" width="9" height="3.4" rx="1" fill="#fff" opacity="0.85" /><rect x="0.5" y="10.5" width="9" height="4.5" rx="1.2" fill="#fff" /></svg>
          <span>{formatBytes(total)}</span>
        </span>
        <span>{clock}</span>
      </div>
    </div>
  {/if}

  <div class="pop k-popover" role="dialog" aria-label="kipple">
    <div class="head">
      <Wordmark size={17} />
      <span class="checked">{checked}</span>
    </div>

    {#if noScan}
      <p class="fail">kipple hasn't scanned this Mac yet. Scanning only reads; nothing is changed.</p>
      <Button variant="clear" size="md" block disabled={scanning} onclick={scanNow}>{scanning ? 'Scanning…' : 'Scan now'}</Button>
    {:else if failed}
      <p class="fail">kipple can't read its data yet. Open kipple to see why.</p>
      <Button variant="clear" size="md" block onclick={() => go('#/')}>Open kipple</Button>
    {:else}
      <div class="main">
        <svg class="thumb" viewBox="0 0 104 140" role="img" aria-label="Your kipple as a stack of glass slabs">
          <StackSlabs {layout} floor={false} />
        </svg>
        <div class="nums">
          <span class="total">{formatBytes(total)}</span>
          <span class="where">of kipple on {volume}</span>
          <span class="safe">{formatBytes(safe)} safe to remove</span>
        </div>
      </div>

      {#if week.rows.length}
        <div class="growth">
          <div class="g-head"><span class="g-label">Grew this week</span><span class="g-total">{formatDelta(week.total)}</span></div>
          {#each week.rows.slice(0, 3) as g (g.key)}
            <div class="g-row"><Dot color={g.color} /><span class="g-name">{g.label}</span><span class="g-delta">{formatDelta(g.delta)}</span></div>
          {/each}
        </div>
      {/if}

      <div class="acts">
        <Button variant="tint" size="md" block weight={650} height={42} fontSize={14} onclick={() => go('#/review')}>
          Review {formatBytes(safe)} of safe items
        </Button>
        <div class="two">
          <Button variant="clear" size="md" block height={38} onclick={() => go('#/')}>Open kipple</Button>
          <Button variant="clear" size="md" block height={38} onclick={scanNow} disabled={scanning}>{scanning ? 'Scanning' : 'Scan now'}</Button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .stage {
    width: 100vw;
    min-height: 100vh;
    position: relative;
    background: var(--k-wall);
    overflow: hidden;
  }
  .stage.native {
    background: transparent;
    min-height: 0;
  }
  .menubar {
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    font-size: 13px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }
  .menus {
    display: flex;
    gap: 18px;
  }
  .status {
    display: flex;
    gap: 14px;
    align-items: center;
    white-space: pre;
  }
  .pill {
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
  }
  .pop {
    position: absolute;
    right: 120px;
    top: 40px;
    width: 380px;
    border-radius: var(--k-r-popover);
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .native .pop {
    position: relative;
    right: auto;
    top: auto;
    width: 100%;
    border-radius: var(--k-r-popover);
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .checked {
    font-size: 12px;
    color: var(--k-text-4);
  }
  .main {
    display: flex;
    gap: 16px;
    align-items: stretch;
  }
  .thumb {
    width: 104px;
    height: 140px;
    flex: none;
    border-radius: var(--k-r-thumb);
    background:
      radial-gradient(60% 42% at 50% 36%, rgba(70, 110, 255, 0.18), rgba(70, 110, 255, 0) 70%),
      var(--k-stage);
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.14);
  }
  .nums {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
  }
  .total {
    font-family: var(--k-font-display);
    font-size: 46px;
    line-height: 1;
    font-weight: 700;
    letter-spacing: -0.045em;
  }
  .where {
    font-size: 13px;
    color: rgba(255, 255, 255, 0.66);
  }
  .safe {
    font-size: 13px;
    color: var(--k-positive);
    font-weight: 600;
    margin-top: 6px;
  }
  .growth {
    display: flex;
    flex-direction: column;
  }
  .g-head {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    padding-bottom: 6px;
  }
  .g-label {
    color: var(--k-text-3);
  }
  .g-total {
    color: var(--k-growth);
    font-weight: 650;
  }
  .g-row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    border-top: 1px solid var(--k-line);
    font-size: 13px;
  }
  .g-name {
    flex: 1;
  }
  .g-delta {
    color: rgba(255, 255, 255, 0.75);
  }
  .acts {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .two {
    display: flex;
    gap: 8px;
  }
  .fail {
    margin: 0;
    font-size: 13px;
    color: var(--k-text-3);
  }
</style>

<script lang="ts">
  // Quarantine and history (docs/UI.md §3.5).
  import type { QuarantineEntry } from '@kipple/client';
  import { Button, Dot, GlassPanel, categoryColor, formatBytes, plural } from '@kipple/design';
  import Toolbar from '../components/Toolbar.svelte';
  import { actionWords } from '../lib/derive.ts';
  import { client } from '../lib/client.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';

  const DAY = 86_400_000;
  const total = $derived(data.quarantine.reduce((s, e) => s + e.size.reclaimable, 0));
  const batches = $derived.by(() => {
    const m = new Map<string, QuarantineEntry[]>();
    for (const e of data.quarantine) m.set(e.plan_id, [...(m.get(e.plan_id) ?? []), e]);
    return [...m.values()]
      .map((entries) => ({
        id: entries[0]?.plan_id ?? '',
        entries,
        created: entries[0]?.created_at ?? '',
        expires: entries[0]?.expires_at ?? '',
        bytes: entries.reduce((s, e) => s + e.size.reclaimable, 0),
      }))
      .sort((a, b) => b.created.localeCompare(a.created));
  });
  let open = $state<Record<string, boolean>>({});
  let busy = $state(false);

  const when = (iso: string) =>
    new Date(iso).toLocaleString('en-GB', { day: 'numeric', month: 'long', hour: '2-digit', minute: '2-digit' });
  const daysLeft = (iso: string) => Math.max(0, Math.ceil((Date.parse(iso) - Date.now()) / DAY));

  async function restore(entries: QuarantineEntry[]) {
    busy = true;
    let restored = 0;
    const skipped: string[] = [];
    for (const e of entries) {
      const r = await client.restore(e.id);
      restored += r.restored.length;
      skipped.push(...r.skipped.map((s) => s.reason));
    }
    await data.refresh();
    busy = false;
    ui.toast(skipped.length ? `Restored ${restored}. ${skipped[0]}` : `Restored ${plural(restored, 'item')}.`);
  }

  async function freeAll() {
    if (!data.quarantine.length) return;
    const ok = await ui.askConfirmText(`Free ${formatBytes(total)} now`, [
      `${formatBytes(total)} in quarantine is deleted for good. There is no undo.`,
    ]);
    if (!ok) return;
    busy = true;
    const r = await client.purge({});
    await data.refresh();
    busy = false;
    ui.toast(`Freed ${formatBytes(r.freed)}.`);
  }
</script>

<Toolbar
  title="Quarantine"
  subtitle={total > 0 ? `${formatBytes(total)} waiting, freed after 7 days` : 'Nothing waiting'}
/>

<div class="cols">
  <div class="left">
    <GlassPanel radius={20} padding="18px 20px" class="sum">
      <span class="num">{formatBytes(total)}</span>
      <p>
        {total > 0
          ? 'This space is still used until it is freed. Each batch is deleted 7 days after it was moved.'
          : 'Nothing is in quarantine. Items you reclaim wait here for 7 days so you can undo them.'}
      </p>
      {#if total > 0}
        <div><Button variant="clear" size="sm" onclick={freeAll} disabled={busy}>Free all now</Button></div>
      {/if}
    </GlassPanel>

    {#if batches.length}
      <GlassPanel tag="section" radius={20} padding="6px 18px" aria-label="Batches" class="batches">
        <ul>
          {#each batches as b (b.id)}
            <li class="batch">
              <div class="b-row">
                <button type="button" class="b-toggle" aria-expanded={!!open[b.id]} onclick={() => (open[b.id] = !open[b.id])}>
                  <span class="b-when">{when(b.created)}</span>
                  <span class="b-what">{plural(b.entries.length, 'item')}, {daysLeft(b.expires)} days left</span>
                </button>
                <span class="b-size">{formatBytes(b.bytes)}</span>
                <Button variant="clear" size="sm" onclick={() => restore(b.entries)} disabled={busy}>Restore</Button>
              </div>
              {#if open[b.id]}
                <ul class="entries">
                  {#each b.entries as e (e.id)}
                    <li>
                      <Dot color={categoryColor(e.item.category)} size={6} />
                      <span class="e-name">{e.item.title}{e.item.project ? `, ${e.item.project.name}` : ''}</span>
                      <span class="e-size">{formatBytes(e.size.reclaimable)}</span>
                      <button type="button" class="link" onclick={() => restore([e])} disabled={busy}>Restore</button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </li>
          {/each}
        </ul>
      </GlassPanel>
    {/if}
  </div>

  <GlassPanel tag="section" radius={20} padding="14px 18px" class="history" aria-labelledby="hist">
    <h2 id="hist">History</h2>
    {#if data.journal.length}
      <ul>
        {#each data.journal.slice(0, 200) as j, i (i)}
          <li>
            <span class="h-text">
              <span class="h-title">{j.title}</span>
              <span class="h-sub">{actionWords(j.action)}, {when(j.ts)}</span>
            </span>
            <span class="h-out">
              {#if j.outcome.status === 'done'}
                {j.action.type === 'quarantine' && j.freed === 0 ? 'In quarantine' : `Freed ${formatBytes(j.freed)}`}
              {:else if j.outcome.status === 'skipped'}
                Skipped
              {:else}
                Failed
              {/if}
            </span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">Nothing has been reclaimed yet.</p>
    {/if}
  </GlassPanel>
</div>

<style>
  .cols {
    flex: 1;
    display: flex;
    gap: 14px;
    min-height: 0;
  }
  .left {
    flex: 3;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    min-height: 0;
  }
  :global(.sum) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .num {
    font-family: var(--k-font-display);
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--k-text-3);
    max-width: 52ch;
  }
  :global(.batches) {
    overflow: auto;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .batch + .batch {
    border-top: 1px solid var(--k-line);
  }
  .b-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 52px;
  }
  .b-toggle {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    border: 0;
    background: transparent;
    padding: 0;
    text-align: left;
  }
  .b-when {
    font-size: 13px;
    font-weight: 600;
  }
  .b-what {
    font-size: 11px;
    color: var(--k-text-4);
  }
  .b-size {
    font-size: 13px;
    color: var(--k-text-2);
  }
  .entries {
    padding: 0 0 10px 12px;
  }
  .entries li {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    font-size: 12px;
  }
  .e-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--k-text-2);
  }
  .e-size {
    color: var(--k-text-3);
  }
  .link {
    border: 0;
    background: transparent;
    padding: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--k-accent-text);
  }
  :global(.history) {
    flex: 2;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 0;
  }
  :global(.history) ul {
    overflow: auto;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }
  :global(.history) li {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    border-top: 1px solid var(--k-line);
  }
  .h-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .h-title {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .h-sub {
    font-size: 11px;
    color: var(--k-text-4);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .h-out {
    font-size: 12px;
    color: var(--k-text-2);
    white-space: nowrap;
  }
  .empty {
    color: var(--k-text-4);
  }
</style>

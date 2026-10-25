<script lang="ts">
  // Explain sheet (docs/UI.md §3.9): what an item is, why this safety, what happens.
  import type { Explanation } from '@kipple/client';
  import { Button, Dot, SafetyTag, categoryColor, formatAgo, formatBytes, tildePath } from '@kipple/design';
  import { client } from '../lib/client.ts';
  import { actionWords } from '../lib/derive.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';
  import Modal from './Modal.svelte';

  const req = $derived(ui.explain);
  const first = $derived(req?.items[0]);
  const total = $derived(req?.items.reduce((s, i) => s + i.size.reclaimable, 0) ?? 0);
  const ids = $derived(req?.items.filter((i) => i.safety !== 'keep').map((i) => i.id) ?? []);
  const inPlan = $derived(ids.length > 0 && ids.every((id) => planStore.has(id)));

  let explanation: Explanation | null = $state(null);
  $effect(() => {
    const id = first?.id;
    explanation = null;
    if (id) client.explain(id).then((e) => (explanation = e)).catch(() => undefined);
  });

  function location(): string {
    const l = first?.locator;
    if (!l) return '';
    if (l.type === 'path') return tildePath(l.path);
    if (l.type === 'paths') return `${l.paths.length} folders`;
    if (l.type === 'docker') return `Docker ${l.kind} ${l.id}`;
    return `${l.store} ${l.key.slice(0, 12)}`;
  }

  function toggle() {
    if (inPlan) planStore.remove(ids);
    else planStore.add(ids);
  }
</script>

{#if req && first}
  <Modal label="About {req.title}" kind="sheet" width={380} onclose={() => ui.closeExplain()}>
    <div class="head">
      <Dot color={categoryColor(first.category)} />
      <h2>{req.title}</h2>
    </div>
    <div class="meta">
      <span class="size">{formatBytes(total)}</span>
      <SafetyTag level={first.safety} />
    </div>

    <section class="reasons" aria-label="Why">
      {#each first.reasons as r, i (i)}<p>{r.text}</p>{/each}
    </section>

    <dl>
      <dt>Action</dt>
      <dd class="k-mono">{actionWords(first.actions[0])}</dd>
      {#if req.items.length === 1}
        <dt>Where</dt>
        <dd class="k-mono">{location()}</dd>
      {:else}
        <dt>Items</dt>
        <dd>{req.items.length}</dd>
      {/if}
      {#if first.project}
        <dt>Project</dt>
        <dd>{first.project.name}</dd>
      {/if}
      {#if first.last_used}
        <dt>Last used</dt>
        <dd>{formatAgo(first.last_used.at)}</dd>
      {/if}
      {#if first.restore_hint}
        <dt>Comes back with</dt>
        <dd class="k-mono">{first.restore_hint}</dd>
      {/if}
      {#if explanation?.integration}
        <dt>Found by</dt>
        <dd>{explanation.integration.name}{explanation.integration.status === 'experimental' ? ' (experimental)' : ''}</dd>
      {/if}
    </dl>

    {#each first.notes ?? [] as note, i (i)}<p class="note">{note}</p>{/each}

    {#if req.items.length > 1}
      <ul class="list" aria-label="Items">
        {#each req.items.slice(0, 50) as it (it.id)}
          <li><span class="t">{it.project?.name ?? it.title}</span><span class="s">{formatBytes(it.size.reclaimable)}</span></li>
        {/each}
        {#if req.items.length > 50}<li class="more">and {req.items.length - 50} more</li>{/if}
      </ul>
    {/if}

    <div class="actions">
      {#if first.safety === 'keep'}
        <p class="note">kipple keeps this. It can't be added to a plan.</p>
      {:else}
        <Button variant={inPlan ? 'clear' : 'tint'} size="md" block onclick={toggle}>
          {inPlan ? 'Remove from plan' : `Add ${formatBytes(total)} to plan`}
        </Button>
      {/if}
      <Button variant="clear" size="md" block onclick={() => ui.closeExplain()}>Close</Button>
    </div>
  </Modal>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.01em;
    overflow-wrap: anywhere;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .size {
    font-family: var(--k-font-display);
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .reasons p {
    margin: 0 0 6px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--k-text-2);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 14px;
    font-size: 12px;
  }
  dt {
    color: var(--k-text-4);
  }
  dd {
    margin: 0;
    color: var(--k-text-2);
    overflow-wrap: anywhere;
  }
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--k-text-3);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 12px;
  }
  .list li {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    height: 28px;
    align-items: center;
    border-top: 1px solid var(--k-line);
    color: var(--k-text-2);
  }
  .list .t {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .more {
    color: var(--k-text-4);
  }
  .actions {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>

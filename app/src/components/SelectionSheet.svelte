<script lang="ts">
  // Edit selection: every item by category with checkboxes. Keep items are locked.
  import type { Category, Item } from '@kipple/client';
  import { Button, Dot, SafetyTag, categoryColor, categoryLabel, formatBytes } from '@kipple/design';
  import { groupItems } from '../lib/derive.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';
  import Modal from './Modal.svelte';

  let draft = $state(new Set<string>());
  let open = $state<Record<string, boolean>>({});

  $effect(() => {
    if (ui.selectionOpen) draft = new Set(planStore.selection ?? []);
  });

  const byCategory = $derived.by(() => {
    const map = new Map<Category, Item[]>();
    for (const i of data.items) map.set(i.category, [...(map.get(i.category) ?? []), i]);
    return [...map.entries()]
      .map(([category, items]) => ({ category, groups: groupItems(items), bytes: items.reduce((s, i) => s + i.size.reclaimable, 0) }))
      .sort((a, b) => b.bytes - a.bytes);
  });

  const draftBytes = $derived(data.items.filter((i) => draft.has(i.id)).reduce((s, i) => s + i.size.reclaimable, 0));

  function groupState(items: Item[]): 'all' | 'some' | 'none' {
    const n = items.filter((i) => draft.has(i.id)).length;
    return n === 0 ? 'none' : n === items.length ? 'all' : 'some';
  }

  function toggleGroup(items: Item[]) {
    const pickable = items.filter((i) => i.safety !== 'keep');
    const next = new Set(draft);
    const all = pickable.every((i) => next.has(i.id));
    pickable.forEach((i) => (all ? next.delete(i.id) : next.add(i.id)));
    draft = next;
  }

  function apply() {
    planStore.set(draft);
    ui.selectionOpen = false;
  }
</script>

{#if ui.selectionOpen}
  <Modal label="Edit selection" kind="sheet" width={460} onclose={() => (ui.selectionOpen = false)}>
    <div class="head">
      <h2>Edit selection</h2>
      <span class="sum">{formatBytes(draftBytes)} selected</span>
    </div>
    <div class="cats">
      {#each byCategory as cat (cat.category)}
        <section>
          <button
            type="button"
            class="cat"
            aria-expanded={!!open[cat.category]}
            onclick={() => (open[cat.category] = !open[cat.category])}
          >
            <Dot color={categoryColor(cat.category)} />
            <span class="t">{categoryLabel(cat.category)}</span>
            <span class="s">{formatBytes(cat.bytes)}</span>
          </button>
          {#if open[cat.category]}
            <ul>
              {#each cat.groups as g (g.key)}
                {@const st = groupState(g.items)}
                {@const locked = g.safety === 'keep'}
                <li>
                  <label class:locked>
                    <input
                      type="checkbox"
                      checked={st === 'all'}
                      indeterminate={st === 'some'}
                      disabled={locked}
                      onchange={() => toggleGroup(g.items)}
                    />
                    <span class="t">{g.title}</span>
                    <SafetyTag level={g.safety} />
                    <span class="s">{formatBytes(g.bytes)}</span>
                  </label>
                  {#if locked}<p class="why">{g.items[0]?.reasons[0]?.text}</p>{/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/each}
    </div>
    <div class="actions">
      <Button variant="clear" size="md" onclick={() => (draft = new Set(data.items.filter((i) => i.safety === 'safe').map((i) => i.id)))}>
        Only safe items
      </Button>
      <Button variant="tint" size="md" onclick={apply}>Use this selection</Button>
    </div>
  </Modal>
{/if}

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
  }
  .sum {
    font-size: 13px;
    color: var(--k-text-3);
  }
  .cats {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .cat {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    border: 0;
    border-top: 1px solid var(--k-line);
    background: transparent;
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    padding: 0;
  }
  ul {
    list-style: none;
    margin: 0 0 8px;
    padding: 0 0 0 18px;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 34px;
    font-size: 12px;
    cursor: pointer;
  }
  label.locked {
    cursor: default;
    opacity: 0.8;
  }
  input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--k-accent);
  }
  .t {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .s {
    width: 64px;
    text-align: right;
    color: var(--k-text-2);
  }
  .why {
    margin: -4px 0 6px 24px;
    font-size: 11px;
    color: var(--k-text-4);
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
</style>

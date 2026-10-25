// The review plan: which items are selected, the plan the core makes from them,
// and running it (SPEC §8).

import { KippleError, type CategorySummary, type Confirmation, type ExecReport, type Plan } from '@kipple/client';
import { formatBytes } from '@kipple/design';
import { client } from '../client.ts';
import { data } from './data.svelte.ts';
import { ui } from './ui.svelte.ts';

export type RunPhase = 'idle' | 'running' | 'done';

const SAVED_KEY = 'kipple.savedSelection';

class PlanStore {
  /** Selected item ids. `null` until the first load picks the safe items. */
  selection: Set<string> | null = $state.raw(null);
  /** True while the selection is still the one kipple picked. */
  pickedByKipple = $state(true);
  plan: Plan | null = $state.raw(null);
  building = $state(false);
  skipQuarantine = $state(false);

  phase: RunPhase = $state('idle');
  progress = $state({ index: 0, total: 0, title: '' });
  result: ExecReport | null = $state.raw(null);
  /** Categories as they were before the last run, for the before and after stacks. */
  before: { categories: CategorySummary[]; total: number } | null = $state.raw(null);
  private abort: AbortController | null = null;
  private buildSeq = 0;

  /** Reclaimable bytes of the current plan (badge in the sidebar). */
  bytes: number = $derived(this.plan?.totals.reclaimable ?? 0);
  hasReview: boolean = $derived(!!this.plan?.entries.some((e) => e.item.safety === 'review'));

  /** Picks every Safe item, like `kipple clean --safe-only`. */
  pickSafe() {
    this.selection = new Set(data.items.filter((i) => i.safety === 'safe').map((i) => i.id));
    this.pickedByKipple = true;
    void this.rebuild();
  }

  ensure() {
    if (this.selection === null && data.status === 'ready') this.pickSafe();
  }

  add(ids: string[]) {
    const next = new Set(this.selection ?? []);
    let changed = false;
    for (const id of ids) {
      if (!next.has(id)) {
        next.add(id);
        changed = true;
      }
    }
    if (!changed) return;
    this.selection = next;
    this.pickedByKipple = false;
    void this.rebuild();
  }

  remove(ids: string[]) {
    const next = new Set(this.selection ?? []);
    ids.forEach((id) => next.delete(id));
    this.selection = next;
    this.pickedByKipple = false;
    void this.rebuild();
  }

  set(ids: Iterable<string>) {
    this.selection = new Set(ids);
    this.pickedByKipple = false;
    void this.rebuild();
  }

  has(id: string): boolean {
    return this.selection?.has(id) ?? false;
  }

  async rebuild() {
    const seq = ++this.buildSeq;
    const ids = [...(this.selection ?? [])].filter((id) => data.itemsById.has(id));
    if (!ids.length) {
      this.plan = null;
      return;
    }
    this.building = true;
    try {
      const plan = await client.plan({ filter: { ids } });
      if (seq === this.buildSeq) this.plan = plan;
    } catch (err) {
      ui.toast(KippleError.from(err).message);
    } finally {
      if (seq === this.buildSeq) this.building = false;
    }
  }

  save() {
    try {
      localStorage.setItem(SAVED_KEY, JSON.stringify([...(this.selection ?? [])]));
      ui.toast('Plan saved. Open Review plan to run it later.');
    } catch {
      ui.toast('This plan could not be saved here.');
    }
  }

  restoreSaved(): boolean {
    try {
      const raw = localStorage.getItem(SAVED_KEY);
      if (!raw) return false;
      const ids = (JSON.parse(raw) as string[]).filter((id) => data.itemsById.has(id));
      if (!ids.length) return false;
      this.set(ids);
      return true;
    } catch {
      return false;
    }
  }

  /** Runs the review plan after the right confirmation (SPEC §8.2). */
  async reclaim() {
    const plan = this.plan;
    if (!plan || this.phase === 'running') return;
    const label = `Reclaim ${formatBytes(plan.totals.reclaimable)}`;
    let confirm: Confirmation | null;
    if (this.hasReview || this.skipQuarantine) {
      confirm = await ui.askConfirm(plan, label, this.skipQuarantine);
    } else {
      confirm = { kind: 'click', purge_now: false };
    }
    if (!confirm) return;
    const before = { categories: data.categories, total: data.total };
    const report = await this.execute(plan, confirm);
    if (report) {
      this.before = before;
      this.result = report;
      this.phase = 'done';
      this.selection = new Set();
      this.plan = null;
    }
  }

  /** Runs any plan (the review plan or an ad-hoc one from AI history). */
  async execute(plan: Plan, confirm: Confirmation): Promise<ExecReport | null> {
    this.phase = 'running';
    this.progress = { index: 0, total: plan.entries.length, title: '' };
    this.abort = new AbortController();
    try {
      const report = await client.execute(plan, confirm, {
        signal: this.abort.signal,
        onEvent: (e) => {
          if (e.event === 'entry_started') this.progress = { index: e.index + 1, total: plan.entries.length, title: e.title };
        },
      });
      await data.refresh();
      return report;
    } catch (err) {
      ui.toast(KippleError.from(err).message);
      return null;
    } finally {
      this.abort = null;
      if (this.phase === 'running') this.phase = 'idle';
    }
  }

  /** Runs an ad-hoc plan for the given items with the dialog when needed. */
  async runItems(ids: string[], label: string): Promise<ExecReport | null> {
    const plan = await client.plan({ filter: { ids } });
    const review = plan.entries.some((e) => e.item.safety === 'review');
    const confirm = review ? await ui.askConfirm(plan, label) : ({ kind: 'click' } as Confirmation);
    if (!confirm) return null;
    const report = await this.execute(plan, confirm);
    if (report) {
      const freed = report.freed_now + report.held_in_quarantine;
      ui.toast(
        report.held_in_quarantine > 0
          ? `Done. ${formatBytes(report.freed_now)} is free now, ${formatBytes(report.held_in_quarantine)} is in quarantine.`
          : `Done. ${formatBytes(freed)} is free now.`,
      );
      if (this.selection) void this.rebuild();
    }
    return report;
  }

  stop() {
    this.abort?.abort();
  }

  dismissResult() {
    this.phase = 'idle';
    this.result = null;
    this.pickSafe();
  }
}

export const planStore = new PlanStore();

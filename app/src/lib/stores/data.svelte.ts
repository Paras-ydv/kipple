// App data loaded from the core through KippleClient.

import {
  isUnavailable,
  KippleError,
  type Category,
  type CategorySummary,
  type IntegrationStatusReport,
  type Item,
  type JournalEntry,
  type ProjectSummary,
  type QuarantineEntry,
  type Report,
  type SizePoint,
  type VolumeInfo,
} from '@kipple/client';
import { client } from '../client.ts';

/** `empty`: the core works but nothing has been scanned yet. */
export type LoadStatus = 'loading' | 'ready' | 'empty' | 'unavailable' | 'error';

class DataStore {
  status: LoadStatus = $state('loading');
  error: string | null = $state(null);
  report: Report | null = $state.raw(null);
  items: Item[] = $state.raw([]);
  projects: ProjectSummary[] = $state.raw([]);
  volume: VolumeInfo | null = $state.raw(null);
  history: SizePoint[] = $state.raw([]);
  quarantine: QuarantineEntry[] = $state.raw([]);
  journal: JournalEntry[] = $state.raw([]);
  integrations: IntegrationStatusReport[] = $state.raw([]);

  /** Scan in progress: files seen and bytes found per category so far. */
  scanning = $state(false);
  scanFiles = $state(0);
  scanFound: Partial<Record<Category, number>> = $state({});
  private scanAbort: AbortController | null = null;

  /** Categories with bytes, largest first (live during a scan). */
  categories: CategorySummary[] = $derived.by(() => {
    if (this.scanning) {
      return Object.entries(this.scanFound)
        .map(([category, reclaimable]) => ({
          category: category as Category,
          reclaimable: reclaimable ?? 0,
          items: 0,
          by_safety: { safe: 0, review: 0, keep: 0 },
        }))
        .sort((a, b) => b.reclaimable - a.reclaimable);
    }
    return (this.report?.summary.categories ?? []).filter((c) => c.reclaimable > 0);
  });

  total: number = $derived(
    this.scanning
      ? Object.values(this.scanFound).reduce((s: number, b) => s + (b ?? 0), 0)
      : (this.report?.summary.reclaimable ?? 0),
  );

  itemsById: Map<string, Item> = $derived(new Map(this.items.map((i) => [i.id, i])));

  async load() {
    try {
      const [report, items, projects, volume, history, quarantine, journal, integrations] = await Promise.all([
        client.report(),
        client.items({}),
        client.projects({}),
        client.volume(),
        client.history({}),
        client.quarantine(),
        client.journal(),
        client.integrations(),
      ]);
      this.report = report;
      this.items = items;
      this.projects = projects;
      this.volume = volume;
      this.history = history;
      this.quarantine = quarantine;
      this.journal = journal;
      this.integrations = integrations;
      this.status = 'ready';
      this.error = null;
    } catch (err) {
      this.fail(err);
    }
  }

  /** Reloads what an action can change. */
  async refresh() {
    try {
      const [report, items, projects, volume, quarantine, journal] = await Promise.all([
        client.report(),
        client.items({}),
        client.projects({}),
        client.volume(),
        client.quarantine(),
        client.journal(),
      ]);
      this.report = report;
      this.items = items;
      this.projects = projects;
      this.volume = volume;
      this.quarantine = quarantine;
      this.journal = journal;
      if (this.status !== 'ready') {
        // First scan finished: load the rest too.
        await this.load();
      }
    } catch (err) {
      this.fail(err);
    }
  }

  async scan() {
    if (this.scanning) return;
    this.scanning = true;
    this.scanFiles = 0;
    this.scanFound = {};
    this.scanAbort = new AbortController();
    try {
      await client.scan(
        {},
        {
          signal: this.scanAbort.signal,
          onEvent: (e) => {
            if (e.event === 'item_found' && e.item.safety !== 'keep') {
              this.scanFound = {
                ...this.scanFound,
                [e.item.category]: (this.scanFound[e.item.category] ?? 0) + e.item.size.reclaimable,
              };
            } else if (e.event === 'progress') {
              this.scanFiles = e.done;
            }
          },
        },
      );
      this.scanning = false;
      await this.refresh();
    } catch (err) {
      this.scanning = false;
      this.fail(err);
    } finally {
      this.scanAbort = null;
    }
  }

  cancelScan() {
    this.scanAbort?.abort();
  }

  private fail(err: unknown) {
    if (isUnavailable(err)) {
      this.status = 'unavailable';
      return;
    }
    if (KippleError.from(err).code === 'no_scan') {
      this.status = 'empty';
      this.error = null;
      return;
    }
    this.status = this.report ? 'ready' : 'error';
    this.error = KippleError.from(err).message;
  }
}

export const data = new DataStore();

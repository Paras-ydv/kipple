// MockClient: an in-memory kipple core over fixture data, for the browser and tests.
// Behaves like the real API: plans are pure, execute revalidates, quarantine can be
// restored, and progress arrives as events. Nothing touches the file system.

import { KippleError, type KippleClient, type StreamOptions } from '../client.ts';
import type {
  ActionKind,
  Category,
  CategorySummary,
  Check,
  Confirmation,
  DoctorReport,
  ExecEntry,
  ExecEvent,
  ExecReport,
  Explanation,
  HistoryRequest,
  IntegrationStatusReport,
  Item,
  ItemId,
  ItemQuery,
  JournalEntry,
  Plan,
  PlanRequest,
  ProjectQuery,
  ProjectSummary,
  PurgeReport,
  PurgeRequest,
  QuarantineEntry,
  QuarantineId,
  Report,
  RestoreReport,
  SafetyMix,
  ScanEvent,
  ScanRequest,
  ScanResult,
  SizePoint,
  Summary,
  VolumeInfo,
  KippleConfig,
} from '../types.ts';
import { CATALOG, integrationInfo } from './catalog.ts';
import { buildFixtures, type Fixtures } from './fixtures.ts';
import { DAY, iso, ulid } from './util.ts';

export { buildFixtures, HOME } from './fixtures.ts';

export interface MockOptions {
  /** Delay per streamed step in ms (scan ticks, executed entries). */
  stepMs?: number;
  /** Fixed clock for tests. */
  now?: number;
  /** Quarantine window (SPEC §8.4). */
  keepDays?: number;
}

const KIPPLE_VERSION = '0.1.0';
const CATEGORY_ORDER: Category[] = [
  'container', 'ai-history', 'package-cache', 'project-artifact', 'mobile', 'model',
  'toolchain', 'build-cache', 'ide-cache', 'logs', 'system',
];

const sleep = (ms: number, signal?: AbortSignal) =>
  new Promise<void>((resolve) => {
    if (ms <= 0) return resolve();
    const t = setTimeout(resolve, ms);
    signal?.addEventListener('abort', () => {
      clearTimeout(t);
      resolve();
    }, { once: true });
  });

const emptyMix = (): SafetyMix => ({ safe: 0, review: 0, keep: 0 });

/** Bytes a plan entry frees: compaction frees the free pages, not the whole file. */
export function entryBytes(item: Item, action: ActionKind): number {
  if (action.type === 'compact') {
    const free = (item.details as { free_pages_bytes?: unknown } | undefined)?.free_pages_bytes;
    return typeof free === 'number' ? free : 0;
  }
  return item.size.reclaimable;
}

const isQuarantine = (a: ActionKind) => a.type === 'quarantine';

export class MockClient implements KippleClient {
  readonly kind = 'mock' as const;
  private fx: Fixtures;
  private inventory: Map<ItemId, Item>;
  private quarantined: QuarantineEntry[] = [];
  private journalEntries: JournalEntry[];
  private plans = new Map<string, Plan>();
  private stepMs: number;
  private keepDays: number;

  constructor(opts: MockOptions = {}) {
    this.fx = buildFixtures(opts.now ?? Date.now());
    this.inventory = new Map(this.fx.items.map((i) => [i.id, i]));
    this.journalEntries = [...this.fx.journal];
    this.stepMs = opts.stepMs ?? 140;
    this.keepDays = opts.keepDays ?? 7;
  }

  private get now(): number {
    return Date.now();
  }

  private live(): Item[] {
    return [...this.inventory.values()];
  }

  // ---- Read side ------------------------------------------------------------

  async integrations(): Promise<IntegrationStatusReport[]> {
    const found = new Set(this.fx.items.map((i) => i.integration));
    return Object.keys(CATALOG).map((id) => {
      const info = integrationInfo(id);
      return {
        ...info,
        enabled: true,
        detection: found.has(id)
          ? { state: 'present', version: CATALOG[id]?.version ?? null }
          : { state: 'not_present' },
      };
    });
  }

  async items(query: ItemQuery = {}): Promise<Item[]> {
    return this.filter(this.live(), query).sort((a, b) => b.size.reclaimable - a.size.reclaimable);
  }

  async projects(query: ProjectQuery = {}): Promise<ProjectSummary[]> {
    const items = this.live();
    const now = this.now;
    let list = this.fx.projects.map((p) => {
      const mine = items.filter((i) => i.project?.id === p.ref.id && i.safety !== 'keep');
      const by_category: ProjectSummary['by_category'] = {};
      const mix = emptyMix();
      let total = 0;
      let apparent = 0;
      for (const i of mine) {
        by_category[i.category] = (by_category[i.category] ?? 0) + i.size.reclaimable;
        mix[i.safety] += i.size.reclaimable;
        total += i.size.reclaimable;
        apparent += i.size.apparent;
      }
      const summary: ProjectSummary = {
        id: p.ref.id,
        name: p.ref.name,
        root: p.ref.root,
        markers: p.markers,
        exists: p.exists,
        item_count: mine.length,
        last_used: { at: iso(now - p.lastDays * DAY), source: 'git_commit' },
        total: { apparent, on_disk: total, reclaimable: total, file_count: null },
        by_category,
        safety_mix: mix,
      };
      return summary;
    });
    list = list.filter((p) => p.total.reclaimable > 0);
    if (query.search) {
      const q = query.search.toLowerCase();
      list = list.filter((p) => p.name.toLowerCase().includes(q) || p.root.toLowerCase().includes(q));
    }
    if (query.orphaned) list = list.filter((p) => !p.exists);
    if (query.older_than_secs) {
      const cutoff = now - query.older_than_secs * 1000;
      list = list.filter((p) => p.last_used && Date.parse(p.last_used.at) <= cutoff);
    }
    list.sort((a, b) => b.total.reclaimable - a.total.reclaimable);
    return query.limit ? list.slice(0, query.limit) : list;
  }

  async explain(id: ItemId): Promise<Explanation> {
    const item = this.inventory.get(id) ?? this.quarantined.find((q) => q.item.id === id)?.item;
    if (!item) throw new KippleError('not_found', 'That item is gone. Scan again to refresh the list.');
    return {
      item,
      integration: integrationInfo(item.integration),
      size_differs: Math.abs(item.size.apparent - item.size.reclaimable) > item.size.reclaimable * 0.1,
      actions: item.actions.map((action) => ({ action, ...describeAction(action, this.keepDays) })),
    };
  }

  async report(): Promise<Report> {
    const summary = this.summary();
    const safeQuarantine = this.live()
      .filter((i) => i.safety === 'safe')
      .reduce((s, i) => s + (i.actions[0] && isQuarantine(i.actions[0]) ? i.size.reclaimable : 0), 0);
    return {
      scan: this.scanInfo(),
      summary,
      top_projects: await this.projects({ limit: 10 }),
      next: {
        command: 'kipple clean --safe-only',
        reclaimable: summary.by_safety.safe,
        undoable: safeQuarantine,
        keep_days: this.keepDays,
      },
    };
  }

  async history(req: HistoryRequest = {}): Promise<SizePoint[]> {
    return this.fx.history.filter(
      (p) =>
        (!req.scope || p.scope === req.scope) &&
        (!req.key || p.key === req.key) &&
        (!req.since || Date.parse(p.ts) >= Date.parse(req.since)),
    );
  }

  async doctor(): Promise<DoctorReport> {
    const integrations = await this.integrations();
    return {
      config_exists: true,
      config_path: '/Users/you/Library/Application Support/dev.kipple.kipple/config.toml',
      data_dir: '/Users/you/Library/Application Support/dev.kipple.kipple',
      platform: { os: 'macos', arch: 'aarch64', trash_available: true, wsl: false },
      warnings: [],
      integrations: integrations.map((i) => ({
        info: integrationInfo(i.id),
        enabled: i.enabled,
        detection: i.detection,
        fixes: [],
        locations: [],
        tools: [],
      })),
    };
  }

  async volume(): Promise<VolumeInfo> {
    const freedSinceStart = this.fx.items.reduce((s, i) => s + i.size.reclaimable, 0) -
      this.live().reduce((s, i) => s + i.size.reclaimable, 0) -
      this.quarantined.reduce((s, q) => s + q.size.reclaimable, 0);
    return { ...this.fx.volume, free: this.fx.volume.free + Math.max(0, freedSinceStart) };
  }

  async journal(): Promise<JournalEntry[]> {
    return [...this.journalEntries];
  }

  private cfg: KippleConfig = {
    scan: { roots: ['~'], exclude: [], include_network: false, follow_symlinks: false },
    safety: { recent_days: 14, protect: [], override: [] },
    quarantine: { keep_days: 7 },
    integrations: { disable: [], enable: [] },
  };

  async config(): Promise<KippleConfig> {
    return structuredClone(this.cfg);
  }

  async setConfig(config: KippleConfig): Promise<KippleConfig> {
    if (config.quarantine.keep_days < 1 || config.safety.recent_days < 1) {
      throw new KippleError('config', 'Days must be at least 1.');
    }
    this.cfg = structuredClone(config);
    return structuredClone(this.cfg);
  }

  async quarantine(): Promise<QuarantineEntry[]> {
    return [...this.quarantined].sort((a, b) => b.created_at.localeCompare(a.created_at));
  }

  // ---- Scan -------------------------------------------------------------------

  async scan(req: ScanRequest = {}, opts: StreamOptions<ScanEvent> = {}): Promise<ScanResult> {
    const emit = opts.onEvent ?? (() => undefined);
    const roots = req.roots?.length ? req.roots : ['/Users/you'];
    emit({ event: 'started', roots });
    const items = this.filter(this.live(), { only: req.only ?? [], min_size: req.min_size ?? null });
    // Discover by category, largest first, so the stack builds bottom-up.
    const byCat = new Map<Category, Item[]>();
    for (const i of items) byCat.set(i.category, [...(byCat.get(i.category) ?? []), i]);
    const order = [...byCat.entries()].sort(
      (a, b) => b[1].reduce((s, i) => s + i.size.reclaimable, 0) - a[1].reduce((s, i) => s + i.size.reclaimable, 0),
    );
    const totalEntries = this.fx.scan.entries;
    let done = 0;
    let bytes = 0;
    const started = this.now;
    for (const [, list] of order) {
      if (opts.signal?.aborted) break;
      for (const item of list) {
        bytes += item.size.reclaimable;
        emit({ event: 'item_found', item });
      }
      done += Math.round(totalEntries / order.length);
      emit({ event: 'progress', done: Math.min(done, totalEntries), total: null, bytes });
      await sleep(this.stepMs * 2, opts.signal);
    }
    const cancelled = !!opts.signal?.aborted;
    this.fx.scan = {
      id: ulid(this.now, `scan${this.now}`),
      startedAt: started,
      finishedAt: this.now,
      entries: cancelled ? done : totalEntries,
    };
    const result: ScanResult = { scan: this.scanInfo(cancelled ? 'cancelled' : 'complete'), summary: this.summary(), warnings: [] };
    emit({ event: 'finished', result });
    return result;
  }

  // ---- Plan and execute ---------------------------------------------------------

  async plan(req: PlanRequest): Promise<Plan> {
    const filter = req.filter ?? {};
    const exclude = new Set(req.exclude_ids ?? []);
    const items = this.filter(this.live(), filter).filter((i) => i.safety !== 'keep' && !exclude.has(i.id));
    const warnings: string[] = [];
    const entries = items
      .map((item) => {
        const action = req.action_overrides?.[item.id] ?? item.actions[0];
        if (!action) {
          warnings.push(`${item.title} has no allowed action and was left out.`);
          return null;
        }
        return { item, action, preflight: preflight(item) };
      })
      .filter((e): e is NonNullable<typeof e> => e !== null)
      // Safe first, then Review (SPEC §8.1); largest first inside each.
      .sort((a, b) => (a.item.safety === b.item.safety ? b.item.size.reclaimable - a.item.size.reclaimable : a.item.safety === 'safe' ? -1 : 1));

    const totals = {
      entries: entries.length,
      reclaimable: 0,
      freed_now: 0,
      held_in_quarantine: 0,
      by_safety: emptyMix(),
      by_action: {} as Record<string, number>,
    };
    for (const e of entries) {
      const b = entryBytes(e.item, e.action);
      totals.reclaimable += b;
      totals.by_safety[e.item.safety] += b;
      totals.by_action[e.action.type] = (totals.by_action[e.action.type] ?? 0) + b;
      if (isQuarantine(e.action)) totals.held_in_quarantine += b;
      else totals.freed_now += b;
    }
    const plan: Plan = {
      id: ulid(this.now, `plan${this.plans.size}`),
      created_at: iso(this.now),
      scan_id: this.fx.scan.id,
      entries,
      totals,
      warnings,
    };
    this.plans.set(plan.id, plan);
    return plan;
  }

  async execute(plan: Plan, confirm: Confirmation, opts: StreamOptions<ExecEvent> = {}): Promise<ExecReport> {
    const hasReview = plan.entries.some((e) => e.item.safety === 'review');
    const purgeNow = !!confirm.purge_now;
    const typedOk = confirm.kind === 'typed' && confirm.text.trim() === 'reclaim';
    if ((hasReview || purgeNow) && !typedOk) {
      throw new KippleError('confirmation', 'Type reclaim to run a plan with Review items or without quarantine.');
    }
    if (!this.plans.has(plan.id)) {
      throw new KippleError('stale_plan', 'This plan is out of date. Make it again.');
    }
    const emit = opts.onEvent ?? (() => undefined);
    emit({ event: 'started', plan_id: plan.id, entries: plan.entries.length });
    const report: ExecReport = {
      plan_id: plan.id,
      entries: [],
      done: 0,
      skipped: 0,
      failed: 0,
      freed_now: 0,
      held_in_quarantine: 0,
      cancelled: false,
    };
    const created = this.now;
    for (const [index, entry] of plan.entries.entries()) {
      if (opts.signal?.aborted) {
        report.cancelled = true;
        break;
      }
      emit({ event: 'entry_started', index, title: entry.item.title });
      await sleep(this.stepMs, undefined);
      const current = this.inventory.get(entry.item.id);
      let result: ExecEntry;
      if (!current) {
        result = { item_id: entry.item.id, title: entry.item.title, action: entry.action, outcome: { status: 'skipped', reason: 'It was already gone.' } };
        report.skipped++;
      } else {
        const bytes = entryBytes(current, entry.action);
        let quarantineId: QuarantineId | null = null;
        if (isQuarantine(entry.action) && !purgeNow) {
          quarantineId = ulid(created + index, `q${current.id}`);
          this.quarantined.push({
            id: quarantineId,
            plan_id: plan.id,
            created_at: iso(created),
            expires_at: iso(created + this.keepDays * DAY),
            integration: current.integration,
            item: current,
            original_paths: current.locator.type === 'path' ? [current.locator.path] : current.locator.type === 'paths' ? current.locator.paths : [],
            size: current.size,
          });
          report.held_in_quarantine += bytes;
        } else {
          report.freed_now += bytes;
        }
        if (entry.action.type === 'compact') {
          const rest = current.size.reclaimable - bytes;
          this.inventory.set(current.id, {
            ...current,
            size: { ...current.size, reclaimable: rest, on_disk: rest },
            details: { ...(current.details ?? {}), free_pages_bytes: 0 },
            actions: [],
            safety: 'keep',
            reasons: [{ code: 'CHAT_HISTORY', text: 'Compacted. What is left is chat data that only Cursor can remove.' }],
          });
        } else {
          this.inventory.delete(current.id);
        }
        result = { item_id: current.id, title: current.title, action: entry.action, outcome: { status: 'done', freed: bytes }, quarantine_id: quarantineId };
        report.done++;
        this.journalEntries.unshift({
          ts: iso(this.now),
          plan_id: plan.id,
          item_id: current.id,
          integration: current.integration,
          title: current.title,
          action: entry.action,
          outcome: result.outcome,
          freed: isQuarantine(entry.action) && !purgeNow ? 0 : bytes,
          original_path: current.locator.type === 'path' ? current.locator.path : null,
          quarantine_id: quarantineId,
        });
      }
      report.entries.push(result);
      emit({ event: 'entry_finished', index, entry: result });
    }
    this.plans.delete(plan.id);
    emit({ event: 'finished' });
    return report;
  }

  async restore(id: QuarantineId): Promise<RestoreReport> {
    const idx = this.quarantined.findIndex((q) => q.id === id);
    if (idx < 0) return { restored: [], skipped: [{ id, reason: 'Nothing in quarantine has that id.' }] };
    const [entry] = this.quarantined.splice(idx, 1);
    if (!entry) return { restored: [], skipped: [] };
    this.inventory.set(entry.item.id, entry.item);
    return { restored: [{ id, title: entry.item.title, paths: entry.original_paths }], skipped: [] };
  }

  async purge(req: PurgeRequest): Promise<PurgeReport> {
    const ids = new Set(req.ids ?? []);
    const cutoff = req.older_than_secs ? this.now - req.older_than_secs * 1000 : Infinity;
    const hit = this.quarantined.filter(
      (q) => (ids.size === 0 || ids.has(q.id)) && Date.parse(q.created_at) <= cutoff,
    );
    this.quarantined = this.quarantined.filter((q) => !hit.includes(q));
    return { purged: hit.map((q) => q.id), freed: hit.reduce((s, q) => s + q.size.reclaimable, 0), errors: [] };
  }

  // ---- Helpers -------------------------------------------------------------------

  private filter(items: Item[], f: ItemQuery): Item[] {
    const ids = f.ids?.length ? f.ids : null;
    const only = f.only?.length ? new Set(f.only) : null;
    const levels = f.safe_only ? new Set(['safe']) : f.safety?.length ? new Set<string>(f.safety) : null;
    const cutoff = f.older_than_secs ? this.now - f.older_than_secs * 1000 : null;
    return items.filter(
      (i) =>
        (!ids || ids.some((id) => i.id.startsWith(id))) &&
        (!only || only.has(i.integration) || only.has(i.category)) &&
        (!levels || levels.has(i.safety)) &&
        (!f.project || i.project?.id === f.project) &&
        (!f.min_size || i.size.reclaimable >= f.min_size) &&
        (!cutoff || !i.last_used || Date.parse(i.last_used.at) <= cutoff),
    );
  }

  private summary(): Summary {
    const items = this.live();
    const cats = new Map<Category, CategorySummary>();
    const mix = emptyMix();
    for (const i of items) {
      const c = cats.get(i.category) ?? { category: i.category, items: 0, reclaimable: 0, by_safety: emptyMix() };
      c.items++;
      c.by_safety[i.safety] += i.size.reclaimable;
      if (i.safety !== 'keep') c.reclaimable += i.size.reclaimable;
      cats.set(i.category, c);
      mix[i.safety] += i.size.reclaimable;
    }
    const categories = [...cats.values()].sort(
      (a, b) => b.reclaimable - a.reclaimable || CATEGORY_ORDER.indexOf(a.category) - CATEGORY_ORDER.indexOf(b.category),
    );
    return {
      categories,
      items: items.length,
      by_safety: mix,
      reclaimable: mix.safe + mix.review,
      quarantine_bytes: this.quarantined.reduce((s, q) => s + q.size.reclaimable, 0),
    };
  }

  private scanInfo(status: 'complete' | 'cancelled' = 'complete') {
    return {
      id: this.fx.scan.id,
      started_at: iso(this.fx.scan.startedAt),
      finished_at: iso(this.fx.scan.finishedAt),
      duration_ms: this.fx.scan.finishedAt - this.fx.scan.startedAt,
      entries_seen: this.fx.scan.entries,
      kipple_version: KIPPLE_VERSION,
      roots: ['/Users/you'],
      status,
    };
  }
}

function preflight(item: Item): Check[] {
  const checks: Check[] = [];
  if (item.integration === 'docker') checks.push({ id: 'docker.running', ok: true, message: 'Docker is running.' });
  if (item.integration === 'codex') checks.push({ id: 'codex.closed', ok: true, message: 'Codex is not running.' });
  if (item.integration === 'cursor') checks.push({ id: 'cursor.closed', ok: true, message: 'Cursor is closed.' });
  if (item.locator.type === 'path') checks.push({ id: 'same.volume', ok: true, message: 'On the same disk as the quarantine.' });
  return checks;
}

function describeAction(action: ActionKind, keepDays: number): { description: string; undoable: boolean } {
  switch (action.type) {
    case 'quarantine':
      return { description: `Moved to quarantine. Undo for ${keepDays} days, then it is deleted.`, undoable: true };
    case 'trash':
      return { description: 'Moved to the trash.', undoable: true };
    case 'native':
      return { description: NATIVE[action.command_id] ?? `Removed with ${action.command_id}.`, undoable: false };
    case 'compact':
      return { description: 'Compacted in place. Frees the unused pages.', undoable: false };
    default:
      return { description: 'Not available in this version.', undoable: false };
  }
}

/** Plain words for native commands, as shown in the explain panel. */
export const NATIVE: Record<string, string> = {
  'docker.builder.prune': 'docker builder prune (no undo)',
  'docker.image.remove': 'docker image rm (no undo)',
  'docker.volume.remove': 'docker volume rm (no undo)',
  'docker.container.remove': 'docker container rm (no undo)',
  'npm.cache.clean': 'npm cache clean (no undo)',
  'uv.cache.clean': 'uv cache clean (no undo)',
  'pip.cache.purge': 'pip cache purge (no undo)',
  'brew.cleanup': 'brew cleanup (no undo)',
  'pnpm.store.prune': 'pnpm store prune (no undo)',
  'ollama.rm': 'ollama rm (no undo)',
  'codex.delete': 'codex delete (no undo)',
};

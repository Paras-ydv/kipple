// KippleClient mirrors the core public API (SPEC §3.4) one to one.
// Implementations: MockClient (browser, fixtures) and TauriClient (desktop app).

import type {
  Confirmation,
  DoctorReport,
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
  ScanEvent,
  ScanRequest,
  ScanResult,
  SizePoint,
  VolumeInfo,
} from './types.ts';

export interface StreamOptions<E> {
  /** Progress events (`ScanSink` / `ExecSink`). */
  onEvent?: (event: E) => void;
  /** Cancels like the core's `CancelToken`: the current entry finishes, then it stops. */
  signal?: AbortSignal;
}

export interface KippleClient {
  readonly kind: 'mock' | 'tauri';

  /** `integrations()`: id, name, kind, detected, version. */
  integrations(): Promise<IntegrationStatusReport[]>;
  /** `scan(req, events)`. */
  scan(req: ScanRequest, opts?: StreamOptions<ScanEvent>): Promise<ScanResult>;
  /** `items(query)`: from the latest scan in the index. */
  items(query?: ItemQuery): Promise<Item[]>;
  /** `projects(query)`. */
  projects(query?: ProjectQuery): Promise<ProjectSummary[]>;
  /** `explain(item)`. */
  explain(id: ItemId): Promise<Explanation>;
  /** `plan(req)`: pure, no side effects. */
  plan(req: PlanRequest): Promise<Plan>;
  /** `execute(plan, confirm, events)`. */
  execute(plan: Plan, confirm: Confirmation, opts?: StreamOptions<ExecEvent>): Promise<ExecReport>;
  /** `quarantine()`. */
  quarantine(): Promise<QuarantineEntry[]>;
  /** `restore(entry)`. */
  restore(id: QuarantineId): Promise<RestoreReport>;
  /** `purge(req)`. */
  purge(req: PurgeRequest): Promise<PurgeReport>;
  /** `history(req)`: growth timeline. */
  history(req?: HistoryRequest): Promise<SizePoint[]>;
  /** `doctor()`. */
  doctor(): Promise<DoctorReport>;
  /** Latest scan report (`kipple report`): summary plus heaviest projects. */
  report(): Promise<Report>;

  // App extras, not part of SPEC §3.4. Served by app/src-tauri, not kipple-core.
  /** The volume that holds the home folder. */
  volume(): Promise<VolumeInfo>;
  /** The action journal (SPEC §8.5), newest first. */
  journal(): Promise<JournalEntry[]>;
}

/** Errors from either implementation. `unavailable` means the core isn't wired yet. */
export class KippleError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = 'KippleError';
    this.code = code;
  }

  static from(err: unknown): KippleError {
    if (err instanceof KippleError) return err;
    if (typeof err === 'string') {
      return new KippleError(/not available yet/i.test(err) ? 'unavailable' : 'error', err);
    }
    if (err && typeof err === 'object') {
      const o = err as { code?: unknown; message?: unknown };
      if (typeof o.message === 'string') {
        return new KippleError(typeof o.code === 'string' ? o.code : 'error', o.message);
      }
    }
    return new KippleError('error', String(err));
  }
}

/** True for the "not available yet" error the Tauri commands return before the core is wired. */
export function isUnavailable(err: unknown): boolean {
  return KippleError.from(err).code === 'unavailable';
}

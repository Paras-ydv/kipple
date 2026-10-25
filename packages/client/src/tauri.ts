// TauriClient: calls the Tauri commands in app/src-tauri, named after SPEC §3.4.
// Progress events arrive on a Tauri `Channel`; cancellation calls `cancel`.

import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import { KippleError, type KippleClient, type StreamOptions } from './client.ts';
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
  KippleConfig,
} from './types.ts';

export { isTauri };

let opCounter = 0;
const nextOp = () => `op-${Date.now().toString(36)}-${(opCounter++).toString(36)}`;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    throw KippleError.from(err);
  }
}

async function stream<T, E>(
  command: string,
  args: Record<string, unknown>,
  opts: StreamOptions<E> | undefined,
): Promise<T> {
  const channel = new Channel<E>();
  if (opts?.onEvent) channel.onmessage = opts.onEvent;
  const op = nextOp();
  const abort = () => void invoke('cancel', { op }).catch(() => undefined);
  opts?.signal?.addEventListener('abort', abort, { once: true });
  try {
    return await call<T>(command, { ...args, op, onEvent: channel });
  } finally {
    opts?.signal?.removeEventListener('abort', abort);
  }
}

export class TauriClient implements KippleClient {
  readonly kind = 'tauri' as const;

  integrations(): Promise<IntegrationStatusReport[]> {
    return call('integrations');
  }
  scan(req: ScanRequest, opts?: StreamOptions<ScanEvent>): Promise<ScanResult> {
    return stream('scan', { req }, opts);
  }
  items(query: ItemQuery = {}): Promise<Item[]> {
    return call('items', { query });
  }
  projects(query: ProjectQuery = {}): Promise<ProjectSummary[]> {
    return call('projects', { query });
  }
  explain(id: ItemId): Promise<Explanation> {
    return call('explain', { item: id });
  }
  plan(req: PlanRequest): Promise<Plan> {
    return call('plan', { req });
  }
  execute(plan: Plan, confirm: Confirmation, opts?: StreamOptions<ExecEvent>): Promise<ExecReport> {
    return stream('execute', { plan, confirm }, opts);
  }
  quarantine(): Promise<QuarantineEntry[]> {
    return call('quarantine');
  }
  restore(id: QuarantineId): Promise<RestoreReport> {
    return call('restore', { entry: id });
  }
  purge(req: PurgeRequest): Promise<PurgeReport> {
    return call('purge', { req });
  }
  history(req: HistoryRequest = {}): Promise<SizePoint[]> {
    return call('history', { req });
  }
  doctor(): Promise<DoctorReport> {
    return call('doctor');
  }
  report(): Promise<Report> {
    return call('report');
  }
  volume(): Promise<VolumeInfo> {
    return call('volume');
  }
  journal(): Promise<JournalEntry[]> {
    return call('journal');
  }
  config(): Promise<KippleConfig> {
    return call('config');
  }
  setConfig(config: KippleConfig): Promise<KippleConfig> {
    return call('set_config', { config });
  }
}

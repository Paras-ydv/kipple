/* eslint-disable */
// Generated from docs/schema/*.json by scripts/gen-types.mjs. Do not edit.
// Regenerate with: pnpm gen:types

/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Detection".
 */
export type Detection =
  | {
      state: 'not_present';
    }
  | {
      state: 'present';
      version?: string | null;
    }
  | {
      reason: string;
      state: 'unsupported';
    };
/**
 * What kind of junk an item is. Drives grouping and colors.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Category".
 */
export type Category =
  | 'project-artifact'
  | 'package-cache'
  | 'toolchain'
  | 'container'
  | 'mobile'
  | 'ai-history'
  | 'model'
  | 'ide-cache'
  | 'build-cache'
  | 'logs'
  | 'system';
/**
 * Stable integration id, e.g. `npm-cache`, `docker`, `codex`.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "IntegrationId".
 */
export type IntegrationId = string;
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "IntegrationKind".
 */
export type IntegrationKind = 'pack' | 'adapter';
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Os".
 */
export type Os = 'macos' | 'linux' | 'windows' | 'wsl';
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "IntegrationStatus".
 */
export type IntegrationStatus = 'stable' | 'experimental';
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ExecEvent".
 */
export type ExecEvent =
  | {
      entries: number;
      event: 'started';
      plan_id: PlanId;
    }
  | {
      event: 'entry_started';
      index: number;
      title: string;
    }
  | {
      entry: ExecEntry;
      event: 'entry_finished';
      index: number;
    }
  | {
      event: 'warning';
      message: string;
    }
  | {
      event: 'finished';
    };
/**
 * ULID of a plan.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "PlanId".
 */
export type PlanId = string;
/**
 * What to do with an item (SPEC §4.3).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ActionKind".
 */
export type ActionKind =
  | {
      type: 'quarantine';
    }
  | {
      type: 'trash';
    }
  | {
      command_id: string;
      type: 'native';
    }
  | {
      method: string;
      type: 'compact';
    }
  | {
      type: 'archive';
    }
  | {
      target: string;
      type: 'relocate';
    }
  | {
      key: string;
      type: 'set_retention';
      value: unknown;
    };
/**
 * `blake3(integration_id + "\0" + canonical_locator)`, first 16 bytes, hex.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ItemId".
 */
export type ItemId = string;
/**
 * What happened to one plan entry.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Outcome".
 */
export type Outcome =
  | {
      freed: number;
      status: 'done';
    }
  | {
      reason: string;
      status: 'skipped';
    }
  | {
      error: string;
      status: 'failed';
    };
/**
 * ULID of a quarantine entry.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "QuarantineId".
 */
export type QuarantineId = string;
/**
 * Filesystem identity captured at scan time; re-checked before acting.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Identity".
 */
export type Identity =
  | {
      dev: number;
      ino: number;
      type: 'inode';
    }
  | {
      nanos: number;
      type: 'created';
    };
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "LastUsedSource".
 */
export type LastUsedSource =
  'git_commit' | 'git_index' | 'source_mtime' | 'item_mtime' | 'tool_metadata' | 'atime';
/**
 * Where an item lives.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Locator".
 */
export type Locator =
  | {
      path: string;
      type: 'path';
    }
  | {
      paths: string[];
      type: 'paths';
    }
  | {
      id: string;
      kind: string;
      type: 'docker';
    }
  | {
      key: string;
      store: string;
      type: 'logical';
    };
/**
 * `blake3(canonical project root)`, first 16 bytes, hex.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ProjectId".
 */
export type ProjectId = string;
/**
 * Stable machine codes for reasons (SPEC §7.3).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ReasonCode".
 */
export type ReasonCode =
  | (
      | 'REGENERABLE'
      | 'REDOWNLOADABLE'
      | 'INACTIVE_PROJECT'
      | 'RECENTLY_USED'
      | 'RECENTLY_WRITTEN'
      | 'ACTIVE_WRITER'
      | 'TOOL_RUNNING'
      | 'IN_USE_BY_CONTAINER'
      | 'UNUSED_BY_ANY_EMULATOR'
      | 'USED_BY_EMULATOR'
      | 'CONTAINS_USER_DATA'
      | 'CHAT_HISTORY'
      | 'MAY_CONTAIN_SECRETS'
      | 'LARGE_REDOWNLOAD'
      | 'ORPHANED_PROJECT'
      | 'MIGRATED_COPY'
      | 'BACKUP_COPY'
      | 'FREE_PAGES'
      | 'PROTECTED_PATH'
      | 'OUTSIDE_ROOTS'
      | 'UNKNOWN_FORMAT'
      | 'NEEDS_ELEVATION'
      | 'NATIVE_TOOL_MISSING'
      | 'SHARED_HARDLINKS'
      | 'PINNED_BY_PROJECT'
    )
  | 'EXPERIMENTAL'
  | 'USER_OVERRIDE'
  | 'CONTAINS_SOURCE';
/**
 * How safe an item is to remove. Ordered from least to most strict.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Safety".
 */
export type Safety = 'safe' | 'review' | 'keep';
/**
 * ULID of a scan.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ScanId".
 */
export type ScanId = string;
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ScanStatus".
 */
export type ScanStatus = 'running' | 'complete' | 'cancelled' | 'failed';
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ScanEvent".
 */
export type ScanEvent =
  | {
      event: 'started';
      roots: string[];
    }
  | {
      bytes: number;
      done: number;
      event: 'progress';
      total?: number | null;
    }
  | {
      event: 'item_found';
      item: Item;
    }
  | {
      event: 'warning';
      message: string;
    }
  | {
      event: 'finished';
      result: ScanResult;
    };
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "SizeScope".
 */
export type SizeScope = 'total' | 'category' | 'integration' | 'project';

export interface KippleSchemaRoot {
  DoctorReport?: DoctorReport;
  ExecEvent?: ExecEvent;
  ExecReport?: ExecReport;
  Explanation?: Explanation;
  IntegrationStatusReport?: IntegrationStatusReport;
  ItemFilter?: ItemFilter;
  Item?: Item;
  JournalEntry?: JournalEntry;
  PlanRequest?: PlanRequest;
  Plan?: Plan;
  ProjectSummary?: ProjectSummary;
  PurgeReport?: PurgeReport;
  QuarantineEntry?: QuarantineEntry;
  Report?: Report;
  RestoreReport?: RestoreReport;
  ScanEvent?: ScanEvent;
  ScanResult?: ScanResult;
  SizePoint?: SizePoint;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "DoctorReport".
 */
export interface DoctorReport {
  config_exists: boolean;
  config_path: string;
  data_dir: string;
  integrations: DoctorIntegration[];
  platform: PlatformInfo;
  warnings: string[];
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "DoctorIntegration".
 */
export interface DoctorIntegration {
  detection: Detection;
  enabled: boolean;
  /**
   * Actionable fixes, one sentence each.
   */
  fixes: string[];
  info: IntegrationInfo;
  locations: LocationCheck[];
  tools: ToolCheck[];
}
/**
 * Static description of an integration.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "IntegrationInfo".
 */
export interface IntegrationInfo {
  category: Category;
  description: string;
  docs_url?: string | null;
  id: IntegrationId;
  /**
   * Process names that make this integration's data in use.
   */
  in_use_processes?: string[];
  kind: IntegrationKind;
  name: string;
  /**
   * Empty means every OS.
   */
  os?: Os[];
  status?: IntegrationStatus;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "LocationCheck".
 */
export interface LocationCheck {
  exists: boolean;
  path: string;
  readable: boolean;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ToolCheck".
 */
export interface ToolCheck {
  found?: string | null;
  name: string;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "PlatformInfo".
 */
export interface PlatformInfo {
  arch: string;
  os: Os;
  trash_available: boolean;
  wsl: boolean;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ExecEntry".
 */
export interface ExecEntry {
  action: ActionKind;
  item_id: ItemId;
  outcome: Outcome;
  quarantine_id?: QuarantineId | null;
  title: string;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ExecReport".
 */
export interface ExecReport {
  /**
   * Stopped early by cancellation.
   */
  cancelled: boolean;
  done: number;
  entries: ExecEntry[];
  failed: number;
  /**
   * Bytes freed right away.
   */
  freed_now: number;
  /**
   * Bytes moved to quarantine; freed after purge.
   */
  held_in_quarantine: number;
  plan_id: PlanId;
  skipped: number;
}
/**
 * Everything about one item (`kipple explain`).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Explanation".
 */
export interface Explanation {
  /**
   * What each allowed action does, in order of preference.
   */
  actions: ActionInfo[];
  integration: IntegrationInfo;
  item: Item;
  /**
   * `apparent` and `reclaimable` differ by more than 10%.
   */
  size_differs: boolean;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ActionInfo".
 */
export interface ActionInfo {
  action: ActionKind;
  description: string;
  undoable: boolean;
}
/**
 * One reclaimable thing reported by an integration.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Item".
 */
export interface Item {
  /**
   * Allowed actions, preferred first. Empty when `safety` is `Keep`.
   */
  actions: ActionKind[];
  category: Category;
  /**
   * Integration-specific extras.
   */
  details?: {
    [k: string]: unknown | undefined;
  };
  id: ItemId;
  /**
   * Identity of the primary path at scan time.
   */
  identity?: Identity | null;
  integration: IntegrationId;
  /**
   * Integration-specific kind, e.g. `node_modules`, `docker_image`.
   */
  kind: string;
  last_used?: LastUsed | null;
  locator: Locator;
  notes?: string[];
  project?: ProjectRef | null;
  /**
   * At least one.
   */
  reasons: Reason[];
  /**
   * Command that regenerates the item, shown by `explain`.
   */
  restore_hint?: string | null;
  safety: Safety;
  size: Size;
  /**
   * Human label.
   */
  title: string;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "LastUsed".
 */
export interface LastUsed {
  at: string;
  source: LastUsedSource;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ProjectRef".
 */
export interface ProjectRef {
  id: ProjectId;
  name: string;
  root: string;
}
/**
 * Why an item has its safety level. One plain sentence.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Reason".
 */
export interface Reason {
  code: ReasonCode;
  text: string;
}
/**
 * Honest sizes (SPEC §5.5).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Size".
 */
export interface Size {
  /**
   * Sum of file lengths.
   */
  apparent: number;
  file_count?: number | null;
  /**
   * Allocated blocks.
   */
  on_disk: number;
  /**
   * What deleting frees, after hardlink and shared-store accounting.
   */
  reclaimable: number;
}
/**
 * An integration and whether it was detected on this machine.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "IntegrationStatusReport".
 */
export interface IntegrationStatusReport {
  category: Category;
  description: string;
  detection: Detection;
  docs_url?: string | null;
  enabled: boolean;
  id: IntegrationId;
  /**
   * Process names that make this integration's data in use.
   */
  in_use_processes?: string[];
  kind: IntegrationKind;
  name: string;
  /**
   * Empty means every OS.
   */
  os?: Os[];
  status?: IntegrationStatus;
}
/**
 * Selects items. Shared by `items`, `plan`, the CLI and MCP.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ItemFilter".
 */
export interface ItemFilter {
  /**
   * Glob patterns matched against item paths.
   */
  exclude?: string[];
  /**
   * Item ids (or unique prefixes).
   */
  ids?: string[];
  /**
   * Reclaimable at least this many bytes.
   */
  min_size?: number | null;
  /**
   * Last used at least this many seconds ago.
   */
  older_than_secs?: number | null;
  /**
   * Integration ids or category ids.
   */
  only?: string[];
  project?: ProjectId | null;
  /**
   * Only items under these roots.
   */
  roots?: string[];
  safe_only?: boolean;
  /**
   * Restrict to these levels. Empty means all.
   */
  safety?: Safety[];
}
/**
 * One executed action (SPEC §8.5).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "JournalEntry".
 */
export interface JournalEntry {
  action: ActionKind;
  freed: number;
  integration: IntegrationId;
  item_id: ItemId;
  original_path?: string | null;
  outcome: Outcome;
  plan_id: PlanId;
  quarantine_id?: QuarantineId | null;
  title: string;
  ts: string;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "PlanRequest".
 */
export interface PlanRequest {
  /**
   * Use this action instead of the preferred one.
   */
  action_overrides?: {
    [k: string]: ActionKind | undefined;
  };
  /**
   * Item ids to leave out.
   */
  exclude_ids?: string[];
  filter?: ItemFilter;
}
/**
 * An ordered list of (item, action) the user approves (SPEC §4.3, §8.1).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Plan".
 */
export interface Plan {
  created_at: string;
  entries: PlanEntry[];
  id: PlanId;
  scan_id: ScanId;
  totals: PlanTotals;
  warnings?: string[];
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "PlanEntry".
 */
export interface PlanEntry {
  action: ActionKind;
  item: Item;
  preflight?: Check[];
}
/**
 * Result of one preflight check.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Check".
 */
export interface Check {
  id: string;
  message: string;
  ok: boolean;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "PlanTotals".
 */
export interface PlanTotals {
  /**
   * Reclaimable bytes per action name.
   */
  by_action: {
    [k: string]: number | undefined;
  };
  by_safety: SafetyMix;
  entries: number;
  /**
   * Freed when the plan runs (native commands, trash, compaction).
   */
  freed_now: number;
  /**
   * Held in quarantine until purged.
   */
  held_in_quarantine: number;
  /**
   * Reclaimable bytes of all entries.
   */
  reclaimable: number;
}
/**
 * Reclaimable bytes per safety level.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "SafetyMix".
 */
export interface SafetyMix {
  keep: number;
  review: number;
  safe: number;
}
/**
 * A source-code directory and the items attributed to it (SPEC §4.2).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ProjectSummary".
 */
export interface ProjectSummary {
  /**
   * Reclaimable bytes per category.
   */
  by_category: {
    'ai-history'?: number;
    'build-cache'?: number;
    container?: number;
    'ide-cache'?: number;
    logs?: number;
    mobile?: number;
    model?: number;
    'package-cache'?: number;
    'project-artifact'?: number;
    system?: number;
    toolchain?: number;
  };
  /**
   * `false` when tool metadata points to a folder that no longer exists.
   */
  exists: boolean;
  id: ProjectId;
  /**
   * Number of attributed items.
   */
  item_count?: number;
  last_used?: LastUsed | null;
  /**
   * Marker files found at the root: `package.json`, `.git`, …
   */
  markers: string[];
  /**
   * Package name if known, else folder name.
   */
  name: string;
  root: string;
  safety_mix: SafetyMix;
  total: Size;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "PurgeReport".
 */
export interface PurgeReport {
  errors?: string[];
  freed: number;
  purged: QuarantineId[];
}
/**
 * One quarantined item (SPEC §8.4).
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "QuarantineEntry".
 */
export interface QuarantineEntry {
  created_at: string;
  expires_at: string;
  id: QuarantineId;
  integration: IntegrationId;
  item: Item;
  /**
   * Original location of each moved path.
   */
  original_paths: string[];
  plan_id: PlanId;
  size: Size;
}
/**
 * Latest scan report: summary plus heaviest projects.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Report".
 */
export interface Report {
  /**
   * What `kipple clean --safe-only` would do.
   */
  next?: NextStep | null;
  scan: ScanInfo;
  summary: Summary;
  top_projects: ProjectSummary[];
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "NextStep".
 */
export interface NextStep {
  command: string;
  keep_days: number;
  reclaimable: number;
  /**
   * Part of `reclaimable` that can be undone during the quarantine window.
   */
  undoable: number;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ScanInfo".
 */
export interface ScanInfo {
  duration_ms: number;
  /**
   * Directory entries visited by the walk.
   */
  entries_seen: number;
  finished_at?: string | null;
  id: ScanId;
  kipple_version: string;
  roots: string[];
  started_at: string;
  status: ScanStatus;
}
/**
 * Report numbers for one scan.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "Summary".
 */
export interface Summary {
  by_safety: SafetyMix;
  categories: CategorySummary[];
  items: number;
  /**
   * Bytes held in quarantine, not yet freed.
   */
  quarantine_bytes: number;
  reclaimable: number;
}
/**
 * One row of the per-category summary.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "CategorySummary".
 */
export interface CategorySummary {
  by_safety: SafetyMix;
  category: Category;
  items: number;
  reclaimable: number;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "RestoreReport".
 */
export interface RestoreReport {
  restored: RestoredEntry[];
  skipped: SkippedRestore[];
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "RestoredEntry".
 */
export interface RestoredEntry {
  id: QuarantineId;
  paths: string[];
  title: string;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "SkippedRestore".
 */
export interface SkippedRestore {
  id: QuarantineId;
  reason: string;
}
/**
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "ScanResult".
 */
export interface ScanResult {
  scan: ScanInfo;
  summary: Summary;
  warnings?: string[];
}
/**
 * Reclaimable bytes at one point in time.
 *
 * This interface was referenced by `KippleSchemaRoot`'s JSON-Schema
 * via the `definition` "SizePoint".
 */
export interface SizePoint {
  key: string;
  reclaimable: number;
  scope: SizeScope;
  ts: string;
}

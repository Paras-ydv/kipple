// Core types. Schema types are generated from docs/schema (src/generated);
// request types below mirror SPEC §3.4 arguments that have no exported schema yet.

export type * from './generated/schema.ts';

import type { ItemFilter, PlanRequest, QuarantineId, SizeScope } from './generated/schema.ts';

/** `ScanRequest` (SPEC §3.4, §5.1). */
export interface ScanRequest {
  roots?: string[];
  /** Integration or category ids. */
  only?: string[];
  min_size?: number | null;
}

/** `ItemQuery` (SPEC §3.4) uses the shared `ItemFilter`. */
export type ItemQuery = ItemFilter;

/** `ProjectQuery` (SPEC §3.4). */
export interface ProjectQuery {
  /** Case-insensitive match on name or path. */
  search?: string;
  /** Last used at least this many seconds ago. */
  older_than_secs?: number | null;
  /** Only projects whose folder is gone (`exists == false`). */
  orphaned?: boolean;
  limit?: number | null;
}

/** How the user confirmed a plan (SPEC §8.2). */
export type Confirmation =
  /** One click; only valid when the plan has no Review items. */
  | { kind: 'click'; purge_now?: boolean }
  /** The user typed a word; must be `reclaim` for plans with Review items. */
  | { kind: 'typed'; text: string; purge_now?: boolean };

/** `PurgeRequest` (SPEC §3.4, §8.4). */
export interface PurgeRequest {
  /** Specific entries; empty means every entry. */
  ids?: QuarantineId[];
  /** Only entries older than this many seconds. */
  older_than_secs?: number | null;
}

/** `HistoryRequest` (SPEC §3.4): growth timeline. */
export interface HistoryRequest {
  scope?: SizeScope;
  /** Category, integration or project key; empty means all of that scope. */
  key?: string | null;
  /** RFC 3339 lower bound. */
  since?: string | null;
}

/** App extra, not in SPEC §3.4: the volume that holds the home folder. */
export interface VolumeInfo {
  name: string;
  mount: string;
  capacity: number;
  free: number;
}

export type { ItemFilter, PlanRequest };

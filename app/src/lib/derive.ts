// Pure helpers that turn core data into what the screens show.

import type { ActionKind, Item, PlanEntry, SizePoint } from '@kipple/client';
import { NATIVE, entryBytes } from '@kipple/client';
import { categoryColor, formatAgo, type FlagKind } from '@kipple/design';

// ---- Labels ---------------------------------------------------------------------

interface GroupLabel {
  /** Row label for a group of `n` items. */
  many: (n: number, projects: number) => string;
  /** Short name used in "and 4 more: uv, pip, …". */
  short: string;
}

const KIND_LABELS: Record<string, GroupLabel> = {
  'docker/docker_build_cache': { many: () => 'Docker build cache', short: 'Docker build cache' },
  'docker/docker_image': { many: (n) => `${n} Docker images`, short: 'Docker images' },
  'docker/docker_volume': { many: (n) => `${n} Docker volumes`, short: 'Docker volumes' },
  'docker/docker_container': { many: (n) => `${n} stopped containers`, short: 'stopped containers' },
  'node-modules/node_modules': { many: (_n, p) => (p > 1 ? `node_modules in ${p} projects` : 'node_modules'), short: 'node_modules' },
  'python-caches/python_cache': { many: () => 'Python caches', short: 'Python caches' },
  'codex/codex_session': { many: (n) => `${n} Codex sessions`, short: 'Codex' },
  'claude-code/claude_code_session': { many: (n) => `${n} Claude Code sessions`, short: 'Claude Code' },
};

const INTEGRATION_LABELS: Record<string, GroupLabel> = {
  'npm-cache': { many: () => 'npm cache', short: 'npm' },
  'uv-cache': { many: () => 'uv cache', short: 'uv' },
  'pip-cache': { many: () => 'pip cache', short: 'pip' },
  'homebrew-cache': { many: () => 'Homebrew cache', short: 'Homebrew' },
  'pnpm-store': { many: () => 'pnpm store', short: 'pnpm' },
  'gradle-cache': { many: () => 'Gradle caches', short: 'Gradle' },
  playwright: { many: () => 'Playwright browsers', short: 'Playwright' },
  'js-build': { many: (_n, p) => (p > 1 ? `Build output in ${p} projects` : 'Build output'), short: 'build output' },
  'xcode-derived-data': { many: () => 'Xcode DerivedData', short: 'DerivedData' },
  'android-system-images': { many: (n) => (n > 1 ? `${n} Android images` : 'Android system image'), short: 'Android image' },
  'android-avd': { many: (n) => `${n} emulators`, short: 'emulators' },
  'ios-device-support': { many: () => 'iOS DeviceSupport', short: 'DeviceSupport' },
  'python-venv': { many: (_n, p) => (p > 1 ? `.venv in ${p} projects` : '.venv'), short: '.venv' },
  'rust-target': { many: () => 'Rust target', short: 'target' },
  ollama: { many: (n) => `${n} Ollama models`, short: 'Ollama' },
  huggingface: { many: (n) => `${n} Hugging Face models`, short: 'Hugging Face' },
  cursor: { many: () => 'Cursor', short: 'Cursor' },
  antigravity: { many: (n) => `${n} Antigravity copies`, short: 'Antigravity' },
  'python-caches': { many: (_n, p) => (p > 1 ? `Python caches in ${p} projects` : 'Python caches'), short: 'Python caches' },
  'maven-project': { many: (_n, p) => (p > 1 ? `Maven target in ${p} projects` : 'Maven target'), short: 'Maven target' },
  'gradle-project': { many: (_n, p) => (p > 1 ? `Gradle build output in ${p} projects` : 'Gradle build output'), short: 'Gradle build' },
  'xcode-project': { many: (_n, p) => (p > 1 ? `Swift and CocoaPods builds in ${p} projects` : 'Swift and CocoaPods builds'), short: 'Swift builds' },
  'cachedir-tag': { many: (n) => `${n} tagged caches`, short: 'tagged caches' },
  'bun-cache': { many: () => 'Bun cache', short: 'Bun' },
  'yarn-cache': { many: () => 'Yarn caches', short: 'Yarn' },
  'cargo-registry': { many: () => 'Cargo registry', short: 'Cargo' },
  'go-build-cache': { many: () => 'Go build cache', short: 'Go build' },
  'go-mod-cache': { many: () => 'Go module cache', short: 'Go modules' },
  puppeteer: { many: (n) => `${n} Puppeteer browsers`, short: 'Puppeteer' },
  'claude-code': { many: (n) => `${n} Claude Code items`, short: 'Claude Code' },
  codex: { many: (n) => `${n} Codex sessions`, short: 'Codex' },
};

export interface ItemGroup {
  key: string;
  title: string;
  short: string;
  items: Item[];
  bytes: number;
  color: string;
  safety: Item['safety'];
}

/** Groups items for compact lists. Single items keep their own title. */
export function groupItems(
  items: Item[],
  keyOf: (i: Item) => string = (i) => `${i.integration}/${i.kind}/${i.safety}`,
  bytesOf: (i: Item) => number = (i) => i.size.reclaimable,
): ItemGroup[] {
  const map = new Map<string, Item[]>();
  for (const i of items) map.set(keyOf(i), [...(map.get(keyOf(i)) ?? []), i]);
  const groups: ItemGroup[] = [];
  for (const [key, list] of map) {
    const first = list[0] as Item;
    const kindKey = `${first.integration}/${first.kind}`;
    const label = KIND_LABELS[kindKey] ?? INTEGRATION_LABELS[first.integration];
    const projects = new Set(list.map((i) => i.project?.id ?? i.id)).size;
    const dangling = list.every((i) => (i.details as { dangling?: boolean } | undefined)?.dangling);
    // A lone project artifact says whose it is ("target in tool").
    let title =
      first.project && (first.category === 'project-artifact' || first.category === 'build-cache') && first.title !== first.project.name
        ? `${first.title} in ${first.project.name}`
        : first.title;
    if (list.length > 1) {
      if (kindKey === 'docker/docker_image' && dangling && keyOf === planKey) title = `${list.length} dangling Docker images`;
      else title = label ? label.many(list.length, projects) : `${list.length} × ${first.title}`;
    }
    groups.push({
      key,
      title,
      short: label?.short ?? first.title,
      items: list,
      bytes: list.reduce((s, i) => s + bytesOf(i), 0),
      color: categoryColor(first.category),
      safety: first.safety,
    });
  }
  return groups.sort((a, b) => b.bytes - a.bytes);
}

/** Plan rows: grouped by integration (and Docker kind). */
export function groupPlanEntries(entries: PlanEntry[]): { removed: ItemGroup[]; quarantined: ItemGroup[] } {
  const byTool = entries.filter((e) => !isQuarantineLike(e.action));
  const held = entries.filter((e) => isQuarantineLike(e.action));
  const bytes = new Map(entries.map((e) => [e.item.id, entryBytes(e.item, e.action)]));
  const of = (list: PlanEntry[]) =>
    groupItems(
      list.map((e) => e.item),
      planKey,
      (i) => bytes.get(i.id) ?? i.size.reclaimable,
    );
  return { removed: of(byTool), quarantined: of(held) };
}

/** Plan rows group by integration, and by kind for Docker. */
export function planKey(i: Item): string {
  return i.integration === 'docker' ? `docker/${i.kind}` : i.integration;
}

export const isQuarantineLike =(a: ActionKind) => a.type === 'quarantine' || a.type === 'trash';

/** "docker volume rm (no undo)", "Moved to quarantine, undo for 7 days". */
export function actionWords(a: ActionKind | undefined, keepDays = 7): string {
  if (!a) return 'Nothing. kipple keeps it.';
  switch (a.type) {
    case 'quarantine':
      return `Moved to quarantine (undo for ${keepDays} days)`;
    case 'trash':
      return 'Moved to the trash';
    case 'native':
      return NATIVE[a.command_id] ?? a.command_id;
    case 'compact':
      return a.method === 'sqlite_vacuum' ? 'Compact the database (no undo)' : 'Compact (no undo)';
    default:
      return a.type;
  }
}

// ---- AI history -----------------------------------------------------------------

export const AI_TOOLS = ['codex', 'cursor', 'claude-code', 'antigravity'] as const;
export type AiTool = (typeof AI_TOOLS)[number];

export const TOOL_NAMES: Record<string, string> = {
  codex: 'Codex',
  cursor: 'Cursor',
  'claude-code': 'Claude Code',
  antigravity: 'Antigravity',
};

export function sessionFlag(item: Item): { kind: FlagKind; text: string } | null {
  const d = item.details as { flag?: string | null } | undefined;
  switch (d?.flag) {
    case 'tool':
      return { kind: 'tool', text: 'Mostly tool output' };
    case 'secret':
      return { kind: 'secret', text: 'API key inside' };
    case 'orphan':
      return { kind: 'orphan', text: 'Folder deleted' };
    case 'open':
      return { kind: 'locked', text: `Open in ${TOOL_NAMES[item.integration] ?? 'the tool'}` };
    default:
      return null;
  }
}

export function sessionSubtitle(item: Item, now = new Date()): string {
  const project = item.project?.name;
  const age = item.last_used ? formatAgo(item.last_used.at, now).toLowerCase() : '';
  // Tools that keep several copies (Antigravity) say which one this is.
  const root = (item.details as { data_root?: unknown } | undefined)?.data_root;
  const copy = item.reasons.some((r) => r.code === 'MIGRATED_COPY') ? 'older copy' : '';
  const where = typeof root === 'string' ? `in ${root}` : '';
  return [copy, where, project, age].filter(Boolean).join(', ');
}

/** Bytes added per month for the last 12 months, from cumulative SizePoints. */
export function monthlyFromHistory(points: SizePoint[], now = new Date()): { label: string; gb: number; current: boolean }[] | null {
  const sorted = [...points].sort((a, b) => a.ts.localeCompare(b.ts));
  if (sorted.length < 3) return null;
  const months = lastTwelveMonths(now);
  const out = months.map((m) => ({ ...m, gb: 0 }));
  // Net change per month: what the month added after anything that was removed.
  for (let i = 1; i < sorted.length; i++) {
    const prev = sorted[i - 1] as SizePoint;
    const cur = sorted[i] as SizePoint;
    const d = new Date(cur.ts);
    const idx = months.findIndex((m) => m.year === d.getUTCFullYear() && m.month === d.getUTCMonth());
    const slot = out[idx];
    if (slot) slot.gb += (cur.reclaimable - prev.reclaimable) / 1e9;
  }
  return out.map(({ label, gb, current }) => ({ label, gb: Math.max(0, gb), current }));
}

/** Bytes per month of last use, as a fallback when there is no history. */
export function monthlyFromItems(items: Item[], now = new Date()): { label: string; gb: number; current: boolean }[] {
  const months = lastTwelveMonths(now);
  const out = months.map((m) => ({ ...m, gb: 0 }));
  for (const i of items) {
    if (!i.last_used) continue;
    const d = new Date(i.last_used.at);
    const idx = months.findIndex((m) => m.year === d.getUTCFullYear() && m.month === d.getUTCMonth());
    const slot = out[idx];
    if (slot) slot.gb += i.size.reclaimable / 1e9;
  }
  return out.map(({ label, gb, current }) => ({ label, gb, current }));
}

function lastTwelveMonths(now: Date) {
  const names = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  return Array.from({ length: 12 }, (_, i) => {
    const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() - 11 + i, 1));
    return { year: d.getUTCFullYear(), month: d.getUTCMonth(), label: names[d.getUTCMonth()] as string, current: i === 11 };
  });
}

// ---- Growth ---------------------------------------------------------------------

const GROWTH_LABELS: Record<string, { label: string; category: string }> = {
  codex: { label: 'Codex sessions', category: 'ai-history' },
  'docker/docker_build_cache': { label: 'Docker build cache', category: 'container' },
  'cursor/chat_db': { label: 'Cursor chat database', category: 'ai-history' },
  'claude-code': { label: 'Claude Code transcripts', category: 'ai-history' },
};

export interface Growth {
  key: string;
  label: string;
  color: string;
  delta: number;
}

/** Growth since `since` per integration key, largest first, and the total. */
export function growth(points: SizePoint[], since: number): { total: number; rows: Growth[] } {
  const byKey = new Map<string, SizePoint[]>();
  for (const p of points) byKey.set(`${p.scope}:${p.key}`, [...(byKey.get(`${p.scope}:${p.key}`) ?? []), p]);
  const delta = (list: SizePoint[]) => {
    const sorted = [...list].sort((a, b) => a.ts.localeCompare(b.ts));
    const base = [...sorted].reverse().find((p) => Date.parse(p.ts) <= since) ?? sorted[0];
    const last = sorted[sorted.length - 1];
    return base && last ? last.reclaimable - base.reclaimable : 0;
  };
  const rows: Growth[] = [];
  let total = 0;
  for (const [k, list] of byKey) {
    const [scope, key] = k.split(':') as [string, string];
    if (scope === 'total') {
      total = delta(list);
      continue;
    }
    if (scope !== 'integration') continue;
    const d = delta(list);
    if (d <= 0) continue;
    const meta = GROWTH_LABELS[key];
    rows.push({ key, label: meta?.label ?? key, color: categoryColor(meta?.category ?? 'system'), delta: d });
  }
  rows.sort((a, b) => b.delta - a.delta);
  if (!total) total = rows.reduce((s, r) => s + r.delta, 0);
  return { total, rows };
}

export function platformFinder(): string {
  const ua = typeof navigator === 'undefined' ? '' : navigator.userAgent;
  if (/Windows/i.test(ua)) return 'Show in Explorer';
  if (/Mac/i.test(ua)) return 'Show in Finder';
  return 'Show in Files';
}

export function deviceWord(): 'Mac' | 'PC' | 'computer' {
  const ua = typeof navigator === 'undefined' ? '' : navigator.userAgent;
  if (/Mac/i.test(ua)) return 'Mac';
  if (/Windows/i.test(ua)) return 'PC';
  return 'computer';
}

// ---- Review copy ----------------------------------------------------------------

const REVIEW_WORDS: Record<string, string> = {
  'ai-history': 'chat history',
  container: 'Docker data',
  mobile: 'emulators',
  model: 'models',
  'package-cache': 'caches',
  'project-artifact': 'project files',
  'build-cache': 'build caches',
  toolchain: 'toolchains',
  'ide-cache': 'editor caches',
  logs: 'logs',
  system: 'system files',
};

/** "chat history, models and emulators": what review bytes are made of, largest first. */
export function reviewPhrase(categories: { category: string; by_safety: { review: number } }[], max = 3): string {
  const words = [...categories]
    .filter((c) => c.by_safety.review > 0)
    .sort((a, b) => b.by_safety.review - a.by_safety.review)
    .map((c) => REVIEW_WORDS[c.category] ?? c.category)
    .filter((w, i, all) => all.indexOf(w) === i)
    .slice(0, max);
  if (words.length < 2) return words[0] ?? '';
  return `${words.slice(0, -1).join(', ')} and ${words[words.length - 1]}`;
}

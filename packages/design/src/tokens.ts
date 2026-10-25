// Design tokens (docs/UI.md §2). Mirrors src/styles/tokens.css; keep both in sync.

/** Category ids as serialized by the core (`Category` in docs/schema). */
export type CategoryId =
  | 'container'
  | 'ai-history'
  | 'package-cache'
  | 'project-artifact'
  | 'mobile'
  | 'model'
  | 'toolchain'
  | 'build-cache'
  | 'ide-cache'
  | 'logs'
  | 'system';

export type SafetyLevel = 'safe' | 'review' | 'keep';

export const color = {
  text: '#F5F6FA',
  textSub: 'rgba(255,255,255,.88)',
  text2: 'rgba(255,255,255,.72)',
  text3: 'rgba(255,255,255,.62)',
  text4: 'rgba(255,255,255,.55)',
  line: 'rgba(255,255,255,.09)',
  lineSoft: 'rgba(255,255,255,.08)',
  lineStrong: 'rgba(255,255,255,.12)',
  track: 'rgba(255,255,255,.07)',
  accent: '#1677FF',
  accentTop: '#5BA9FF',
  accentIcon: '#7FB8FF',
  accentText: '#9FCBFF',
  positive: '#8CF0A6',
  growth: '#FFC46B',
  secret: '#FFD597',
  secretBody: 'rgba(255,236,210,.82)',
  stage: '#06080F',
  trafficClose: '#FF5F57',
  trafficMin: '#FEBC2E',
  trafficZoom: '#28C840',
} as const;

export interface CategoryToken {
  id: CategoryId;
  /** Sentence-case label used in the app. */
  label: string;
  /** Lowercase legend label used in the terminal UI. */
  short: string;
  /** Label used on the share card chips. */
  share: string;
  color: string;
}

export const categories: Record<CategoryId, CategoryToken> = {
  container: { id: 'container', label: 'Containers', short: 'containers', share: 'Docker', color: '#2F6BFF' },
  'ai-history': { id: 'ai-history', label: 'AI history', short: 'ai history', share: 'AI chat history', color: '#8B5CF6' },
  'package-cache': { id: 'package-cache', label: 'Package caches', short: 'caches', share: 'caches', color: '#FFB020' },
  'project-artifact': { id: 'project-artifact', label: 'Project artifacts', short: 'artifacts', share: 'build output', color: '#3CCB5A' },
  mobile: { id: 'mobile', label: 'Mobile', short: 'mobile', share: 'mobile', color: '#12C8C2' },
  model: { id: 'model', label: 'Models', short: 'models', share: 'models', color: '#FF5C8A' },
  toolchain: { id: 'toolchain', label: 'Toolchains', short: 'toolchains', share: 'toolchains', color: '#FF7A45' },
  'build-cache': { id: 'build-cache', label: 'Build caches', short: 'build caches', share: 'build caches', color: '#A3D635' },
  'ide-cache': { id: 'ide-cache', label: 'IDE caches', short: 'ide caches', share: 'IDE caches', color: '#4CC9F0' },
  logs: { id: 'logs', label: 'Logs', short: 'logs', share: 'logs', color: '#B0B8C8' },
  system: { id: 'system', label: 'System', short: 'system', share: 'system', color: '#7B8591' },
};

export const kippleSwatch = 'linear-gradient(90deg, #2F6BFF, #8B5CF6, #FFB020, #3CCB5A)';

export interface SafetyToken {
  word: string;
  bg: string;
  fg: string;
  /** Terminal UI colour. */
  term: string;
  /** Legend dot shape. */
  shape: 'round' | 'square' | 'bar';
}

export const safety: Record<SafetyLevel, SafetyToken> = {
  safe: { word: 'Safe', bg: 'rgba(60,203,90,.18)', fg: '#7DE89A', term: '#6FC08F', shape: 'round' },
  review: { word: 'Review', bg: 'rgba(255,176,32,.18)', fg: '#FFC46B', term: '#E6A740', shape: 'square' },
  keep: { word: 'Keep', bg: 'rgba(255,95,87,.18)', fg: '#FF9C96', term: '#E57A70', shape: 'bar' },
};

export type FlagKind = 'tool' | 'secret' | 'orphan' | 'locked';

export const flags: Record<FlagKind, { bg: string; fg: string }> = {
  tool: { bg: 'rgba(255,255,255,.1)', fg: 'rgba(255,255,255,.75)' },
  secret: { bg: 'rgba(255,176,60,.2)', fg: '#FFC46B' },
  orphan: { bg: 'rgba(127,184,255,.18)', fg: '#9FCBFF' },
  locked: { bg: 'rgba(255,95,87,.18)', fg: '#FF9C96' },
};

export const font = {
  ui: `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, "Segoe UI", Roboto, "Helvetica Neue", sans-serif`,
  display: `-apple-system, BlinkMacSystemFont, "SF Pro Display", system-ui, "Segoe UI", Roboto, sans-serif`,
  brand: `"Mona Sans", -apple-system, system-ui, sans-serif`,
  mono: `ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace`,
} as const;

export interface TypeToken {
  size: number;
  weight: number;
  tracking: string;
  lineHeight?: number;
}

export const type: Record<string, TypeToken> = {
  hero: { size: 132, weight: 700, tracking: '-0.05em', lineHeight: 0.9 },
  heroUnit: { size: 44, weight: 600, tracking: '-0.02em' },
  displayL: { size: 64, weight: 700, tracking: '-0.045em', lineHeight: 1 },
  displayM: { size: 54, weight: 700, tracking: '-0.045em', lineHeight: 1 },
  displayS: { size: 46, weight: 700, tracking: '-0.045em', lineHeight: 1 },
  numberL: { size: 34, weight: 700, tracking: '-0.03em' },
  subhead: { size: 24, weight: 600, tracking: '-0.01em' },
  title: { size: 20, weight: 700, tracking: '-0.01em' },
  wordmark: { size: 17, weight: 800, tracking: '-0.02em' },
  bodyL: { size: 15, weight: 400, tracking: '0', lineHeight: 1.5 },
  body: { size: 13, weight: 400, tracking: '0' },
  small: { size: 12, weight: 400, tracking: '0' },
  caption: { size: 11, weight: 400, tracking: '0' },
};

export const radius = {
  bar: 7,
  row: 9,
  thumb: 16,
  sidebar: 17,
  panel: 20,
  figure: 22,
  popover: 24,
  window: 26,
} as const;

export const motion = {
  grow: { duration: 240, easing: 'cubic-bezier(.2,.8,.2,1)' },
  response: { duration: 150, easing: 'ease-out' },
} as const;

/** Category colour with a neutral fallback for unknown ids. */
export function categoryColor(id: string): string {
  return (categories as Record<string, CategoryToken | undefined>)[id]?.color ?? categories.system.color;
}

export function categoryLabel(id: string): string {
  return (categories as Record<string, CategoryToken | undefined>)[id]?.label ?? id;
}

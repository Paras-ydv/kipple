// Formatting rules from docs/UI.md §7 (Words).

export const GB = 1_000_000_000;
export const MB = 1_000_000;

/**
 * SI sizes: whole MB under 1 GB; one decimal from 1 GB; whole numbers at
 * 100 GB and above drop the decimal ("184 GB", "107.5 GB", "24.0 GB").
 */
export function formatBytes(bytes: number): string {
  const { value, unit } = splitBytes(bytes);
  return `${value} ${unit}`;
}

/** Number and unit separately, for display numbers ("184" + "GB"). */
export function splitBytes(bytes: number): { value: string; unit: 'GB' | 'MB' | 'KB' | 'bytes' } {
  const b = Math.max(0, bytes);
  if (b >= GB || Math.round(b / MB) >= 1000) {
    const gb = Math.round((b / GB) * 10) / 10;
    const text = gb >= 100 && Number.isInteger(gb) ? gb.toFixed(0) : gb.toFixed(1);
    return { value: text, unit: 'GB' };
  }
  if (b >= MB) return { value: String(Math.round(b / MB)), unit: 'MB' };
  if (b >= 1000) return { value: String(Math.round(b / 1000)), unit: 'KB' };
  return { value: String(b), unit: 'bytes' };
}

/** Disk capacity and free space: whole GB ("41 GB free of 512 GB"). */
export function formatDisk(bytes: number): string {
  return `${Math.round(bytes / GB)} GB`;
}

/** Signed growth delta ("+6.2 GB"). */
export function formatDelta(bytes: number): string {
  const sign = bytes < 0 ? '−' : '+';
  return `${sign}${formatBytes(Math.abs(bytes))}`;
}

/** Large counts in words: "1.02 million", "412,000". */
export function formatCount(n: number): string {
  if (n >= 1_000_000) {
    const m = n / 1_000_000;
    return `${m.toFixed(m < 10 ? 2 : 1)} million`;
  }
  return n.toLocaleString('en-US');
}

const UNITS: [number, string][] = [
  [365 * 24 * 3600, 'year'],
  [30 * 24 * 3600, 'month'],
  [7 * 24 * 3600, 'week'],
  [24 * 3600, 'day'],
  [3600, 'hour'],
  [60, 'minute'],
];

/** Relative age without "ago": "7 months", "2 minutes", "today". */
export function formatAge(iso: string | Date, now: Date = new Date()): string {
  const then = typeof iso === 'string' ? new Date(iso) : iso;
  const secs = Math.max(0, (now.getTime() - then.getTime()) / 1000);
  if (secs < 60) return 'just now';
  for (const [size, name] of UNITS) {
    if (secs >= size) {
      // Months and years round to the nearest ("58 days" reads as "2 months").
      const n = size >= 30 * 24 * 3600 ? Math.max(1, Math.round(secs / size)) : Math.floor(secs / size);
      return `${n} ${name}${n === 1 ? '' : 's'}`;
    }
  }
  return 'just now';
}

/** "7 months ago", "2 minutes ago", "Just now". */
export function formatAgo(iso: string | Date, now: Date = new Date()): string {
  const age = formatAge(iso, now);
  return age === 'just now' ? 'Just now' : `${age} ago`;
}

/** Seconds in plain words: "18 seconds", "2 minutes". */
export function formatDuration(ms: number): string {
  if (ms < 1000) return 'under a second';
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} second${s === 1 ? '' : 's'}`;
  const m = Math.round(s / 60);
  return `${m} minute${m === 1 ? '' : 's'}`;
}

/** "2 November" */
export function formatDay(iso: string | Date): string {
  const d = typeof iso === 'string' ? new Date(iso) : iso;
  return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'long' });
}

let homeDir: string | undefined;

/** Sets the home folder `tildePath` shortens against (from the core). */
export function setHomeDir(home: string | undefined): void {
  homeDir = home?.replace(/[\\/]+$/, '') || undefined;
}

/** Shortens a home path with "~". */
export function tildePath(path: string, home: string | undefined = homeDir): string {
  if (home && (path === home || path.startsWith(`${home}/`) || path.startsWith(`${home}\\`))) {
    return `~${path.slice(home.length)}`;
  }
  return path.replace(/^\/(Users|home)\/[^/]+/, '~');
}

/** "1 item" / "3 items" */
export function plural(n: number, word: string, pluralWord = `${word}s`): string {
  return `${n.toLocaleString('en-US')} ${n === 1 ? word : pluralWord}`;
}

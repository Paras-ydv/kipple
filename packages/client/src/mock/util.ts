// Small deterministic helpers for fixtures.

export const GB = 1_000_000_000;
export const MB = 1_000_000;
export const DAY = 86_400_000;

export const gb = (x: number) => Math.round(x * GB);

/** Deterministic PRNG (mulberry32). */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** 32 hex chars from a string (FNV-1a, two lanes). Stands in for blake3 ids. */
export function hexId(input: string): string {
  let h1 = 0x811c9dc5;
  let h2 = 0x01000193 ^ 0x5bd1e995;
  for (let i = 0; i < input.length; i++) {
    const c = input.charCodeAt(i);
    h1 = Math.imul(h1 ^ c, 0x01000193) >>> 0;
    h2 = Math.imul(h2 ^ c, 0x5bd1e995) >>> 0;
  }
  const lane = (h: number, salt: number) => {
    let x = (h ^ salt) >>> 0;
    let out = '';
    for (let i = 0; i < 2; i++) {
      x = Math.imul(x ^ (x >>> 13), 0x85ebca6b) >>> 0;
      out += x.toString(16).padStart(8, '0');
    }
    return out;
  };
  return lane(h1, 0x9e3779b9) + lane(h2, 0x7f4a7c15);
}

/** ULID-like id from a timestamp and a seed string. */
export function ulid(ms: number, seed: string): string {
  const alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  let t = ms;
  let time = '';
  for (let i = 0; i < 10; i++) {
    time = alphabet[t % 32] + time;
    t = Math.floor(t / 32);
  }
  const rand = hexId(seed)
    .slice(0, 16)
    .split('')
    .map((c) => alphabet[parseInt(c, 16) * 2])
    .join('');
  return time + rand;
}

/**
 * Splits `total` bytes into `n` positive parts that sum exactly to `total`,
 * with some spread so lists look real.
 */
export function split(total: number, n: number, rand: () => number, spread = 0.8): number[] {
  if (n <= 0) return [];
  const weights = Array.from({ length: n }, () => 1 - spread / 2 + rand() * spread);
  const sum = weights.reduce((s, w) => s + w, 0);
  const parts = weights.map((w) => Math.max(1, Math.floor((w / sum) * total)));
  const diff = total - parts.reduce((s, p) => s + p, 0);
  parts[0] = (parts[0] ?? 0) + diff;
  return parts;
}

export const iso = (ms: number) => new Date(ms).toISOString();

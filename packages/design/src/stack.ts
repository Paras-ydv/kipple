// Geometry of the core sample, the glass stack (docs/UI.md §3.9).
// Shared by the GlassStack component, the share card SVG and the app icon.

export interface StackLayer {
  key: string;
  color: string;
  value: number;
  label?: string;
  /** Clear glass slab (share card "what is left"). */
  clear?: boolean;
}

export interface StackOptions {
  /** Centre x of the stack. */
  cx: number;
  /** Slab width. */
  w: number;
  /** Top of the available area. */
  y0: number;
  /** Bottom of the stack (floor). */
  y1: number;
  /**
   * Fixed pixels per unit of value. When set, the stack is anchored at `y1`
   * and its height follows the values (used to compare stacks side by side).
   * When omitted, slabs fill `y0..y1` exactly.
   */
  scale?: number;
  /** Keep the order given instead of sorting largest first. */
  keepOrder?: boolean;
}

export interface Slab {
  key: string;
  color: string;
  label?: string;
  clear: boolean;
  value: number;
  /** Top of the lid. */
  top: number;
  /** Front height. */
  h: number;
  /** Lid depth. */
  d: number;
  x: number;
  w: number;
  cx: number;
  /** Vertical centre of the slab, for labels. */
  cy: number;
}

export interface StackLayout {
  slabs: Slab[];
  /** Top of the topmost lid. */
  top: number;
  bottom: number;
  d: number;
  gap: number;
  w: number;
  cx: number;
}

export function stackMetrics(w: number) {
  const d = Math.round(w * 0.2);
  return { d, gap: Math.round(d * 0.35), hmin: Math.round(w * 0.08) };
}

/** Lays out slabs bottom-up, largest at the bottom. */
export function layoutStack(layers: StackLayer[], opts: StackOptions): StackLayout {
  const { cx, w, y0, y1 } = opts;
  const { d, gap, hmin } = stackMetrics(w);
  const list = layers.filter((l) => l.value > 0);
  if (!opts.keepOrder) list.sort((a, b) => b.value - a.value);
  const n = list.length;

  let heights: number[];
  if (opts.scale !== undefined) {
    const s = opts.scale;
    heights = list.map((l) => Math.max(hmin, l.value * s));
  } else {
    const avail = y1 - y0 - n * d - Math.max(0, n - 1) * gap;
    heights = fitHeights(
      list.map((l) => l.value),
      avail,
      hmin,
    );
  }

  const slabs: Slab[] = [];
  let y = y1;
  list.forEach((l, i) => {
    const h = heights[i] ?? hmin;
    const top = y - (d + h);
    slabs.push({
      key: l.key,
      color: l.color,
      label: l.label,
      clear: !!l.clear,
      value: l.value,
      top,
      h,
      d,
      x: cx - w / 2,
      w,
      cx,
      cy: top + (d + h) / 2,
    });
    y = top - gap;
  });

  return {
    slabs,
    top: slabs.length ? (slabs[slabs.length - 1] as Slab).top : y1,
    bottom: y1,
    d,
    gap,
    w,
    cx,
  };
}

/** Distributes `avail` px over values with a per-slab minimum. */
function fitHeights(values: number[], avail: number, hmin: number): number[] {
  const n = values.length;
  if (n === 0) return [];
  if (avail <= n * hmin) return values.map(() => Math.max(0, avail / n));
  const fixed = new Array<boolean>(n).fill(false);
  for (;;) {
    const freeSum = values.reduce((s, v, i) => (fixed[i] ? s : s + v), 0);
    const fixedCount = fixed.filter(Boolean).length;
    const s = freeSum > 0 ? (avail - fixedCount * hmin) / freeSum : 0;
    let changed = false;
    values.forEach((v, i) => {
      if (!fixed[i] && v * s < hmin) {
        fixed[i] = true;
        changed = true;
      }
    });
    if (!changed) return values.map((v, i) => (fixed[i] ? hmin : v * s));
  }
}

/** Pixels per unit so the largest of several stacks fits `maxHeight`. */
export function sharedScale(totals: number[], counts: number[], w: number, maxHeight: number): number {
  const { d, gap } = stackMetrics(w);
  let best = Infinity;
  totals.forEach((t, i) => {
    const n = counts[i] ?? 1;
    const room = maxHeight - n * d - Math.max(0, n - 1) * gap;
    if (t > 0) best = Math.min(best, room / t);
  });
  return Number.isFinite(best) ? Math.max(0, best) : 0;
}

// ---- Paint -----------------------------------------------------------------

export interface SlabPaint {
  glow: { cx: number; cy: number; rx: number; ry: number; color: string; opacity: number };
  body: { x: number; y: number; w: number; h: number; r: number; top: string; bottom: string; stroke: string };
  lid: { x: number; y: number; w: number; h: number; r: number; fill: string; stroke: string };
  streak: { x: number; y: number; w: number; h: number };
}

export function paintSlab(s: Slab): SlabPaint {
  const r = Math.min(14, s.w * 0.08);
  const body = {
    x: s.x,
    y: s.top + s.d / 2,
    w: s.w,
    h: s.h + s.d / 2,
    r,
    top: s.clear ? 'rgba(255,255,255,0.08)' : rgba(mix(s.color, '#FFFFFF', 0.2), 0.92),
    bottom: s.clear ? 'rgba(255,255,255,0.04)' : rgba(mix(s.color, '#000000', 0.38), 0.92),
    stroke: s.clear ? 'rgba(255,255,255,0.3)' : 'rgba(255,255,255,0.28)',
  };
  const lid = {
    x: s.x,
    y: s.top,
    w: s.w,
    h: s.d,
    r: s.d / 2,
    fill: s.clear ? 'rgba(255,255,255,0.14)' : rgba(mix(s.color, '#FFFFFF', 0.45), 0.96),
    stroke: s.clear ? 'rgba(255,255,255,0.3)' : 'rgba(255,255,255,0.55)',
  };
  return {
    glow: {
      cx: s.cx,
      cy: s.cy,
      rx: s.w * 0.55,
      ry: (s.d + s.h) * 0.6,
      color: s.color,
      opacity: s.clear ? 0 : 0.28,
    },
    body,
    lid,
    streak: {
      x: s.x + s.w * 0.06,
      y: s.top + s.d + 4,
      w: Math.max(3, s.w * 0.025),
      h: Math.max(0, s.h - s.d / 2 - 10),
    },
  };
}

export function floorShadow(layout: StackLayout) {
  return { cx: layout.cx, cy: layout.bottom + layout.d * 0.35, rx: layout.w * 0.7, ry: layout.d * 0.35 };
}

// ---- Colour helpers --------------------------------------------------------

function parseHex(hex: string): [number, number, number] {
  const h = hex.replace('#', '');
  const full = h.length === 3 ? h.split('').map((c) => c + c).join('') : h;
  const n = parseInt(full.slice(0, 6), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Mixes `a` toward `b` by `t` (0..1); returns hex. */
export function mix(a: string, b: string, t: number): string {
  const [r1, g1, b1] = parseHex(a);
  const [r2, g2, b2] = parseHex(b);
  const c = (x: number, y: number) => Math.round(x + (y - x) * t);
  return `#${[c(r1, r2), c(g1, g2), c(b1, b2)].map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}

export function rgba(hex: string, alpha: number): string {
  const [r, g, b] = parseHex(hex);
  return `rgba(${r},${g},${b},${alpha})`;
}

// ---- SVG string renderer (share card, app icon) ----------------------------

const fmt = (n: number) => String(Math.round(n * 100) / 100);

/** `<defs>` needed by `stackSvg`, with ids prefixed by `id`. */
export function stackDefs(id: string): string {
  return [
    `<filter id="${id}-glow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="18"/></filter>`,
    `<filter id="${id}-floor" x="-50%" y="-200%" width="200%" height="500%"><feGaussianBlur stdDeviation="10"/></filter>`,
    `<linearGradient id="${id}-streak" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".5"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>`,
  ].join('');
}

/** Renders a laid-out stack as SVG markup (bottom slab first). */
export function stackSvg(layout: StackLayout, id: string, opts: { glow?: boolean; floor?: boolean } = {}): string {
  const glow = opts.glow ?? true;
  const glows: string[] = [];
  const parts: string[] = [];
  const defs: string[] = [];
  if (opts.floor ?? true) {
    const f = floorShadow(layout);
    parts.push(
      `<ellipse cx="${fmt(f.cx)}" cy="${fmt(f.cy)}" rx="${fmt(f.rx)}" ry="${fmt(f.ry)}" fill="#000" fill-opacity=".45" filter="url(#${id}-floor)"/>`,
    );
  }
  layout.slabs.forEach((s, i) => {
    const p = paintSlab(s);
    const gid = `${id}-b${i}`;
    defs.push(
      `<linearGradient id="${gid}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${p.body.top}"/><stop offset="1" stop-color="${p.body.bottom}"/></linearGradient>`,
    );
    if (glow && p.glow.opacity > 0) {
      glows.push(
        `<ellipse cx="${fmt(p.glow.cx)}" cy="${fmt(p.glow.cy)}" rx="${fmt(p.glow.rx)}" ry="${fmt(p.glow.ry)}" fill="${p.glow.color}" fill-opacity="${p.glow.opacity}" filter="url(#${id}-glow)"/>`,
      );
    }
    parts.push(
      `<rect x="${fmt(p.body.x)}" y="${fmt(p.body.y)}" width="${fmt(p.body.w)}" height="${fmt(p.body.h)}" rx="${fmt(p.body.r)}" fill="url(#${gid})" stroke="${p.body.stroke}" stroke-width="1"/>`,
      `<rect x="${fmt(p.lid.x)}" y="${fmt(p.lid.y)}" width="${fmt(p.lid.w)}" height="${fmt(p.lid.h)}" rx="${fmt(p.lid.r)}" fill="${p.lid.fill}" stroke="${p.lid.stroke}" stroke-width="1"/>`,
    );
    if (p.streak.h > 0) {
      parts.push(
        `<rect x="${fmt(p.streak.x)}" y="${fmt(p.streak.y)}" width="${fmt(p.streak.w)}" height="${fmt(p.streak.h)}" rx="2" fill="url(#${id}-streak)"/>`,
      );
    }
  });
  // Floor first, then every glow, then the slabs bottom-up.
  const [floor, ...slabs] = opts.floor ?? true ? parts : ['', ...parts];
  return `<defs>${defs.join('')}</defs>${floor}${glows.join('')}${slabs.join('')}`;
}

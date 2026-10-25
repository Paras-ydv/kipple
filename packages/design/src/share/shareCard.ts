// Reference implementation of the share card SVG (docs/UI.md §5).
// kipple-share renders the same SVG with resvg; keep the two in step.
// resvg does not apply variable-font axes, so the SVG names the static Mona Sans
// families first (packages/design/fonts/*.ttf); browsers fall back to the
// variable 'Mona Sans' with font-stretch.

import { formatBytes } from '../format.ts';
import { layoutStack, stackDefs, stackSvg, type StackLayer } from '../stack.ts';
import { categories, type CategoryId } from '../tokens.ts';

export const SHARE_WIDTH = 1200;
export const SHARE_HEIGHT = 630;
export const SHARE_URL = 'github.com/Paras-ydv/kipple';

export type Device = 'Mac' | 'PC' | 'computer';

export interface ShareCategory {
  id: CategoryId;
  /** Reclaimable bytes found (found variant) or bytes freed (reclaimed variant). */
  bytes: number;
}

export interface ShareCardInput {
  variant: 'found' | 'reclaimed';
  /** Total found, or total freed. */
  total: number;
  device: Device;
  categories: ShareCategory[];
  /** Reclaimed variant: bytes still left after the clean. */
  remaining?: number;
  /** Text width in px for a CSS font shorthand; defaults to an estimate. */
  measure?: (text: string, font: string) => number;
}

export function deviceForPlatform(os: string): Device {
  if (os === 'macos') return 'Mac';
  if (os === 'windows') return 'PC';
  return 'computer';
}

const esc = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

function estimate(text: string, font: string): number {
  const size = Number(/(\d+(?:\.\d+)?)px/.exec(font)?.[1] ?? 16);
  const bold = /\b(7|8|9)00\b/.test(font);
  return text.length * size * (bold ? 0.6 : 0.53);
}

export function shareHeadline(input: Pick<ShareCardInput, 'variant' | 'total' | 'device'>): [string, string] {
  const size = formatBytes(input.total);
  return input.variant === 'found'
    ? [`My ${input.device} was hiding`, `${size} of kipple.`]
    : ['I just reclaimed', `${size} of kipple.`];
}

export function renderShareCardSvg(input: ShareCardInput): string {
  const measure = input.measure ?? estimate;
  const id = 'kc';
  const [line1, line2] = shareHeadline(input);

  // Headline steps down 4 px until both lines fit 600 px (minimum 48).
  let size = 64;
  const headFont = (s: number) => `800 ${s}px "Mona Sans"`;
  while (size > 48 && Math.max(measure(line1, headFont(size)), measure(line2, headFont(size))) > 600) size -= 4;
  const base1 = 196;
  const base2 = base1 + Math.round(size * 1.02);

  // Chips: three largest categories.
  const top3 = [...input.categories].filter((c) => c.bytes > 0).sort((a, b) => b.bytes - a.bytes).slice(0, 3);
  let x = 64;
  const chipY = 496;
  const chips = top3
    .map((c) => {
      const tok = categories[c.id];
      const value = formatBytes(c.bytes);
      const w = 33 + measure(value, '700 17px "Mona Sans"') + measure(` ${tok.share}`, '400 17px "Mona Sans"') + 14;
      const svg = [
        `<rect x="${x}" y="${chipY}" width="${w.toFixed(1)}" height="42" rx="18" fill="url(#${id}-chip)" stroke="rgba(255,255,255,.18)" stroke-width="1"/>`,
        `<line x1="${x + 18}" y1="${chipY + 1}" x2="${(x + w - 18).toFixed(1)}" y2="${chipY + 1}" stroke="rgba(255,255,255,.4)" stroke-width="1"/>`,
        `<circle cx="${x + 19}" cy="${chipY + 21}" r="5" fill="${tok.color}"/>`,
        `<text x="${x + 33}" y="${chipY + 27}" font-size="17" fill="#F4F5F8"><tspan font-weight="700">${esc(value)}</tspan><tspan font-weight="400"> ${esc(tok.share)}</tspan></text>`,
      ].join('');
      x += w + 10;
      return svg;
    })
    .join('');

  // Stack on the right.
  const layers: StackLayer[] = input.categories
    .filter((c) => c.bytes > 0)
    .sort((a, b) => b.bytes - a.bytes)
    .map((c) => ({ key: c.id, color: categories[c.id].color, value: c.bytes }));
  if (input.variant === 'reclaimed' && (input.remaining ?? 0) > 0) {
    layers.push({ key: 'left', color: '#FFFFFF', value: input.remaining ?? 0, clear: true });
  }
  const layout = layoutStack(layers, { cx: 930, w: 240, y0: 70, y1: 556, keepOrder: input.variant === 'reclaimed' });

  const glow = (cx: number, cy: number, r: number, color: string, op: number, n: string) =>
    `<radialGradient id="${id}-${n}" cx="${cx}" cy="${cy}" r="${r}" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="${color}" stop-opacity="${op}"/><stop offset="1" stop-color="${color}" stop-opacity="0"/></radialGradient>`;

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${SHARE_WIDTH}" height="${SHARE_HEIGHT}" viewBox="0 0 ${SHARE_WIDTH} ${SHARE_HEIGHT}" font-family="'Mona Sans', sans-serif">`,
    '<defs>',
    glow(930, 300, 460, '#347CFF', 0.42, 'g1'),
    glow(1120, 640, 420, '#B25CFF', 0.38, 'g2'),
    glow(760, 640, 360, '#FF8454', 0.26, 'g3'),
    glow(40, -40, 520, '#347CFF', 0.16, 'g4'),
    `<linearGradient id="${id}-chip" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".12"/><stop offset="1" stop-color="#fff" stop-opacity=".04"/></linearGradient>`,
    stackDefs(`${id}-s`),
    '</defs>',
    `<rect width="${SHARE_WIDTH}" height="${SHARE_HEIGHT}" fill="#06080F"/>`,
    ['g1', 'g2', 'g3', 'g4'].map((g) => `<rect width="${SHARE_WIDTH}" height="${SHARE_HEIGHT}" fill="url(#${id}-${g})"/>`).join(''),
    `<text x="64" y="86" font-family="'Mona Sans Expanded', 'Mona Sans', sans-serif" font-size="26" font-weight="800" font-stretch="125%" letter-spacing="-0.78" fill="#F4F5F8">kipple</text>`,
    `<text font-family="'Mona Sans Display SemiExpanded', 'Mona Sans', sans-serif" font-size="${size}" font-weight="800" font-stretch="108%" letter-spacing="${(-0.04 * size).toFixed(2)}" fill="#F4F5F8">`,
    `<tspan x="64" y="${base1}">${esc(line1)}</tspan><tspan x="64" y="${base2}">${esc(line2)}</tspan></text>`,
    chips,
    `<text x="64" y="574" font-size="16" font-weight="400" fill="rgba(255,255,255,.62)">Found with kipple, an open-source disk cleaner for developers. <tspan fill="#FFFFFF">${SHARE_URL}</tspan></text>`,
    stackSvg(layout, `${id}-s`),
    '</svg>',
  ].join('');
}

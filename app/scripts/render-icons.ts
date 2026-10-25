// Renders the app icon, tray icon and favicon from the AppIcon board design
// (docs/UI.md §3.10). Run with: pnpm --filter app icons
// Writes src-tauri/icons/{icon.svg,source.png,tray.png} and public/favicon.png;
// `tauri icon` then makes every platform size from source.png.

import { Resvg } from '@resvg/resvg-js';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { layoutStack, stackDefs, stackSvg } from '../../packages/design/src/stack.ts';

const here = dirname(fileURLToPath(import.meta.url));
const icons = resolve(here, '../src-tauri/icons');
const pub = resolve(here, '../public');
mkdirSync(icons, { recursive: true });
mkdirSync(pub, { recursive: true });

function appIconSvg(): string {
  const layout = layoutStack(
    [
      { key: 'container', color: '#2F6BFF', value: 40 },
      { key: 'ai', color: '#8B5CF6', value: 28 },
      { key: 'cache', color: '#FFB020', value: 20 },
      { key: 'artifact', color: '#3CCB5A', value: 14 },
    ],
    { cx: 512, w: 360, y0: 250, y1: 800 },
  );
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">',
    '<defs>',
    '<clipPath id="sq"><rect x="100" y="100" width="824" height="824" rx="185"/></clipPath>',
    '<linearGradient id="ring" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#FF8454"/><stop offset=".5" stop-color="#FF5C8A"/><stop offset="1" stop-color="#B25CFF"/></linearGradient>',
    '<filter id="ringGlow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="30"/></filter>',
    '<radialGradient id="bg" cx="512" cy="420" r="560" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#1A1C4A"/><stop offset="1" stop-color="#0A0B26"/></radialGradient>',
    '<linearGradient id="rim" x1="0" y1="0" x2="0.62" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".34"/><stop offset=".32" stop-color="#fff" stop-opacity="0"/><stop offset=".7" stop-color="#fff" stop-opacity="0"/><stop offset="1" stop-color="#fff" stop-opacity=".12"/></linearGradient>',
    stackDefs('st'),
    '</defs>',
    '<g clip-path="url(#sq)">',
    '<rect x="100" y="100" width="824" height="824" fill="url(#bg)"/>',
    '<circle cx="512" cy="470" r="250" fill="none" stroke="url(#ring)" stroke-width="46" opacity=".7" filter="url(#ringGlow)"/>',
    '<circle cx="512" cy="470" r="250" fill="none" stroke="url(#ring)" stroke-width="46"/>',
    stackSvg(layout, 'st'),
    '<rect x="100" y="100" width="824" height="824" fill="url(#rim)"/>',
    '</g>',
    '<rect x="100.75" y="100.75" width="822.5" height="822.5" rx="184.5" fill="none" stroke="#fff" stroke-opacity=".18" stroke-width="1.5"/>',
    '<path d="M285 101.5 H739" stroke="#fff" stroke-opacity=".6" stroke-width="1.5" stroke-linecap="round"/>',
    '</svg>',
  ].join('');
}

/** Template image for the macOS menu bar: the three-slab glyph, black on transparent. */
function traySvg(size: number): string {
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 22 22">`,
    `<g transform="translate(6 3)">`,
    '<rect x="0.5" y="1.5" width="9" height="2.6" rx="1" fill="#000" fill-opacity=".7"/>',
    '<rect x="0.5" y="5.6" width="9" height="3.4" rx="1" fill="#000" fill-opacity=".85"/>',
    '<rect x="0.5" y="10.5" width="9" height="4.5" rx="1.2" fill="#000"/>',
    '</g></svg>',
  ].join('');
}

function png(svg: string, width: number): Buffer {
  return new Resvg(svg, { fitTo: { mode: 'width', value: width } }).render().asPng();
}

const icon = appIconSvg();
writeFileSync(resolve(icons, 'icon.svg'), icon);
writeFileSync(resolve(icons, 'source.png'), png(icon, 1024));
writeFileSync(resolve(icons, 'tray.png'), png(traySvg(44), 44));
writeFileSync(resolve(pub, 'favicon.png'), png(icon, 64));
console.log('Wrote icon.svg, source.png, tray.png and favicon.png');

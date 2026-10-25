// Desktop-only actions served by app/src-tauri (not part of the core API).
// In the browser preview they fall back to something sensible.

import { invoke } from '@tauri-apps/api/core';
import { native } from './client.ts';
import { ui } from './stores/ui.svelte.ts';

/** Shows a path in Finder, Explorer or the file manager. */
export async function reveal(path: string, label: string) {
  if (!native) {
    ui.toast(`${label} works in the desktop app.`);
    return;
  }
  try {
    await invoke('reveal', { path });
  } catch {
    ui.toast(`Could not open ${path}.`);
  }
}

/** From the menu bar popover: shows the main window on a route. */
export async function openMain(hash: string) {
  if (!native) {
    window.location.href = `./index.html${hash}`;
    return;
  }
  await invoke('show_main', { route: hash });
}

/** Hides the menu bar popover window. */
export async function hideTray() {
  if (native) await invoke('hide_tray').catch(() => undefined);
}

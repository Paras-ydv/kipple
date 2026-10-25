// Global shortcuts (docs/UI.md §3.0).

import { NAV, router, type Route } from './router.svelte.ts';
import { data } from './stores/data.svelte.ts';
import { ui } from './stores/ui.svelte.ts';

function typing(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  return el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName);
}

export function handleGlobalKey(e: KeyboardEvent) {
  const mod = e.metaKey || e.ctrlKey;
  if (e.key === 'Escape' && ui.anyDialog) {
    e.preventDefault();
    ui.closeTop();
    return;
  }
  if (mod && !e.shiftKey && !e.altKey && /^[1-6]$/.test(e.key)) {
    const item = NAV[Number(e.key) - 1];
    if (item) {
      e.preventDefault();
      router.go({ name: item.name } as Route);
    }
    return;
  }
  if (mod && e.key.toLowerCase() === 'r') {
    e.preventDefault();
    void data.scan();
    return;
  }
  if ((mod && e.key.toLowerCase() === 'f') || (e.key === '/' && !typing(e.target))) {
    e.preventDefault();
    ui.searchRequest++;
  }
}

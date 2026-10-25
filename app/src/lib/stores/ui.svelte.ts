// Transient UI state: the explain sheet, the confirmation dialog, toasts.

import type { Confirmation, Item, Plan } from '@kipple/client';

export interface ConfirmRequest {
  /** The plan to run; absent for actions that aren't plans (freeing quarantine). */
  plan: Plan | null;
  /** Sentences shown instead of the plan summary. */
  lines?: string[];
  /** Free quarantined bytes right away (no undo). */
  purgeNow: boolean;
  /** What the confirm button says, e.g. "Reclaim 76.5 GB". */
  action: string;
  resolve: (c: Confirmation | null) => void;
}

export interface Toast {
  id: number;
  text: string;
}

class UiStore {
  /** Items shown in the explain sheet (a single item or one group). */
  explain: { title: string; items: Item[] } | null = $state.raw(null);
  confirm: ConfirmRequest | null = $state.raw(null);
  selectionOpen = $state(false);
  shareOpen = $state(false);
  toasts: Toast[] = $state([]);
  /** Bumped to ask the current screen to focus its search field. */
  searchRequest = $state(0);
  private toastId = 0;

  openExplain(title: string, items: Item[]) {
    this.explain = { title, items };
  }

  closeExplain() {
    this.explain = null;
  }

  /** Opens the confirmation dialog; resolves with the confirmation or null. */
  askConfirm(plan: Plan, action: string, purgeNow = false): Promise<Confirmation | null> {
    return new Promise((resolve) => {
      this.confirm = {
        plan,
        purgeNow,
        action,
        resolve: (c) => {
          this.confirm = null;
          resolve(c);
        },
      };
    });
  }

  /** Typed confirmation for an action that isn't a plan. */
  askConfirmText(action: string, lines: string[]): Promise<boolean> {
    return new Promise((resolve) => {
      this.confirm = {
        plan: null,
        lines,
        purgeNow: true,
        action,
        resolve: (c) => {
          this.confirm = null;
          resolve(c !== null);
        },
      };
    });
  }

  toast(text: string) {
    const id = ++this.toastId;
    this.toasts = [...this.toasts, { id, text }];
    setTimeout(() => {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, 4000);
  }

  get anyDialog(): boolean {
    return !!(this.explain || this.confirm || this.selectionOpen || this.shareOpen);
  }

  closeTop() {
    if (this.confirm) this.confirm.resolve(null);
    else if (this.shareOpen) this.shareOpen = false;
    else if (this.selectionOpen) this.selectionOpen = false;
    else if (this.explain) this.explain = null;
  }
}

export const ui = new UiStore();

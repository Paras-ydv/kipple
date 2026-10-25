<script lang="ts">
  // Dialog or side sheet with a focus trap. Escape is handled globally (keyboard.ts).
  import type { Snippet } from 'svelte';
  import { fade, fly } from 'svelte/transition';

  interface Props {
    label: string;
    kind?: 'dialog' | 'sheet';
    width?: number;
    onclose: () => void;
    children: Snippet;
  }

  let { label, kind = 'dialog', width = 420, onclose, children }: Props = $props();
  let panel: HTMLElement | undefined = $state();

  const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])';

  $effect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const first = panel?.querySelector<HTMLElement>('[data-autofocus]') ?? panel?.querySelector<HTMLElement>(FOCUSABLE);
    first?.focus();
    return () => previous?.focus?.();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !panel) return;
    const items = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
    if (!items.length) return;
    const first = items[0] as HTMLElement;
    const last = items[items.length - 1] as HTMLElement;
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

<div class="backdrop {kind}" transition:fade={{ duration: 120 }}>
  <button type="button" class="scrim" aria-label="Close" tabindex="-1" onclick={onclose}></button>
  <div
    bind:this={panel}
    class="panel k-popover {kind}"
    role="dialog"
    aria-modal="true"
    aria-label={label}
    tabindex="-1"
    style:width="{width}px"
    {onkeydown}
    transition:fly={{ x: kind === 'sheet' ? 24 : 0, y: kind === 'dialog' ? 8 : 0, duration: 150 }}
  >
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 50;
    display: flex;
    border-radius: inherit;
  }
  .backdrop.dialog {
    align-items: center;
    justify-content: center;
  }
  .backdrop.sheet {
    justify-content: flex-end;
    padding: 10px;
  }
  .scrim {
    position: absolute;
    inset: 0;
    border: 0;
    padding: 0;
    background: rgba(4, 6, 14, 0.42);
    border-radius: inherit;
    cursor: default;
  }
  .panel {
    position: relative;
    border-radius: var(--k-r-popover);
    padding: 20px;
    max-height: calc(100% - 40px);
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .panel.sheet {
    max-height: 100%;
    height: 100%;
  }
  .panel:focus {
    outline: none;
  }
</style>

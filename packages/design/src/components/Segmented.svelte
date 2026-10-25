<script lang="ts" module>
  export interface Segment<V extends string> {
    value: V;
    label: string;
    /** Trailing detail, e.g. a size. */
    detail?: string;
  }
</script>

<script lang="ts" generics="T extends string">
  interface Props {
    segments: Segment<T>[];
    value: T;
    label: string;
    onchange?: (value: T) => void;
    /** Selected weight (600 on Projects, 650 on AI history). */
    selectedWeight?: number;
  }

  let { segments, value = $bindable(), label, onchange, selectedWeight = 600 }: Props = $props();

  let buttons: HTMLButtonElement[] = $state([]);

  function select(v: T, focus = false) {
    value = v;
    onchange?.(v);
    if (focus) {
      const i = segments.findIndex((s) => s.value === v);
      buttons[i]?.focus();
    }
  }

  function onkeydown(e: KeyboardEvent) {
    const i = segments.findIndex((s) => s.value === value);
    let next: number;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = (i + 1) % segments.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = (i - 1 + segments.length) % segments.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = segments.length - 1;
    else return;
    e.preventDefault();
    const seg = segments[next];
    if (seg) select(seg.value, true);
  }
</script>

<div class="segmented k-clear" role="tablist" aria-label={label} tabindex="-1" {onkeydown}>
  {#each segments as seg, i (seg.value)}
    {@const selected = seg.value === value}
    <button
      bind:this={buttons[i]}
      type="button"
      role="tab"
      aria-selected={selected}
      tabindex={selected ? 0 : -1}
      class:k-lens={selected}
      style:font-weight={selected ? selectedWeight : 500}
      onclick={() => select(seg.value)}
    >
      {seg.label}
      {#if seg.detail}<span class="detail">{seg.detail}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: flex;
    height: 36px;
    border-radius: 18px;
    padding: 3px;
    gap: 2px;
  }
  button {
    border: 0;
    background: transparent;
    height: 30px;
    padding: 0 14px;
    border-radius: 15px;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.8);
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    transition: background var(--k-dur-response) ease-out;
  }
  button[aria-selected='true'] {
    color: var(--k-text);
  }
  button:hover:not([aria-selected='true']) {
    background: rgba(255, 255, 255, 0.06);
  }
  .detail {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.6);
  }
</style>

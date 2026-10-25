<script lang="ts">
  // Static glass stacks for the site, rendered at build time (no client JS).
  import { StackSlabs, categoryColor, layoutStack, sharedScale, type StackLayer } from '@kipple/design';

  interface Stack {
    key: string;
    layers: { category: string; bytes: number }[];
    dull?: boolean;
  }

  interface Props {
    /** One stack (hero) or several (shelf). */
    stacks: Stack[];
    width: number;
    height: number;
    slab: number;
    /** Floor y for shelves; the hero fills top to floor. */
    floor: number;
    top: number;
    /** Stack centres; defaults to evenly spaced. */
    centres?: number[];
    label: string;
    class?: string;
  }

  let { stacks, width, height, slab, floor, top, centres, label, class: className = '' }: Props = $props();

  const layouts = $derived.by(() => {
    const layers = stacks.map((s) =>
      s.layers.map((l): StackLayer => ({ key: l.category, color: categoryColor(l.category), value: l.bytes })),
    );
    const totals = stacks.map((s) => s.layers.reduce((a, l) => a + l.bytes, 0));
    const scale = stacks.length > 1 ? sharedScale(totals, layers.map((l) => l.length), slab, floor - top) : undefined;
    return stacks.map((s, i) => {
      const cx = centres?.[i] ?? (width / (stacks.length + 1)) * (i + 1);
      return { s, layout: layoutStack(layers[i] ?? [], { cx, w: slab, y0: top, y1: floor, scale }) };
    });
  });
</script>

<svg class={className} viewBox="0 0 {width} {height}" role="img" aria-label={label} preserveAspectRatio="xMidYMid meet">
  {#each layouts as { s, layout } (s.key)}
    <StackSlabs {layout} dull={s.dull} />
  {/each}
</svg>

<style>
  svg {
    display: block;
    overflow: visible;
  }
</style>

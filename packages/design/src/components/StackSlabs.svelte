<script lang="ts">
  // One glass stack as an SVG group (docs/UI.md §3.9). Place inside an <svg>.
  import { cubicOut } from 'svelte/easing';
  import { floorShadow, paintSlab, type StackLayout } from '../stack.ts';

  interface Props {
    layout: StackLayout;
    /** Untouched projects: desaturated and darker. */
    dull?: boolean;
    glow?: boolean;
    floor?: boolean;
    /** Animate slabs that appear (scan in progress). */
    grow?: boolean;
  }

  let { layout, dull = false, glow = true, floor = true, grow = false }: Props = $props();

  const uid = $props.id();
  const painted = $derived(layout.slabs.map((s) => ({ s, p: paintSlab(s) })));
  const shadow = $derived(floorShadow(layout));

  function reducedMotion(): boolean {
    return typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches;
  }

  function growIn(_node: Element) {
    const duration = grow && !reducedMotion() ? 240 : 0;
    return {
      duration,
      easing: cubicOut,
      css: (t: number) => `transform: scaleY(${t}); opacity: ${Math.min(1, t * 1.5)};`,
    };
  }
</script>

<g class="stack" filter={dull ? `url(#${uid}-dull)` : undefined}>
  <defs>
    <filter id="{uid}-glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="18" />
    </filter>
    <filter id="{uid}-floor" x="-50%" y="-200%" width="200%" height="500%">
      <feGaussianBlur stdDeviation="10" />
    </filter>
    <filter id="{uid}-dull" color-interpolation-filters="sRGB">
      <feColorMatrix type="saturate" values="0.25" />
      <feComponentTransfer>
        <feFuncR type="linear" slope="0.7" />
        <feFuncG type="linear" slope="0.7" />
        <feFuncB type="linear" slope="0.7" />
      </feComponentTransfer>
    </filter>
    <linearGradient id="{uid}-streak" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#fff" stop-opacity=".5" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    {#each painted as { s, p }, i (s.key)}
      <linearGradient id="{uid}-b{i}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color={p.body.top} />
        <stop offset="1" stop-color={p.body.bottom} />
      </linearGradient>
    {/each}
  </defs>

  {#if floor && layout.slabs.length}
    <ellipse
      cx={shadow.cx}
      cy={shadow.cy}
      rx={shadow.rx}
      ry={shadow.ry}
      fill="#000"
      fill-opacity=".45"
      filter="url(#{uid}-floor)"
    />
  {/if}

  {#if glow}
    {#each painted as { s, p } (s.key)}
      {#if p.glow.opacity > 0}
        <ellipse
          class="geo"
          cx={p.glow.cx}
          cy={p.glow.cy}
          rx={p.glow.rx}
          ry={p.glow.ry}
          fill={p.glow.color}
          fill-opacity={p.glow.opacity}
          filter="url(#{uid}-glow)"
        />
      {/if}
    {/each}
  {/if}

  {#each painted as { s, p }, i (s.key)}
    <g class="slab" in:growIn>
      <rect
        class="geo"
        x={p.body.x}
        y={p.body.y}
        width={p.body.w}
        height={p.body.h}
        rx={p.body.r}
        fill="url(#{uid}-b{i})"
        stroke={p.body.stroke}
        stroke-width="1"
      />
      <rect
        class="geo"
        x={p.lid.x}
        y={p.lid.y}
        width={p.lid.w}
        height={p.lid.h}
        rx={p.lid.r}
        fill={p.lid.fill}
        stroke={p.lid.stroke}
        stroke-width="1"
      />
      {#if p.streak.h > 0}
        <rect
          class="geo"
          x={p.streak.x}
          y={p.streak.y}
          width={p.streak.w}
          height={p.streak.h}
          rx="2"
          fill="url(#{uid}-streak)"
        />
      {/if}
    </g>
  {/each}
</g>

<style>
  .slab {
    transform-box: fill-box;
    transform-origin: 50% 100%;
  }
  .geo {
    transition:
      y var(--k-dur-grow) var(--k-ease-grow),
      height var(--k-dur-grow) var(--k-ease-grow),
      cy var(--k-dur-grow) var(--k-ease-grow),
      ry var(--k-dur-grow) var(--k-ease-grow);
  }
</style>

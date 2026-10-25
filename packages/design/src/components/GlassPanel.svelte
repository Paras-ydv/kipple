<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Material = 'glass' | 'clear' | 'chip' | 'popover' | 'amber' | 'window';

  type Props = {
    material?: Material;
    radius?: number;
    padding?: string;
    tag?: 'div' | 'section' | 'aside' | 'nav' | 'article' | 'header' | 'li';
    children: Snippet;
  } & Omit<HTMLAttributes<HTMLElement>, 'children'>;

  let {
    material = 'glass',
    radius = 20,
    padding = '16px 18px',
    tag = 'div',
    children,
    class: className = '',
    ...rest
  }: Props = $props();
</script>

<svelte:element
  this={tag}
  class="panel k-{material} {className}"
  style:border-radius="{radius}px"
  style:padding
  {...rest}
>
  {@render children()}
</svelte:element>

<style>
  .panel {
    position: relative;
    min-width: 0;
  }
</style>

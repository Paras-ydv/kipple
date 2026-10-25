<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'tint' | 'clear' | 'warm' | 'plain';
  type Size = 'sm' | 'md' | 'lg' | 'xl';

  type Props = {
    variant?: Variant;
    size?: Size;
    /** Stretch to the container width. */
    block?: boolean;
    /** Weight override; defaults follow the design per variant and size. */
    weight?: number;
    /** Height override in px (the design uses 36, 38 and 42 in places). */
    height?: number;
    fontSize?: number;
    /** Horizontal padding in px. */
    pad?: number;
    href?: string;
    children: Snippet;
  } & Omit<HTMLButtonAttributes, 'children'> &
    Omit<HTMLAnchorAttributes, 'children'>;

  let {
    variant = 'clear',
    size = 'md',
    block = false,
    weight,
    height,
    fontSize,
    pad,
    href,
    children,
    class: className = '',
    type = 'button',
    ...rest
  }: Props = $props();

  const heights: Record<Size, number> = { sm: 32, md: 40, lg: 44, xl: 48 };
  const fontSizes: Record<Size, number> = { sm: 12, md: 13, lg: 15, xl: 15 };
  const defaultWeight = $derived(
    weight ?? (variant === 'tint' || variant === 'warm' ? (size === 'xl' ? 700 : 650) : size === 'lg' ? 550 : 500),
  );
</script>

{#if href}
  <a
    {href}
    class="btn k-{variant} {className}"
    class:block
    style:height="{height ?? heights[size]}px"
    style:font-size="{fontSize ?? fontSizes[size]}px"
    style:font-weight={defaultWeight}
    style:padding-inline={pad !== undefined ? `${pad}px` : undefined}
    {...rest as HTMLAnchorAttributes}>{@render children()}</a
  >
{:else}
  <button
    {type}
    class="btn k-{variant} {className}"
    class:block
    style:height="{height ?? heights[size]}px"
    style:font-size="{fontSize ?? fontSizes[size]}px"
    style:font-weight={defaultWeight}
    style:padding-inline={pad !== undefined ? `${pad}px` : undefined}
    {...rest as HTMLButtonAttributes}>{@render children()}</button
  >
{/if}

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 0 18px;
    border-radius: 999px;
    color: var(--k-text);
    text-decoration: none;
    white-space: nowrap;
    cursor: pointer;
    transition:
      filter var(--k-dur-response) ease-out,
      transform var(--k-dur-response) ease-out,
      opacity var(--k-dur-response) ease-out;
  }
  .btn.k-tint {
    color: #fff;
  }
  .btn.k-plain {
    background: transparent;
    border: 0;
  }
  .block {
    display: flex;
    width: 100%;
  }
  .btn:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .btn:active:not(:disabled) {
    transform: translateY(0.5px);
    filter: brightness(0.96);
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>

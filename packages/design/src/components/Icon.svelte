<script lang="ts">
  import { iconPaths, type IconName } from '../icons/paths.ts';

  interface Props {
    name: IconName;
    size?: number;
    stroke?: string;
    strokeWidth?: number;
    /** Accessible name; when omitted the icon is decorative. */
    label?: string;
  }

  let { name, size = 16, stroke = 'currentColor', strokeWidth = 2, label }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  {stroke}
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  focusable="false"
>
  <path d={iconPaths[name]} />
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>

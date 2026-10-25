<script lang="ts">
  interface Props {
    color: string;
    size?: number;
    shape?: 'round' | 'square' | 'bar';
    /** A CSS background other than a flat colour (the kipple swatch). */
    background?: string;
    outline?: boolean;
  }

  let { color, size = 8, shape = 'round', background, outline = false }: Props = $props();
</script>

<span
  class="dot {shape}"
  class:outline
  aria-hidden="true"
  style:width="{shape === 'bar' ? Math.max(3, size / 2.5) : size}px"
  style:height="{size}px"
  style:background={outline ? 'transparent' : (background ?? color)}
  style:border-color={outline ? color : undefined}
></span>

<style>
  .dot {
    display: inline-block;
    flex: none;
  }
  .round {
    border-radius: 50%;
  }
  .square {
    border-radius: 2px;
  }
  .bar {
    border-radius: 1px;
  }
  .outline {
    border: 1px solid;
  }
</style>

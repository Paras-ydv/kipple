<script lang="ts">
  interface Props {
    size?: number;
    tag?: 'span' | 'a';
    href?: string;
  }

  let { size = 17, tag = 'span', href }: Props = $props();
</script>

<svelte:element
  this={tag}
  {href}
  class="wordmark"
  style:font-size="{size}px"
  style:letter-spacing={size >= 22 ? '-0.03em' : '-0.02em'}>kipple</svelte:element
>

<style>
  .wordmark {
    font-family: var(--k-font-brand);
    font-stretch: 125%;
    font-weight: 800;
    line-height: 1.2;
    color: var(--k-text);
    text-decoration: none;
  }
</style>

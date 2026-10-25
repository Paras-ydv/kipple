<script lang="ts">
  // Inline state panel: loading, empty, error, or "not available yet".
  import { Button, GlassPanel } from '@kipple/design';

  interface Props {
    title: string;
    body?: string;
    action?: { label: string; onclick: () => void };
  }

  let { title, body, action }: Props = $props();
</script>

<div class="wrap">
  <GlassPanel radius={20} padding="22px 24px" role="status">
    <div class="content">
      <p class="title">{title}</p>
      {#if body}<p class="body">{body}</p>{/if}
      {#if action}
        <div><Button variant="clear" size="sm" onclick={action.onclick}>{action.label}</Button></div>
      {/if}
    </div>
  </GlassPanel>
</div>

<style>
  .wrap {
    flex: 1;
    display: grid;
    place-items: center;
  }
  .content {
    max-width: 44ch;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  p {
    margin: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 650;
  }
  .body {
    font-size: 13px;
    line-height: 1.5;
    color: var(--k-text-3);
  }
</style>

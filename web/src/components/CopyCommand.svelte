<script lang="ts">
  // The only interactive bit of the site: copies the install command.
  import { Icon } from '@kipple/design';

  interface Props {
    command: string;
  }
  let { command }: Props = $props();
  let copied = $state(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      copied = false;
    }
  }
</script>

<button type="button" class="copy k-glass" onclick={copy} aria-label="Copy {command}">
  <span class="cmd">{command}</span>
  <span class="state" aria-live="polite">
    {#if copied}Copied{:else}<Icon name="copy" size={15} stroke="#fff" />{/if}
  </span>
</button>

<style>
  .copy {
    height: 46px;
    padding: 0 16px;
    border-radius: 23px;
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    color: inherit;
  }
  .cmd {
    font-family: var(--k-font-mono);
  }
  .state {
    display: inline-flex;
    min-width: 15px;
    font-size: 12px;
    color: var(--k-text-2);
  }
</style>

<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    value?: string;
    label: string;
    placeholder?: string;
    width?: number;
    input?: HTMLInputElement;
  }

  let { value = $bindable(''), label, placeholder = 'Search', width = 190, input = $bindable() }: Props = $props();

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && value) {
      e.stopPropagation();
      value = '';
    }
  }
</script>

<label class="search k-clear" style:width="{width}px">
  <Icon name="search" size={15} stroke="rgba(255,255,255,.7)" />
  <input bind:this={input} bind:value type="search" {placeholder} aria-label={label} {onkeydown} />
</label>

<style>
  .search {
    height: 36px;
    border-radius: 18px;
    padding: 0 14px;
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
  .search:focus-within {
    outline: 2px solid var(--k-focus);
    outline-offset: 2px;
  }
  input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    outline: none;
    color: #fff;
    font-size: 13px;
  }
  input::placeholder {
    color: var(--k-text-4);
  }
  input::-webkit-search-cancel-button {
    filter: invert(1) opacity(0.6);
  }
</style>

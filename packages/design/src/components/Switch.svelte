<script lang="ts">
  interface Props {
    checked?: boolean;
    label: string;
    disabled?: boolean;
    onchange?: (checked: boolean) => void;
  }

  let { checked = $bindable(false), label, disabled = false, onchange }: Props = $props();
</script>

<input
  type="checkbox"
  role="switch"
  aria-label={label}
  aria-checked={checked}
  bind:checked
  {disabled}
  onchange={() => onchange?.(checked)}
/>

<style>
  input {
    appearance: none;
    -webkit-appearance: none;
    flex: none;
    width: 42px;
    height: 26px;
    border-radius: 13px;
    background: rgba(255, 255, 255, 0.16);
    box-shadow:
      inset 0 1px 2px rgba(0, 0, 0, 0.35),
      inset 0 0 0 1px rgba(255, 255, 255, 0.12);
    position: relative;
    margin: 0;
    cursor: pointer;
    transition: background var(--k-dur-response) ease-out;
  }
  input::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.35);
    transition: transform var(--k-dur-response) ease-out;
  }
  input:checked {
    background: var(--k-accent);
  }
  input:checked::after {
    transform: translateX(16px);
  }
  input:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>

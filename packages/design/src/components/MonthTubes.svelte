<script lang="ts">
  interface Month {
    label: string;
    /** Value in GB. */
    gb: number;
    current?: boolean;
  }

  interface Props {
    months: Month[];
    label: string;
    tubeHeight?: number;
  }

  let { months, label, tubeHeight = 160 }: Props = $props();
  const peak = $derived(Math.max(0, ...months.map((m) => m.gb)));
  const max = $derived((peak > 0 ? peak : 1) * 1.08);
  // One unit for every label, picked from the largest month.
  const unit = $derived(peak >= 1 ? { div: 1, digits: 1 } : peak >= 0.001 ? { div: 0.001, digits: 0 } : { div: 0.000001, digits: 0 });
  const unitName = $derived(peak >= 1 ? 'GB' : peak >= 0.001 ? 'MB' : 'KB');
  const value = (gb: number) => (gb / unit.div).toFixed(unit.digits);
</script>

<div class="tubes" role="img" aria-label="{label} (in {unitName})" data-unit={unitName}>
  {#each months as m (m.label)}
    <div class="month" aria-hidden="true">
      <span class="value">{value(m.gb)}</span>
      <div class="tube k-tube" style:height="{tubeHeight}px">
        <div class="liquid k-liquid" style:height="{Math.max(6, Math.round((m.gb / max) * 100))}%"></div>
        <div class="shine" style:height="{Math.round(tubeHeight * 0.75)}px"></div>
      </div>
      <span class="label" class:current={m.current}>{m.label}</span>
    </div>
  {/each}
</div>

<style>
  .tubes {
    flex: 1;
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    height: 100%;
    min-width: 0;
  }
  .month {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .value {
    font-size: 11px;
    color: var(--k-text-2);
  }
  .tube {
    position: relative;
    width: 30px;
    border-radius: 8px;
    overflow: hidden;
  }
  .liquid {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: 5px;
    transition: height var(--k-dur-grow) var(--k-ease-grow);
  }
  .shine {
    position: absolute;
    left: 6px;
    top: 8px;
    width: 3px;
    border-radius: 2px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.55), rgba(255, 255, 255, 0));
  }
  .label {
    font-size: 11px;
    color: var(--k-text-4);
  }
  .label.current {
    color: var(--k-text);
  }
</style>

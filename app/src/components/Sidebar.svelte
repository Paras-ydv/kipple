<script lang="ts">
  import { Icon, Wordmark, formatBytes } from '@kipple/design';
  import { native } from '../lib/client.ts';
  import { NAV, router } from '../lib/router.svelte.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';

  const VERSION = '0.1.0';

  const badges = $derived.by(() => {
    const ai = data.report?.summary.categories.find((c) => c.category === 'ai-history')?.reclaimable ?? 0;
    const q = data.quarantine.reduce((s, e) => s + e.size.reclaimable, 0);
    return {
      ai: ai > 0 ? formatBytes(ai) : '',
      review: planStore.bytes > 0 ? formatBytes(planStore.bytes) : '',
      quarantine: q > 0 ? formatBytes(q) : '',
    } as Record<string, string>;
  });

  const active = $derived(router.route.name === 'categories' ? 'overview' : router.route.name);
</script>

<nav class="sidebar k-glass" aria-label="Sidebar">
  {#if native}
    <div class="lights-space" data-tauri-drag-region></div>
  {:else}
    <div class="lights" aria-hidden="true">
      <span style:background="var(--k-traffic-close)"></span>
      <span style:background="var(--k-traffic-min)"></span>
      <span style:background="var(--k-traffic-zoom)"></span>
    </div>
  {/if}

  <div class="section">
    <span class="label" id="nav-library">Library</span>
    <ul aria-labelledby="nav-library">
      {#each NAV as item, i (item.name)}
        {@const selected = active === item.name}
        <li>
          <a
            href={item.href}
            class:k-sel={selected}
            aria-current={selected ? 'page' : undefined}
            aria-keyshortcuts="Meta+{i + 1}"
          >
            <Icon name={item.icon} size={17} strokeWidth={1.8} stroke={selected ? '#7FB8FF' : 'rgba(255,255,255,0.8)'} />
            <span class="name" style:font-weight={selected ? 600 : 500}>{item.label}</span>
            {#if badges[item.name]}<span class="badge">{badges[item.name]}</span>{/if}
          </a>
        </li>
      {/each}
    </ul>
  </div>

  <div class="foot">
    <Wordmark size={17} />
    <span class="version">{VERSION}</span>
  </div>
</nav>

<style>
  .sidebar {
    width: 236px;
    flex: none;
    border-radius: var(--k-r-sidebar);
    padding: 16px 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .lights {
    display: flex;
    gap: 8px;
    padding: 2px 8px;
  }
  .lights span {
    width: 12px;
    height: 12px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 0.5px rgba(0, 0, 0, 0.25);
  }
  .lights-space {
    height: 16px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    color: var(--k-text-4);
    padding: 4px 10px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  a {
    text-decoration: none;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: var(--k-r-row);
    font-size: 13px;
    transition: background var(--k-dur-response) ease-out;
  }
  a:hover:not(.k-sel) {
    background: rgba(255, 255, 255, 0.06);
  }
  .name {
    flex: 1;
  }
  .badge {
    font-size: 12px;
    color: var(--k-text-4);
  }
  .foot {
    margin-top: auto;
    padding: 0 10px;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  .version {
    font-size: 11px;
    color: var(--k-text-4);
  }
</style>

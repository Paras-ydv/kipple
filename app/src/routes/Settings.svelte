<script lang="ts">
  // Settings (docs/UI.md §3.6). Scanning, safety and integrations are kipple's
  // config file (SPEC §10.3), read and written through the core. Menu bar and
  // update preferences belong to the app and stay on this device.
  import type { KippleConfig } from '@kipple/client';
  import { KippleError } from '@kipple/client';
  import { GlassPanel, Switch, categoryLabel, tildePath, type CategoryId } from '@kipple/design';
  import Toolbar from '../components/Toolbar.svelte';
  import { client, native } from '../lib/client.ts';
  import { data } from '../lib/stores/data.svelte.ts';

  const KEY = 'kipple.settings';
  interface AppPrefs {
    menuBar: boolean;
    checkEvery: string;
    growthAlertGb: number;
    updates: boolean;
  }
  const defaults: AppPrefs = { menuBar: true, checkEvery: '1h', growthAlertGb: 10, updates: false };

  function loadPrefs(): AppPrefs {
    try {
      return { ...defaults, ...(JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<AppPrefs>) };
    } catch {
      return { ...defaults };
    }
  }

  let prefs = $state(loadPrefs());
  $effect(() => {
    const snapshot = JSON.stringify(prefs);
    try {
      localStorage.setItem(KEY, snapshot);
    } catch {
      // Not saved: private window or storage blocked. The values still apply for now.
    }
  });

  // kipple's config, from the core.
  let config = $state<KippleConfig | null>(null);
  let status = $state<{ kind: 'saved' | 'error'; text: string } | null>(null);

  $effect(() => {
    client
      .config()
      .then((c) => (config = c))
      .catch((err) => (status = { kind: 'error', text: KippleError.from(err).message }));
  });

  async function save() {
    if (!config) return;
    try {
      config = await client.setConfig($state.snapshot(config) as KippleConfig);
      status = { kind: 'saved', text: 'Saved. It applies from the next scan.' };
      void data.load();
    } catch (err) {
      status = { kind: 'error', text: KippleError.from(err).message };
      config = await client.config().catch(() => config);
    }
  }

  const groups = $derived.by(() => {
    const m = new Map<string, typeof data.integrations>();
    for (const i of data.integrations) m.set(i.category, [...(m.get(i.category) ?? []), i]);
    return [...m.entries()];
  });

  function isOn(i: (typeof data.integrations)[number]): boolean {
    if (!config) return i.enabled;
    if (config.integrations.disable.includes(i.id)) return false;
    return config.integrations.enable.includes(i.id) || i.enabled;
  }

  function toggleIntegration(i: (typeof data.integrations)[number], on: boolean) {
    if (!config) return;
    const { disable, enable } = config.integrations;
    config.integrations.disable = on ? disable.filter((x) => x !== i.id) : [...new Set([...disable, i.id])];
    // Opt-in integrations (off unless enabled) need to be listed to turn on.
    config.integrations.enable = on && !i.enabled ? [...new Set([...enable, i.id])] : enable.filter((x) => x !== i.id || on);
    void save();
  }
</script>

<Toolbar title="Settings" subtitle={status?.text ?? 'Changes save right away'} />

<div class="scroll">
  <div class="col">
    <GlassPanel tag="section" radius={20} padding="16px 20px" aria-labelledby="s-scan">
      <h2 id="s-scan">Scanning</h2>
      <div class="row">
        <span>
          <span class="t">Scan folders</span>
          <span class="d k-mono">{config ? config.scan.roots.map((r) => (r === '~' ? '~ (your home folder)' : tildePath(r))).join(', ') : '…'}</span>
        </span>
      </div>
      {#if config}
        <label class="row">
          <span>
            <span class="t">Count a project as recent for</span>
            <span class="d">Build output of projects used more recently than this needs your review first.</span>
          </span>
          <span class="num"><input type="number" min="1" max="365" bind:value={config.safety.recent_days} onchange={save} /> days</span>
        </label>
      {/if}
    </GlassPanel>

    <GlassPanel tag="section" radius={20} padding="16px 20px" aria-labelledby="s-safety">
      <h2 id="s-safety">Safety</h2>
      {#if config}
        <label class="row">
          <span><span class="t">Keep quarantined items for</span><span class="d">Then they are deleted for good.</span></span>
          <span class="num"><input type="number" min="1" max="60" bind:value={config.quarantine.keep_days} onchange={save} /> days</span>
        </label>
      {/if}
      <div class="row">
        <span><span class="t">Protected folders</span><span class="d">kipple never touches your documents, photos, keys or the system.</span></span>
      </div>
    </GlassPanel>

    <GlassPanel tag="section" radius={20} padding="16px 20px" aria-labelledby="s-int">
      <h2 id="s-int">Integrations</h2>
      {#each groups as [cat, list] (cat)}
        <h3>{categoryLabel(cat as CategoryId)}</h3>
        {#each list as i (i.id)}
          <div class="row">
            <span>
              <span class="t">{i.name}{i.status === 'experimental' ? ' (experimental)' : ''}</span>
              <span class="d">{i.detection.state === 'present' ? `Found${i.detection.version ? ` ${i.detection.version}` : ''}` : 'Not found'}</span>
            </span>
            <Switch label="Use {i.name}" checked={isOn(i)} onchange={(on) => toggleIntegration(i, on)} />
          </div>
        {/each}
      {/each}
    </GlassPanel>

    <GlassPanel tag="section" radius={20} padding="16px 20px" aria-labelledby="s-menu">
      <h2 id="s-menu">Menu bar</h2>
      <label class="row">
        <span class="t">Show kipple in the menu bar</span>
        <Switch label="Show kipple in the menu bar" bind:checked={prefs.menuBar} />
      </label>
      <label class="row">
        <span class="t">Check every</span>
        <select bind:value={prefs.checkEvery}>
          <option value="1h">1 hour</option>
          <option value="6h">6 hours</option>
          <option value="1d">Day</option>
        </select>
      </label>
      <label class="row">
        <span><span class="t">Show a dot when kipple grew more than</span></span>
        <span class="num"><input type="number" min="1" max="500" bind:value={prefs.growthAlertGb} /> GB</span>
      </label>
    </GlassPanel>

    <GlassPanel tag="section" radius={20} padding="16px 20px" aria-labelledby="s-upd">
      <h2 id="s-upd">Updates</h2>
      <label class="row">
        <span><span class="t">Check for updates</span><span class="d">Off by default. Updates are signed and checked before they install.</span></span>
        <Switch label="Check for updates" bind:checked={prefs.updates} />
      </label>
    </GlassPanel>

    {#if !native}
      <p class="note">This preview keeps settings in memory.</p>
    {/if}
  </div>
</div>

<style>
  .scroll {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .col {
    max-width: 720px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 12px;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 15px;
    font-weight: 650;
  }
  h3 {
    margin: 12px 0 2px;
    font-size: 11px;
    font-weight: 600;
    color: var(--k-text-4);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 48px;
    border-top: 1px solid var(--k-line);
    font-size: 13px;
  }
  .row > span:first-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .d {
    font-size: 11px;
    color: var(--k-text-4);
  }
  .num {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--k-text-3);
  }
  input[type='number'],
  select {
    height: 30px;
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.24);
    background: rgba(255, 255, 255, 0.07);
    padding: 0 10px;
    font-size: 13px;
  }
  input[type='number'] {
    width: 72px;
    text-align: right;
  }
  select option {
    color: #111;
  }
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--k-text-4);
  }
</style>

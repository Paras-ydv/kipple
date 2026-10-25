<script lang="ts">
  // AI history (docs/UI.md §3.3).
  import type { Item } from '@kipple/client';
  import { Button, FlagTag, GlassPanel, MonthTubes, Segmented, formatBytes, plural } from '@kipple/design';
  import Toolbar from '../components/Toolbar.svelte';
  import {
    AI_TOOLS,
    TOOL_NAMES,
    monthlyFromHistory,
    monthlyFromItems,
    sessionFlag,
    sessionSubtitle,
    type AiTool,
  } from '../lib/derive.ts';
  import { router } from '../lib/router.svelte.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';

  interface Props {
    tool?: string;
  }
  let { tool }: Props = $props();

  const byTool = $derived(
    Object.fromEntries(AI_TOOLS.map((t) => [t, data.items.filter((i) => i.integration === t)])) as Record<AiTool, Item[]>,
  );
  const sizeOf = (list: Item[]) => list.filter((i) => i.safety !== 'keep').reduce((s, i) => s + i.size.reclaimable, 0);
  const tools = $derived(
    AI_TOOLS.filter((t) => byTool[t].length > 0).sort((a, b) => sizeOf(byTool[b]) - sizeOf(byTool[a])),
  );
  const current: AiTool = $derived((tools.includes(tool as AiTool) ? tool : tools[0]) as AiTool);
  const sessions = $derived([...(byTool[current] ?? [])].sort((a, b) => b.size.reclaimable - a.size.reclaimable));
  const total = $derived(sizeOf(sessions));
  const aiTotal = $derived(tools.reduce((s, t) => s + sizeOf(byTool[t]), 0));

  // Checked sessions: kipple's suggestion per tool, then whatever the user changes.
  let checked = $state(new Set<string>());
  let onlySecrets = $state(false);
  $effect(() => {
    checked = new Set(sessions.filter(suggested).map((i) => i.id));
    onlySecrets = false;
  });

  function suggested(i: Item): boolean {
    if (i.safety === 'keep' || i.kind === 'cursor_state_db') return false;
    const d = i.details as { suggested?: boolean; flag?: string | null } | undefined;
    if (i.integration === 'codex') return !!d?.suggested;
    if (i.integration === 'antigravity') return true;
    return d?.flag === 'orphan';
  }

  const visible = $derived(onlySecrets ? sessions.filter((i) => sessionFlag(i)?.kind === 'secret') : sessions);
  const picked = $derived(sessions.filter((i) => checked.has(i.id)));
  const pickedBytes = $derived(picked.reduce((s, i) => s + i.size.reclaimable, 0));

  const months = $derived.by(() => {
    const fromHistory = monthlyFromHistory(data.history.filter((p) => p.scope === 'integration' && p.key === current));
    return fromHistory ?? monthlyFromItems(sessions);
  });
  const thisMonth = $derived(months[months.length - 1]?.gb ?? 0);
  const first = $derived(months.find((m) => m.gb > 0));

  const secrets = $derived(data.items.filter((i) => i.category === 'ai-history' && sessionFlag(i)?.kind === 'secret'));
  const secretPhrase = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const i of secrets) {
      const kind = String((i.details as { secret?: string | null } | undefined)?.secret ?? 'secret').replace(' API key', ' key');
      counts.set(kind, (counts.get(kind) ?? 0) + 1);
    }
    const words = ['no', 'one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine'];
    const parts = [...counts].map(([k, n]) =>
      n === 1 ? `${/^[AEIOU]/i.test(k) ? 'An' : 'A'} ${k}` : `${words[n] ?? n} ${k}s`,
    );
    const text = parts.length > 1 ? `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]?.replace(/^An? /, (m) => m.toLowerCase())}` : (parts[0] ?? '');
    return `${text.charAt(0).toUpperCase()}${text.slice(1)} ${secrets.length === 1 ? 'was' : 'were'} pasted into chats.`;
  });
  const orphanSessions = $derived(byTool['claude-code'].filter((i) => sessionFlag(i)?.kind === 'orphan').length);
  const cursorDb = $derived(byTool.cursor.find((i) => i.kind === 'cursor_state_db'));
  const cursorFree = $derived(((cursorDb?.details as { free_pages_bytes?: number } | undefined)?.free_pages_bytes ?? 0));

  const name = (t: string) => TOOL_NAMES[t] ?? t;
  const unit = (t: AiTool, n: number) =>
    t === 'cursor' ? plural(n, 'item') : t === 'antigravity' ? plural(n, 'copy', 'copies') : plural(n, 'session');

  function explainText(t: AiTool): string {
    switch (t) {
      case 'codex':
        return `Each session keeps every tool output, so one long session can pass 1 GB. This month alone added ${formatBytes(thisMonth * 1e9)}.`;
      case 'cursor':
        return cursorDb
          ? `${formatBytes(cursorDb.size.reclaimable)} is chat data that only Cursor can remove safely. Run Delete Old Chats in Cursor, then compact the database here.`
          : 'Workspace storage for folders you deleted can go.';
      case 'claude-code':
        return `Cleans transcripts older than 30 days by itself. ${plural(orphanSessions, 'session')} belong to folders you deleted.`;
      case 'antigravity':
        return 'Keeps an old copy of each conversation after it migrates them. Support is experimental, so kipple asks before it moves them.';
    }
  }

  const sidePanels = $derived(tools.filter((t) => t !== current).slice(0, 2));

  const actionLabel = $derived.by(() => {
    const n = picked.length;
    const size = formatBytes(pickedBytes);
    switch (current) {
      case 'codex':
        return `Delete ${plural(n, 'session')} with Codex (${size})`;
      case 'claude-code':
        return `Move ${plural(n, 'session')} to quarantine (${size})`;
      case 'cursor':
        return `Move ${plural(n, 'folder')} to quarantine (${size})`;
      default:
        return `Move ${plural(n, 'copy', 'copies')} to quarantine (${size})`;
    }
  });

  const footnote = $derived.by(() => {
    const open = sessions.some((i) => i.safety === 'keep');
    switch (current) {
      case 'codex':
        return open ? 'Quit Codex first. The open session stays.' : 'Quit Codex first.';
      case 'claude-code':
        return 'Transcripts are moved whole and never edited.';
      case 'cursor':
        return 'Quit Cursor first.';
      default:
        return 'Antigravity support is experimental.';
    }
  });

  function toggle(i: Item) {
    const next = new Set(checked);
    if (next.has(i.id)) next.delete(i.id);
    else next.add(i.id);
    checked = next;
  }

  async function run() {
    if (!picked.length) return;
    // Keep this tool's tab after the sizes change.
    if (tool !== current) router.go({ name: 'ai', tool: current }, { replace: true });
    await planStore.runItems(
      picked.map((i) => i.id),
      actionLabel.replace(/ \(.*\)$/, ''),
    );
  }

  async function compactCursor() {
    if (cursorDb) await planStore.runItems([cursorDb.id], `Compact database, frees ${formatBytes(cursorFree)}`);
  }

  function showSecrets() {
    const t = secrets[0]?.integration;
    if (t && t !== current) router.go({ name: 'ai', tool: t });
    onlySecrets = true;
  }
</script>

<Toolbar title="AI history" subtitle="{formatBytes(aiTotal)} across {plural(tools.length, 'tool')}">
  <Segmented
    label="Tools"
    value={current}
    selectedWeight={650}
    onchange={(t) => router.go({ name: 'ai', tool: t }, { replace: true })}
    segments={tools.map((t) => ({ value: t, label: name(t), detail: formatBytes(sizeOf(byTool[t])) }))}
  />
</Toolbar>

<GlassPanel tag="section" radius={22} padding="20px 24px" class="tool" aria-labelledby="ai-tool">
  <div class="tool-text">
    <h2 id="ai-tool">{name(current)}</h2>
    <span class="tool-total">{formatBytes(total)}</span>
    <span class="tool-count">in {unit(current, sessions.length)}</span>
    <p>{explainText(current)}</p>
  </div>
  <MonthTubes
    {months}
    label="{name(current)} history added per month, from {formatBytes((first?.gb ?? 0) * 1e9)} a year ago to {formatBytes(thisMonth * 1e9)} this month"
  />
</GlassPanel>

<div class="lower">
  <GlassPanel tag="section" radius={20} padding="10px 18px 14px" class="sessions" aria-label="{name(current)} sessions">
    {#if onlySecrets}
      <div class="filterbar">
        <span>Showing sessions with keys in them</span>
        <button type="button" class="link" onclick={() => (onlySecrets = false)}>Show all</button>
      </div>
    {/if}
    <ul>
      {#each visible as s (s.id)}
        {@const flag = sessionFlag(s)}
        {@const locked = s.safety === 'keep'}
        <li>
          <label class:locked>
            <input
              type="checkbox"
              checked={checked.has(s.id)}
              disabled={locked}
              onchange={() => toggle(s)}
              aria-label="Select {s.title}"
              aria-describedby={locked ? `lock-${s.id}` : undefined}
            />
            <span class="s-text">
              <span class="s-title">{s.title}</span>
              <span class="s-sub">{sessionSubtitle(s)}</span>
            </span>
            {#if flag}<FlagTag kind={flag.kind} text={flag.text} />{/if}
            {#if locked}<span id="lock-{s.id}" class="k-visually-hidden">{s.reasons[0]?.text}</span>{/if}
            <span class="s-size">{formatBytes(s.size.reclaimable)}</span>
          </label>
        </li>
      {/each}
    </ul>
    <div class="foot">
      <span class="note">{footnote}</span>
      <Button variant="tint" size="md" height={38} pad={18} weight={650} onclick={run} disabled={!picked.length || planStore.phase === 'running'}>
        {actionLabel}
      </Button>
    </div>
  </GlassPanel>

  <div class="side">
    {#if secrets.length}
      <div class="secret k-amber">
        <span class="secret-title">{plural(secrets.length, 'session')} contain API keys</span>
        <span class="secret-body">
          {secretPhrase} Rotate {secrets.length === 1 ? 'it' : 'them'}, then remove the {secrets.length === 1 ? 'session' : 'sessions'}.
        </span>
        <button type="button" class="link secret-link" onclick={showSecrets}>Show the {secrets.length} sessions</button>
      </div>
    {/if}
    {#each sidePanels as t (t)}
      <GlassPanel radius={20} padding={t === 'cursor' ? '16px 18px' : '14px 18px'} class="other">
        <div class="other-head" class:big={t === 'cursor'}>
          <a href="#/ai/{t}">{name(t)}</a>
          <span>{formatBytes(sizeOf(byTool[t]))}</span>
        </div>
        <span class="other-body">{explainText(t)}</span>
        {#if t === 'cursor' && cursorDb && cursorFree > 0}
          <div>
            <Button variant="clear" size="sm" onclick={compactCursor}>Compact database, frees {formatBytes(cursorFree)}</Button>
          </div>
        {/if}
      </GlassPanel>
    {/each}
  </div>
</div>

<style>
  :global(.tool) {
    display: flex;
    gap: 32px;
    align-items: flex-end;
    height: 250px;
    flex: none;
  }
  .tool-text {
    width: 270px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-self: stretch;
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
  }
  .tool-total {
    font-family: var(--k-font-display);
    font-size: 54px;
    line-height: 1;
    font-weight: 700;
    letter-spacing: -0.045em;
  }
  .tool-count {
    font-size: 13px;
    color: var(--k-text-3);
  }
  .tool-text p {
    margin: auto 0 0;
    font-size: 13px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.7);
  }
  .lower {
    flex: 1;
    display: flex;
    gap: 14px;
    min-height: 0;
  }
  :global(.sessions) {
    flex: 3;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .filterbar {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--k-text-3);
    padding: 6px 0;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: auto;
    flex: 1;
    min-height: 0;
  }
  label {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 46px;
    border-bottom: 1px solid var(--k-line-soft);
    cursor: pointer;
  }
  label.locked {
    cursor: default;
  }
  input {
    width: 16px;
    height: 16px;
    accent-color: var(--k-accent);
    margin: 0;
    flex: none;
  }
  .s-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .s-title {
    font-size: 13px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .s-sub {
    font-size: 11px;
    color: var(--k-text-4);
  }
  .s-size {
    width: 64px;
    text-align: right;
    font-size: 13px;
    color: rgba(255, 255, 255, 0.8);
    flex: none;
  }
  .foot {
    padding-top: 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .note {
    font-size: 12px;
    color: var(--k-text-4);
  }
  .side {
    flex: 2;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .secret {
    border-radius: 20px;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .secret-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--k-secret);
  }
  .secret-body {
    font-size: 12px;
    line-height: 1.5;
    color: var(--k-secret-body);
  }
  .link {
    border: 0;
    background: transparent;
    padding: 0;
    font-size: 12px;
    font-weight: 650;
    text-decoration: underline;
    text-underline-offset: 3px;
    color: var(--k-text-2);
    align-self: flex-start;
  }
  .secret-link {
    color: var(--k-secret);
  }
  :global(.other) {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .other-head {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 650;
  }
  .other-head.big {
    font-size: 14px;
    font-weight: 700;
  }
  .other-head a {
    text-decoration: none;
  }
  .other-head a:hover {
    text-decoration: underline;
  }
  .other-body {
    font-size: 12px;
    line-height: 1.5;
    color: rgba(255, 255, 255, 0.68);
  }
</style>

<script lang="ts">
  // Typed confirmation for plans with Review items or without quarantine (SPEC §8.2).
  import { Button, Dot, SafetyTag, categoryColor, formatBytes } from '@kipple/design';
  import { ui } from '../lib/stores/ui.svelte.ts';
  import Modal from './Modal.svelte';

  const WORD = 'reclaim';
  let typed = $state('');
  const req = $derived(ui.confirm);
  const ok = $derived(typed.trim() === WORD);
  const review = $derived(req?.plan?.entries.filter((e) => e.item.safety === 'review') ?? []);
  const totals = $derived(req?.plan?.totals);
  const freedNow = $derived(totals ? totals.freed_now + (req?.purgeNow ? totals.held_in_quarantine : 0) : 0);
  const held = $derived(totals && !req?.purgeNow ? totals.held_in_quarantine : 0);

  $effect(() => {
    if (req) typed = '';
  });

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (req && ok) req.resolve({ kind: 'typed', text: typed.trim(), purge_now: req.purgeNow });
  }
</script>

{#if req}
  <Modal label="Confirm: {req.action}" width={440} onclose={() => req.resolve(null)}>
    <h2>{req.action}</h2>
    <div class="lines">
      {#each req.lines ?? [] as line, i (i)}<p><span>{line}</span></p>{/each}
      {#if freedNow > 0}
        <p><Dot color="var(--k-positive)" /><span><strong>{formatBytes(freedNow)} is freed right away.</strong> There is no undo for it.</span></p>
      {/if}
      {#if held > 0}
        <p><Dot color="var(--k-accent-text)" /><span><strong>{formatBytes(held)} waits in quarantine for 7 days</strong> so you can undo it.</span></p>
      {/if}
    </div>

    {#if review.length}
      <section aria-label="Items to review">
        <p class="sub">This plan includes {review.length === 1 ? 'an item' : `${review.length} items`} marked Review:</p>
        <ul>
          {#each review.slice(0, 8) as e (e.item.id)}
            <li>
              <Dot color={categoryColor(e.item.category)} size={6} />
              <span class="t">{e.item.title}</span>
              <SafetyTag level="review" />
              <span class="s">{formatBytes(e.item.size.reclaimable)}</span>
            </li>
          {/each}
          {#if review.length > 8}<li class="more">and {review.length - 8} more</li>{/if}
        </ul>
      </section>
    {/if}

    <form onsubmit={submit}>
      <label for="confirm-word">Type <strong>{WORD}</strong> to continue</label>
      <input
        id="confirm-word"
        data-autofocus
        bind:value={typed}
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        aria-describedby="confirm-hint"
      />
      <span id="confirm-hint" class="k-visually-hidden">The button turns on when the field says {WORD}.</span>
      <div class="buttons">
        <Button variant="clear" size="md" onclick={() => req.resolve(null)}>Cancel</Button>
        <Button variant="tint" size="md" type="submit" disabled={!ok}>{req.action}</Button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
  }
  .lines {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .lines p {
    margin: 0;
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-size: 13px;
    line-height: 1.45;
    color: var(--k-text-3);
  }
  .lines strong {
    color: var(--k-text);
    font-weight: 650;
  }
  .sub {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--k-text-3);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 12px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    border-top: 1px solid var(--k-line);
  }
  .t {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .s {
    width: 64px;
    text-align: right;
    color: var(--k-text-2);
  }
  .more {
    color: var(--k-text-4);
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  label {
    font-size: 13px;
  }
  input {
    height: 38px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.24);
    background: rgba(255, 255, 255, 0.07);
    padding: 0 12px;
    font-family: var(--k-font-mono);
    font-size: 14px;
  }
  input:focus {
    outline: 2px solid var(--k-focus);
    outline-offset: 1px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }
</style>

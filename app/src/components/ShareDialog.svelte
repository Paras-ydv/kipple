<script lang="ts">
  // Share card preview and PNG export (docs/UI.md §5).
  import { Button, renderShareCardSvg, SHARE_HEIGHT, SHARE_WIDTH, type CategoryId, type ShareCardInput } from '@kipple/design';
  import monaUrl from '@kipple/design/fonts/MonaSansVF-wdth-opsz-wght.woff2?url';
  import { deviceWord } from '../lib/derive.ts';
  import { data } from '../lib/stores/data.svelte.ts';
  import { planStore } from '../lib/stores/plan.svelte.ts';
  import { ui } from '../lib/stores/ui.svelte.ts';
  import Modal from './Modal.svelte';

  let fontsReady = $state(false);
  let saving = $state(false);

  $effect(() => {
    if (!ui.shareOpen) return;
    Promise.all([document.fonts.load('800 64px "Mona Sans"'), document.fonts.load('400 17px "Mona Sans"')]).then(
      () => (fontsReady = true),
      () => (fontsReady = true),
    );
  });

  let ctx: CanvasRenderingContext2D | null = null;
  function measure(text: string, font: string): number {
    ctx ??= document.createElement('canvas').getContext('2d');
    if (!ctx) return text.length * 10;
    ctx.font = font;
    return ctx.measureText(text).width;
  }

  const input: ShareCardInput = $derived.by(() => {
    const result = planStore.result;
    if (result && planStore.phase === 'done') {
      const freedByCat = new Map<CategoryId, number>();
      for (const e of result.entries) {
        if (e.outcome.status !== 'done') continue;
        const item = data.journal.find((j) => j.item_id === e.item_id);
        const cat = (data.integrations.find((i) => i.id === item?.integration)?.category ?? 'system') as CategoryId;
        freedByCat.set(cat, (freedByCat.get(cat) ?? 0) + e.outcome.freed);
      }
      const freed = result.freed_now + result.held_in_quarantine;
      return {
        variant: 'reclaimed',
        total: freed,
        device: deviceWord(),
        categories: [...freedByCat].map(([id, bytes]) => ({ id, bytes })),
        remaining: data.report?.summary.reclaimable ?? 0,
        measure,
      };
    }
    return {
      variant: 'found',
      total: data.report?.summary.reclaimable ?? 0,
      device: deviceWord(),
      categories: (data.report?.summary.categories ?? []).map((c) => ({ id: c.category as CategoryId, bytes: c.reclaimable })),
      measure,
    };
  });

  const svg = $derived(fontsReady ? renderShareCardSvg(input) : '');

  async function save() {
    saving = true;
    try {
      const font = await fetch(monaUrl).then((r) => r.arrayBuffer());
      const bytes = new Uint8Array(font);
      let bin = '';
      for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      const b64 = btoa(bin);
      const style = `<style>@font-face{font-family:'Mona Sans';src:url(data:font/woff2;base64,${b64}) format('woff2');font-weight:200 900;font-stretch:75% 125%}</style>`;
      const withFont = svg.replace('<defs>', `<defs>${style}`);
      const img = new Image();
      img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(withFont)}`;
      await img.decode();
      const canvas = document.createElement('canvas');
      canvas.width = SHARE_WIDTH;
      canvas.height = SHARE_HEIGHT;
      canvas.getContext('2d')?.drawImage(img, 0, 0);
      const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/png'));
      if (!blob) throw new Error('no image');
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = 'kipple-card.png';
      a.click();
      URL.revokeObjectURL(a.href);
      ui.toast('Saved kipple-card.png.');
    } catch {
      ui.toast('The image could not be saved.');
    } finally {
      saving = false;
    }
  }
</script>

{#if ui.shareOpen}
  <Modal label="Share card" width={680} onclose={() => (ui.shareOpen = false)}>
    <h2>Share card</h2>
    <p class="sub">No paths, project names or user names are on the card.</p>
    <div class="preview" role="img" aria-label="Share card: {input.variant === 'found' ? 'My Mac was hiding' : 'I just reclaimed'} kipple">
      {#if svg}{@html svg}{/if}
    </div>
    <div class="actions">
      <Button variant="clear" size="md" onclick={() => (ui.shareOpen = false)}>Close</Button>
      <Button variant="tint" size="md" onclick={save} disabled={!svg || saving}>{saving ? 'Saving' : 'Save image'}</Button>
    </div>
  </Modal>
{/if}

<style>
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
  }
  .sub {
    margin: -8px 0 0;
    font-size: 12px;
    color: var(--k-text-3);
  }
  .preview {
    aspect-ratio: 1200 / 630;
    border-radius: 14px;
    overflow: hidden;
    box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.12);
    background: #06080f;
  }
  .preview :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>

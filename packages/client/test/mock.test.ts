// The mock must show the numbers drawn on the design boards.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { MockClient } from '../src/mock/index.ts';

const GB = 1e9;
const near = (bytes: number, gb: number) => Math.abs(bytes / GB - gb) < 0.05;

test('overview totals match the boards', async () => {
  const c = new MockClient({ stepMs: 0 });
  const r = await c.report();
  assert.ok(near(r.summary.reclaimable, 184), 'total 184 GB');
  assert.ok(near(r.summary.by_safety.safe, 76.5), 'safe 76.5 GB');
  assert.ok(near(r.summary.by_safety.review, 107.5), 'review 107.5 GB');
  const cats = Object.fromEntries(r.summary.categories.map((x) => [x.category, x.reclaimable / GB]));
  assert.deepEqual(
    Object.fromEntries(Object.entries(cats).map(([k, v]) => [k, Math.round(v * 10) / 10])),
    { container: 61.2, 'ai-history': 38.9, 'package-cache': 27.5, 'project-artifact': 24, mobile: 21.6, model: 10.8 },
  );
});

test('projects match the boards', async () => {
  const c = new MockClient({ stepMs: 0 });
  const ps = await c.projects();
  assert.equal(ps.length, 61);
  assert.ok(near(ps.reduce((s, p) => s + p.total.reclaimable, 0), 112));
  assert.deepEqual(
    ps.slice(0, 7).map((p) => p.name),
    ['nomad-app', 'llm-bench', 'android-shop', 'kipple', 'client-portal', 'portfolio-v3', 'rn-expense'],
  );
  const nomad = ps[0];
  assert.ok(nomad && near(nomad.total.reclaimable, 14.2) && near(nomad.safety_mix.safe, 6.1));
});

test('safe plan frees 43.0 GB now and holds 33.5 GB', async () => {
  const c = new MockClient({ stepMs: 0 });
  const plan = await c.plan({ filter: { safe_only: true } });
  assert.ok(near(plan.totals.reclaimable, 76.5));
  assert.ok(near(plan.totals.freed_now, 43.0));
  assert.ok(near(plan.totals.held_in_quarantine, 33.5));
});

test('codex has 318 sessions and 23 suggested ones', async () => {
  const c = new MockClient({ stepMs: 0 });
  const items = await c.items({ only: ['codex'] });
  assert.equal(items.length, 318);
  assert.ok(near(items.reduce((s, i) => s + i.size.reclaimable, 0), 21.4));
  const sug = items.filter((i) => (i.details as { suggested?: boolean }).suggested);
  assert.equal(sug.length, 23);
  assert.ok(near(sug.reduce((s, i) => s + i.size.reclaimable, 0), 11.4));
});

test('review items need the typed word, and quarantine can be undone', async () => {
  const c = new MockClient({ stepMs: 0 });
  const vol = (await c.items({ only: ['docker'], safety: ['review'] }))[0];
  assert.ok(vol);
  const plan = await c.plan({ filter: { ids: [vol.id] } });
  await assert.rejects(c.execute(plan, { kind: 'click' }), /Type reclaim/);
  const nm = (await c.items({ only: ['node-modules'] }))[0];
  assert.ok(nm);
  const p2 = await c.plan({ filter: { ids: [nm.id] } });
  const rep = await c.execute(p2, { kind: 'click' });
  assert.equal(rep.done, 1);
  const q = await c.quarantine();
  assert.equal(q.length, 1);
  const restored = await c.restore(q[0]?.id ?? '');
  assert.equal(restored.restored.length, 1);
  assert.equal((await c.items({ ids: [nm.id] })).length, 1);
});

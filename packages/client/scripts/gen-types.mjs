// Generates TypeScript types from the JSON Schemas in docs/schema/.
//
// The core exports one schema per public type (`cargo xtask schema`). Each
// file carries its own `$defs`; they are merged into one schema so every type
// is emitted once, then compiled with json-schema-to-typescript.
//
// Usage: pnpm --filter @kipple/client gen:types

import { readFileSync, readdirSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { compile } from 'json-schema-to-typescript';

const here = dirname(fileURLToPath(import.meta.url));
const schemaDir = resolve(here, '../../../docs/schema');
const outFile = resolve(here, '../src/generated/schema.ts');

const defs = {};
const roots = [];

for (const file of readdirSync(schemaDir).filter((f) => f.endsWith('.json')).sort()) {
  const schema = JSON.parse(readFileSync(join(schemaDir, file), 'utf8'));
  const { $defs = {}, $schema: _ignored, title, ...root } = schema;
  for (const [name, def] of Object.entries($defs)) {
    const prev = defs[name];
    const strip = ({ title: _t, ...rest }) => JSON.stringify(rest);
    if (prev && strip(prev) !== strip(def)) {
      throw new Error(`Conflicting definition of ${name} in ${file}`);
    }
    defs[name] = def;
  }
  if (!title) throw new Error(`${file} has no title`);
  // A root type may also appear as a $def of another schema; the root wins.
  defs[title] = { ...root, title };
  roots.push(title);
}

// `{ $ref, description, default }` would make json-schema-to-typescript emit a
// renamed copy of the referenced type (`Item1`, `Size1`). Keep only the $ref.
function plainRefs(node) {
  if (Array.isArray(node)) return node.map(plainRefs);
  if (node && typeof node === 'object') {
    if (typeof node.$ref === 'string') return { $ref: node.$ref };
    return Object.fromEntries(Object.entries(node).map(([k, v]) => [k, plainRefs(v)]));
  }
  return node;
}
for (const name of Object.keys(defs)) defs[name] = plainRefs(defs[name]);

// Give every definition a title so json-schema-to-typescript names it after
// its key instead of inventing a name.
for (const [name, def] of Object.entries(defs)) {
  if (!def.title) def.title = name;
}

const merged = {
  title: 'KippleSchemaRoot',
  type: 'object',
  additionalProperties: false,
  properties: Object.fromEntries(roots.map((r) => [r, { $ref: `#/$defs/${r}` }])),
  $defs: defs,
};

const banner = `/* eslint-disable */
// Generated from docs/schema/*.json by scripts/gen-types.mjs. Do not edit.
// Regenerate with: pnpm gen:types`;

let ts = await compile(merged, 'KippleSchemaRoot', {
  bannerComment: banner,
  additionalProperties: false,
  unreachableDefinitions: true,
  declareExternallyReferenced: true,
  enableConstEnums: false,
  strictIndexSignatures: true,
  style: { singleQuote: true, semi: true, printWidth: 100 },
});

mkdirSync(dirname(outFile), { recursive: true });
writeFileSync(outFile, ts);
console.log(`Wrote ${outFile} (${roots.length} root types, ${Object.keys(defs).length} definitions)`);

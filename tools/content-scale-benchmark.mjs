#!/usr/bin/env node
import { performance } from 'node:perf_hooks';
import YAML from 'yaml';

const count = positive(process.argv[2], 25_000);
const documents = [];
for (let index = 0; index < count; index += 1) {
  documents.push({
    type: 'entity',
    id: `SyntheticEntity${index}`,
    parent: index === 0 ? undefined : `SyntheticEntity${Math.floor((index - 1) / 4)}`,
    components: [
      { type: 'Transform' },
      { type: 'Sprite', layers: [{ map: 'base', texture: '/Resources/Textures/error.png' }] },
      { type: 'SyntheticState', value: index, enabled: index % 2 === 0 },
    ],
  });
}
const text = YAML.stringify(documents);
const started = performance.now();
const parsed = YAML.parse(text);
const parseMs = performance.now() - started;
if (!Array.isArray(parsed) || parsed.length !== count) throw new Error('Synthetic content roundtrip failed');
console.log(JSON.stringify({
  prototypes: count,
  yamlBytes: Buffer.byteLength(text),
  parseMs: Number(parseMs.toFixed(2)),
  prototypesPerSecond: Math.round(count / (parseMs / 1000)),
}, null, 2));

function positive(raw, fallback) {
  const value = Number.parseInt(raw ?? '', 10);
  return Number.isSafeInteger(value) && value > 0 ? value : fallback;
}

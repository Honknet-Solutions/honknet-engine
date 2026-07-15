#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { relative, resolve, sep } from 'node:path';

const root = resolve(process.argv[2] ?? '.');
const output = resolve(root, process.argv[3] ?? 'SOURCE_MANIFEST.sha256');
const excluded = new Set(['.git', 'node_modules', 'target', 'release', '.honknet-backups']);
const files = [];
await walk(root);
files.sort();
const lines = [];
for (const file of files) {
  if (file === output) continue;
  const data = await readFile(file);
  const digest = createHash('sha256').update(data).digest('hex');
  lines.push(`${digest}  ${relative(root, file).split(sep).join('/')}`);
}
await writeFile(output, `${lines.join('\n')}\n`);
console.log(`Wrote ${lines.length} checksums to ${output}`);

async function walk(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (excluded.has(entry.name)) continue;
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) await walk(path);
    else if (entry.isFile()) files.push(path);
  }
}

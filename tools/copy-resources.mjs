import { cp, mkdir, rm, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const source = path.join(root, 'game/example-module/resources');
const target = path.join(root, 'apps/client/public/Resources');
const manifestPath = path.join(root, 'apps/client/public/asset-manifest.json');

await rm(target, { recursive: true, force: true });
await mkdir(target, { recursive: true });
await cp(source, target, { recursive: true });

const entries = [];
async function walk(directory) {
  for (const entry of await readdir(directory)) {
    const full = path.join(directory, entry);
    const info = await stat(full);
    if (info.isDirectory()) {
      await walk(full);
      continue;
    }
    const data = await readFile(full);
    entries.push({
      path: '/Resources/' + path.relative(source, full).replaceAll('\\\\', '/'),
      bytes: info.size,
      sha256: createHash('sha256').update(data).digest('hex'),
    });
  }
}
await walk(source);
entries.sort((a, b) => a.path.localeCompare(b.path));
await writeFile(manifestPath, JSON.stringify({ version: 1, entries }, null, 2));
console.log(`Copied ${entries.length} resources.`);

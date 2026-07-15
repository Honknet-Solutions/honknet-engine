import { createHash } from 'node:crypto';
import { cp, mkdir, readdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import YAML from 'yaml';

const root = process.cwd();
const gameRoot = resolveGameRoot(root);
const clientPublic = process.env.HONKNET_CLIENT_PUBLIC
  ? path.resolve(root, process.env.HONKNET_CLIENT_PUBLIC)
  : path.join(root, 'apps/client/public');
const source = path.join(gameRoot, 'resources');
const target = path.join(clientPublic, 'Resources');
const manifestPath = path.join(clientPublic, 'asset-manifest.json');
const uiSource = path.join(gameRoot, 'content/ui');
const uiTarget = path.join(target, 'UI');
const localizationSource = path.join(gameRoot, 'localization');
const localizationTarget = path.join(target, 'Localization');

await rm(target, { recursive: true, force: true });
await mkdir(target, { recursive: true });
await cp(source, target, { recursive: true });

const uiEntries = [];
try {
  await mkdir(uiTarget, { recursive: true });
  await cp(uiSource, uiTarget, { recursive: true });
  for (const file of await collectFiles(uiSource)) {
    if (!/\.hui(?:\.ya?ml)?$/i.test(file)) continue;
    const relative = path.relative(uiSource, file).replaceAll('\\', '/');
    const parsed = YAML.parse(await readFile(file, 'utf8'));
    if (!parsed || typeof parsed !== 'object' || typeof parsed.type !== 'string') {
      throw new Error(`Invalid HUI document: ${file}`);
    }
    const id = typeof parsed.id === 'string'
      ? parsed.id
      : relative.replace(/\.hui(?:\.ya?ml)?$/i, '').replaceAll('/', '.');
    uiEntries.push({ id, path: `/Resources/UI/${relative}` });
  }
  uiEntries.sort((left, right) => left.id.localeCompare(right.id));
  await writeFile(path.join(uiTarget, 'manifest.json'), JSON.stringify({ version: 1, entries: uiEntries }, null, 2));
} catch (error) {
  if (error?.code !== 'ENOENT') throw error;
}

const localization = {};
try {
  await mkdir(localizationTarget, { recursive: true });
  await cp(localizationSource, localizationTarget, { recursive: true });
  for (const file of await collectFiles(localizationSource)) {
    if (!file.toLowerCase().endsWith('.ftl')) continue;
    const relative = path.relative(localizationSource, file).replaceAll('\\', '/');
    const locale = relative.split('/')[0];
    if (!locale) continue;
    localization[locale] ??= {};
    Object.assign(localization[locale], parseFtl(await readFile(file, 'utf8')));
  }
  await writeFile(
    path.join(localizationTarget, 'manifest.json'),
    JSON.stringify({ version: 1, locales: localization }, null, 2),
  );
} catch (error) {
  if (error?.code !== 'ENOENT') throw error;
}

const entries = [];
for (const file of await collectFiles(target)) {
  const info = await stat(file);
  const data = await readFile(file);
  entries.push({
    path: '/Resources/' + path.relative(target, file).replaceAll('\\', '/'),
    bytes: info.size,
    sha256: createHash('sha256').update(data).digest('hex'),
  });
}
entries.sort((left, right) => left.path.localeCompare(right.path));
await writeFile(manifestPath, JSON.stringify({ version: 2, entries, hui: uiEntries.length, locales: Object.keys(localization) }, null, 2));
console.log(`Copied ${entries.length} resources, ${uiEntries.length} HUI documents, ${Object.keys(localization).length} locales.`);

async function collectFiles(directory) {
  const output = [];
  async function walk(current) {
    for (const entry of await readdir(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) await walk(full);
      else output.push(full);
    }
  }
  await walk(directory);
  return output;
}

function parseFtl(sourceText) {
  const messages = {};
  let currentKey = null;
  for (const rawLine of sourceText.split(/\r?\n/)) {
    const match = /^([A-Za-z0-9_.-]+)\s*=\s*(.*)$/.exec(rawLine);
    if (match) {
      currentKey = match[1];
      messages[currentKey] = match[2];
      continue;
    }
    if (currentKey && /^\s+\S/.test(rawLine)) messages[currentKey] += `\n${rawLine.trim()}`;
    else if (!rawLine.trim()) currentKey = null;
  }
  return messages;
}

function resolveGameRoot(workspaceRoot) {
  const configured = process.env.HONKNET_GAME_ROOT;
  if (!configured) return path.join(workspaceRoot, 'examples/minimal-game');
  return path.isAbsolute(configured) ? configured : path.resolve(workspaceRoot, configured);
}

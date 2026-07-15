import { cp, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const toolsDir = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(toolsDir, '..');
const source = path.join(root, 'templates', 'empty-game');
const target = path.join(root, 'packages', 'cli', 'template');
await rm(target, { recursive: true, force: true });
await cp(source, target, { recursive: true });
console.log('Copied game template into @honknet/cli.');

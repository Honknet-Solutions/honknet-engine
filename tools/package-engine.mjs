#!/usr/bin/env node
import { readFile, rm, mkdir } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { resolve, basename, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const packageJson = JSON.parse(await readFile(resolve(root, 'package.json'), 'utf8'));
const version = packageJson.version;
const parent = dirname(root);
const folder = basename(root);
const outputDirectory = resolve(root, 'release');
const output = resolve(outputDirectory, `honknet-engine-sdk-${version}.tar.gz`);
await mkdir(outputDirectory, { recursive: true });
await rm(output, { force: true });
await run(process.execPath, [resolve(root, 'tools/source-manifest.mjs'), root]);
await run('tar', [
  '--exclude=.git',
  '--exclude=node_modules',
  '--exclude=target',
  '--exclude=release',
  '-czf', output,
  '-C', parent,
  folder,
]);
console.log(output);

function run(command, args) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, { stdio: 'inherit' });
    child.once('error', rejectPromise);
    child.once('exit', (code) => code === 0
      ? resolvePromise()
      : rejectPromise(new Error(`${command} failed with ${code}`)));
  });
}

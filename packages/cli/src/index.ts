#!/usr/bin/env node
import { access, cp, mkdir, readFile, stat } from 'node:fs/promises';
import { constants } from 'node:fs';
import { spawn } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ENGINE_VERSION = '0.2.0-rc.1';
const args = process.argv.slice(2);
const command = args.shift() ?? 'help';

try {
  switch (command) {
    case 'help':
    case '--help':
    case '-h':
      printHelp();
      break;
    case 'version':
    case '--version':
    case '-v':
      console.log(ENGINE_VERSION);
      break;
    case 'doctor':
      await doctor();
      break;
    case 'new':
      await createProject(args[0]);
      break;
    case 'verify':
      await verify();
      break;
    default:
      throw new Error(`Unknown command: ${command}`);
  }
} catch (error) {
  console.error(`[honknet] ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
}

function printHelp(): void {
  console.log(`Honknet CLI ${ENGINE_VERSION}

Usage:
  honknet doctor               Check local toolchain and engine layout
  honknet new <directory>      Create a game project from the empty template
  honknet verify               Run web and Rust verification gates
  honknet version              Print the engine version
`);
}

async function doctor(): Promise<void> {
  const root = await locateEngineRoot(process.cwd());
  const checks: Array<[string, boolean, string]> = [];
  checks.push(['Node.js >= 22', Number(process.versions.node.split('.')[0]) >= 22, process.version]);
  checks.push(['Engine root', Boolean(root), root ?? 'not found']);
  checks.push(['npm', await commandExists(process.platform === 'win32' ? 'npm.cmd' : 'npm'), 'required']);
  checks.push(['cargo', await commandExists('cargo'), 'required for Rust server']);
  checks.push(['rustc', await commandExists('rustc'), 'required for Rust server']);
  if (root) {
    checks.push(['engine.toml', await exists(join(root, 'engine.toml')), join(root, 'engine.toml')]);
    checks.push(['Cargo.toml', await exists(join(root, 'Cargo.toml')), join(root, 'Cargo.toml')]);
    checks.push(['package-lock.json', await exists(join(root, 'package-lock.json')), join(root, 'package-lock.json')]);
  }
  for (const [name, ok, detail] of checks) {
    console.log(`${ok ? 'OK ' : 'ERR'}  ${name.padEnd(24)} ${detail}`);
  }
  if (checks.some(([, ok]) => !ok)) process.exitCode = 1;
}

async function createProject(directory: string | undefined): Promise<void> {
  if (!directory) throw new Error('Target directory is required.');
  const target = resolve(directory);
  if (await exists(target)) {
    const info = await stat(target);
    if (!info.isDirectory()) throw new Error(`Target exists and is not a directory: ${target}`);
    const entries = await import('node:fs/promises').then(({ readdir }) => readdir(target));
    const nonRepositoryEntries = entries.filter((entry) => entry !== '.git');
    if (nonRepositoryEntries.length > 0) {
      throw new Error(`Target directory is not empty: ${target}`);
    }
  } else {
    await mkdir(target, { recursive: true });
  }
  const cliTemplate = resolve(dirname(fileURLToPath(import.meta.url)), '..', 'template');
  const root = await locateEngineRoot(dirname(fileURLToPath(import.meta.url)))
    ?? await locateEngineRoot(process.cwd());
  const template = await exists(cliTemplate)
    ? cliTemplate
    : root
      ? join(root, 'templates', 'empty-game')
      : null;
  if (!template) throw new Error('Could not locate the bundled Honknet game template.');
  await access(template, constants.R_OK);
  await cp(template, target, { recursive: true, force: false });
  console.log(`Created Honknet game project at ${target}`);
}

async function verify(): Promise<void> {
  const root = await locateEngineRoot(process.cwd());
  if (!root) throw new Error('Run this command inside a Honknet Engine checkout.');
  await run('npm', ['ci', '--no-audit', '--no-fund'], root);
  await run('npm', ['run', 'validate'], root);
  await run('npm', ['run', 'typecheck'], root);
  await run('npm', ['test'], root);
  await run('npm', ['run', 'build'], root);
  await run('cargo', ['fmt', '--all', '--', '--check'], root);
  await run('cargo', ['clippy', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings'], root);
  await run('cargo', ['test', '--workspace', '--all-features'], root);
  await run('cargo', ['build', '--workspace', '--release'], root);
}

async function locateEngineRoot(start: string): Promise<string | null> {
  const configured = process.env.HONKNET_ENGINE_ROOT;
  if (configured && await isEngineRoot(resolve(configured))) return resolve(configured);
  let current = resolve(start);
  for (;;) {
    if (await isEngineRoot(current)) return current;
    const parent = dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

async function isEngineRoot(path: string): Promise<boolean> {
  if (!await exists(join(path, 'Cargo.toml')) || !await exists(join(path, 'package.json'))) return false;
  try {
    const packageJson = JSON.parse(await readFile(join(path, 'package.json'), 'utf8')) as { name?: string };
    return packageJson.name === 'honknet-engine';
  } catch {
    return false;
  }
}

async function exists(path: string): Promise<boolean> {
  try { await access(path, constants.F_OK); return true; } catch { return false; }
}

async function commandExists(command: string): Promise<boolean> {
  return new Promise((resolvePromise) => {
    const child = spawn(command, ['--version'], { stdio: 'ignore', shell: false });
    child.once('error', () => resolvePromise(false));
    child.once('exit', (code) => resolvePromise(code === 0));
  });
}

async function run(command: string, commandArgs: string[], cwd: string): Promise<void> {
  await new Promise<void>((resolvePromise, rejectPromise) => {
    const executable = process.platform === 'win32' && command === 'npm' ? 'npm.cmd' : command;
    const child = spawn(executable, commandArgs, { cwd, stdio: 'inherit', shell: false });
    child.once('error', rejectPromise);
    child.once('exit', (code, signal) => {
      if (code === 0) resolvePromise();
      else rejectPromise(new Error(`${command} ${commandArgs.join(' ')} failed (${signal ?? code})`));
    });
  });
}

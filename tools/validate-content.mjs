import { access, readFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';
import YAML from 'yaml';

const root = process.cwd();
const gameRoot = resolveGameRoot(root);
const errors = [];
const warnings = [];

const yamlFiles = await walk(gameRoot, (file) => /\.(ya?ml)$/i.test(file));
const documents = [];
for (const file of yamlFiles) {
  try {
    const text = await readFile(file, 'utf8');
    const parsed = YAML.parse(text);
    documents.push({ file, parsed });
  } catch (error) {
    errors.push(`${relative(file)}: ${formatError(error)}`);
  }
}

const prototypes = new Map();
for (const { file, parsed } of documents) {
  const list = Array.isArray(parsed) ? parsed : [];
  for (const entry of list) {
    if (entry?.type !== 'entity') continue;
    if (typeof entry.id !== 'string' || entry.id.trim() === '') {
      errors.push(`${relative(file)}: entity prototype without a valid id`);
      continue;
    }
    if (prototypes.has(entry.id)) {
      errors.push(`${relative(file)}: duplicate prototype ${entry.id}`);
      continue;
    }
    prototypes.set(entry.id, { file, entry });
  }
}

for (const [id, { file, entry }] of prototypes) {
  if (entry.parent && !prototypes.has(entry.parent)) {
    errors.push(`${relative(file)}: prototype ${id} references missing parent ${entry.parent}`);
  }
  for (const component of entry.components ?? []) {
    if (component?.type !== 'Sprite') continue;
    const layers = Array.isArray(component.layers) ? component.layers : [component];
    for (const layer of layers) {
      const resource = layer.sprite ?? layer.texture;
      if (typeof resource !== 'string') continue;
      const local = resourcePath(resource);
      if (!local) {
        warnings.push(`${relative(file)}: sprite path ${resource} is outside /Resources`);
        continue;
      }
      try {
        await access(local);
      } catch {
        errors.push(`${relative(file)}: missing sprite resource ${resource}`);
      }
    }
  }
}

for (const { file, parsed } of documents) {
  if (parsed?.type === 'component-schema') validateComponentSchema(file, parsed);
  if (parsed?.type === 'Window') validateHui(file, parsed);
  if (parsed?.events && parsed?.id) validateBehavior(file, parsed);
  if (!Array.isArray(parsed) && parsed && typeof parsed === 'object' && parsed.map && typeof parsed.map === 'object') validateMap(file, parsed.map);
}

const rsiMetaFiles = await walk(path.join(gameRoot, 'resources'), (file) => path.basename(file) === 'meta.json' && file.includes('.rsi'));
for (const file of rsiMetaFiles) {
  await validateRsi(file);
}

if (warnings.length > 0) {
  console.warn(`Content validation warnings (${warnings.length}):`);
  for (const warning of warnings) console.warn(`  - ${warning}`);
}
if (errors.length > 0) {
  console.error(`Content validation failed (${errors.length}):`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exitCode = 1;
} else {
  console.log(`Content validation passed: ${prototypes.size} prototypes, ${rsiMetaFiles.length} RSI resources, ${yamlFiles.length} YAML documents.`);
}

function validateComponentSchema(file, schema) {
  if (typeof schema.id !== 'string' || !schema.id) errors.push(`${relative(file)}: component schema requires id`);
  if (!schema.fields || typeof schema.fields !== 'object') errors.push(`${relative(file)}: component schema requires fields`);
  for (const [name, field] of Object.entries(schema.fields ?? {})) {
    if (!field || typeof field !== 'object' || typeof field.type !== 'string') {
      errors.push(`${relative(file)}: field ${name} requires a type`);
    }
    if (typeof field.minimum === 'number' && typeof field.maximum === 'number' && field.minimum > field.maximum) {
      errors.push(`${relative(file)}: field ${name} minimum exceeds maximum`);
    }
  }
}

function validateHui(file, node) {
  const allowed = new Set(['Window', 'Row', 'Column', 'Label', 'Button', 'Image', 'List', 'Input', 'Checkbox', 'Spacer', 'Panel']);
  const visit = (value, location) => {
    if (!value || typeof value !== 'object') {
      errors.push(`${relative(file)}: ${location} is not an object`);
      return;
    }
    if (!allowed.has(value.type)) warnings.push(`${relative(file)}: unknown HUI node type ${String(value.type)} at ${location}`);
    if (Array.isArray(value.children)) value.children.forEach((child, index) => visit(child, `${location}.children[${index}]`));
  };
  visit(node, 'root');
}

function validateBehavior(file, graph) {
  const knownNodes = new Set(['Sequence', 'Branch', 'Log', 'EmitSystemMessage', 'SetComponent', 'RemoveComponent', 'Spawn', 'Delete', 'EmitEvent', 'OpenUi', 'PlaySound', 'Delay', 'ToggleDoor']);
  const visit = (node, location) => {
    if (!node || typeof node !== 'object' || typeof node.node !== 'string') {
      errors.push(`${relative(file)}: invalid behavior node at ${location}`);
      return;
    }
    if (!knownNodes.has(node.node)) warnings.push(`${relative(file)}: unknown behavior node ${node.node} at ${location}`);
    for (const key of ['children', 'success', 'failure']) {
      if (Array.isArray(node[key])) node[key].forEach((child, index) => visit(child, `${location}.${key}[${index}]`));
    }
  };
  for (const [event, nodes] of Object.entries(graph.events ?? {})) {
    if (!Array.isArray(nodes)) {
      errors.push(`${relative(file)}: behavior event ${event} must be an array`);
      continue;
    }
    nodes.forEach((node, index) => visit(node, `events.${event}[${index}]`));
  }
}

function validateMap(file, map) {
  if (typeof map.id !== 'string' || !map.id) errors.push(`${relative(file)}: map requires id`);
  const gridIds = new Set();
  for (const grid of map.grids ?? []) {
    if (typeof grid.id !== 'string' || !grid.id) errors.push(`${relative(file)}: grid requires id`);
    else if (gridIds.has(grid.id)) errors.push(`${relative(file)}: duplicate grid ${grid.id}`);
    else gridIds.add(grid.id);
    for (const chunk of grid.chunks ?? []) {
      if (!Array.isArray(chunk.tiles)) errors.push(`${relative(file)}: chunk on grid ${grid.id} requires tiles`);
    }
  }
  for (const entity of map.entities ?? []) {
    if (!prototypes.has(entity.prototype)) errors.push(`${relative(file)}: map entity references missing prototype ${entity.prototype}`);
    if (entity.grid && !gridIds.has(entity.grid)) errors.push(`${relative(file)}: map entity references missing grid ${entity.grid}`);
  }
}

async function validateRsi(metaFile) {
  let meta;
  try {
    meta = JSON.parse(await readFile(metaFile, 'utf8'));
  } catch (error) {
    errors.push(`${relative(metaFile)}: ${formatError(error)}`);
    return;
  }
  const size = Array.isArray(meta.size) ? meta.size : [meta.size?.x, meta.size?.y];
  if (!Number.isInteger(size[0]) || !Number.isInteger(size[1]) || size[0] <= 0 || size[1] <= 0) {
    errors.push(`${relative(metaFile)}: invalid RSI frame size`);
  }
  const names = new Set();
  for (const state of meta.states ?? []) {
    if (typeof state.name !== 'string' || !state.name) {
      errors.push(`${relative(metaFile)}: RSI state without name`);
      continue;
    }
    if (names.has(state.name)) errors.push(`${relative(metaFile)}: duplicate RSI state ${state.name}`);
    names.add(state.name);
    const directions = state.directions ?? state.dirs ?? 1;
    if (![1, 4, 8].includes(directions)) errors.push(`${relative(metaFile)}: state ${state.name} directions must be 1, 4 or 8`);
    const image = path.join(path.dirname(metaFile), `${state.name}.png`);
    try { await access(image); } catch { errors.push(`${relative(metaFile)}: missing image ${state.name}.png`); }
    if (state.delays && (!Array.isArray(state.delays) || state.delays.length !== directions)) {
      errors.push(`${relative(metaFile)}: state ${state.name} delays must contain ${directions} direction arrays`);
    }
  }
}

async function walk(directory, predicate) {
  const files = [];
  let entries;
  try { entries = await readdir(directory, { withFileTypes: true }); }
  catch { return files; }
  for (const entry of entries) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await walk(full, predicate));
    else if (entry.isFile() && predicate(full)) files.push(full);
  }
  return files.sort();
}

function resourcePath(resource) {
  if (!resource.startsWith('/Resources/')) return null;
  return path.join(gameRoot, 'resources', resource.slice('/Resources/'.length));
}

function relative(file) { return path.relative(root, file).replaceAll('\\', '/'); }
function formatError(error) { return error instanceof Error ? error.message : String(error); }

function resolveGameRoot(workspaceRoot) {
  const configured = process.env.HONKNET_GAME_ROOT;
  if (!configured) return path.join(workspaceRoot, 'examples/minimal-game');
  return path.isAbsolute(configured) ? configured : path.resolve(workspaceRoot, configured);
}

import { validateHuiDocument, type HuiNode } from '@honknet/hui-runtime';
import YAML from 'yaml';

import { detectFileKind } from './fileSystem';
import { StudioProject } from './project';
import type { ValidationMessage } from './types';

export type ProjectValidationResult = {
  messages: ValidationMessage[];
  checkedFiles: number;
  durationMs: number;
};

type ParsedFile = {
  path: string;
  kind: ReturnType<typeof detectFileKind>;
  value: unknown;
};

export async function validateProject(project: StudioProject): Promise<ProjectValidationResult> {
  const start = performance.now();
  const files = await project.files.listFiles();
  const fileSet = new Set(files.map(normalizePath));
  const messages: ValidationMessage[] = [];
  const parsedFiles: ParsedFile[] = [];
  const prototypeDefinitions = new Map<string, string>();
  const localizationByLocale = new Map<string, Map<string, string>>();

  for (const path of files) {
    const kind = detectFileKind(path);
    try {
      if (kind === 'localization') {
        const source = await project.readText(path);
        validateFtl(path, source, localizationByLocale, messages);
      } else if (kind === 'rsi' && path.toLowerCase().endsWith('/meta.json')) {
        parsedFiles.push({ path, kind, value: JSON.parse(await project.readText(path)) as unknown });
      } else if (/\.(?:ya?ml|json)$/i.test(path)) {
        const source = await project.readText(path);
        const value = path.toLowerCase().endsWith('.json') ? JSON.parse(source) as unknown : YAML.parse(source) as unknown;
        parsedFiles.push({ path, kind, value });
        collectPrototypeIds(path, value, prototypeDefinitions, messages);
      }
    } catch (error) {
      messages.push({ severity: 'error', path, message: formatError(error) });
    }
  }

  const localizationKeys = new Set<string>();
  for (const keys of localizationByLocale.values()) for (const key of keys.keys()) localizationKeys.add(key);

  for (const parsed of parsedFiles) {
    try {
      switch (parsed.kind) {
        case 'hui':
          validateHui(parsed.path, parsed.value, project, fileSet, localizationKeys, messages);
          break;
        case 'prototype':
          validatePrototypeFile(parsed.path, parsed.value, prototypeDefinitions, project, fileSet, localizationKeys, messages);
          break;
        case 'component-schema':
          validateSchema(parsed.path, parsed.value, messages);
          break;
        case 'map':
          validateMap(parsed.path, parsed.value, prototypeDefinitions, messages);
          break;
        case 'behavior':
          validateBehavior(parsed.path, parsed.value, prototypeDefinitions, messages);
          break;
        case 'state-machine':
          validateStateMachine(parsed.path, parsed.value, messages);
          break;
        case 'rsi':
          await validateRsi(parsed.path, parsed.value, project, messages);
          break;
        default:
          break;
      }
    } catch (error) {
      messages.push({ severity: 'error', path: parsed.path, message: formatError(error) });
    }
  }

  validateLocaleCoverage(localizationByLocale, messages);

  const errorCount = messages.filter((message) => message.severity === 'error').length;
  const warningCount = messages.filter((message) => message.severity === 'warning').length;
  messages.unshift({
    severity: errorCount === 0 ? 'info' : 'error',
    message: `Project validation: ${files.length} files, ${errorCount} errors, ${warningCount} warnings.`,
  });
  return { messages, checkedFiles: files.length, durationMs: performance.now() - start };
}

function validateFtl(
  path: string,
  source: string,
  locales: Map<string, Map<string, string>>,
  messages: ValidationMessage[],
): void {
  const locale = localeFromPath(path);
  const keys = locales.get(locale) ?? new Map<string, string>();
  locales.set(locale, keys);
  let lineNumber = 0;
  for (const line of source.split(/\r?\n/)) {
    lineNumber += 1;
    const match = /^\s*([A-Za-z0-9_.-]+)\s*=/.exec(line);
    const key = match?.[1];
    if (!key) continue;
    const existing = keys.get(key);
    if (existing) messages.push({ severity: 'error', path, line: lineNumber, message: `Duplicate localization key ${key}; first defined in ${existing}.` });
    else keys.set(key, path);
  }
}

function collectPrototypeIds(
  path: string,
  value: unknown,
  definitions: Map<string, string>,
  messages: ValidationMessage[],
): void {
  for (const document of asDocuments(value)) {
    if (!isRecord(document) || document.type !== 'entity' || typeof document.id !== 'string') continue;
    const existing = definitions.get(document.id);
    if (existing) messages.push({ severity: 'error', path, message: `Duplicate prototype ID ${document.id}; first defined in ${existing}.` });
    else definitions.set(document.id, path);
  }
}

function validateHui(
  path: string,
  value: unknown,
  project: StudioProject,
  files: Set<string>,
  localizationKeys: Set<string>,
  messages: ValidationMessage[],
): void {
  if (!isRecord(value) || typeof value.type !== 'string') {
    messages.push({ severity: 'error', path, message: 'HUI root must be an object with a type.' });
    return;
  }
  const root = value as HuiNode;
  for (const issue of validateHuiDocument(root)) {
    if (issue.severity === 'info') continue;
    messages.push({ severity: issue.severity, path, message: `${issue.path}: ${issue.message}` });
  }
  walkRecords(root, (node, nodePath) => {
    for (const property of ['source', 'icon'] as const) {
      const resource = node[property];
      if (typeof resource === 'string' && !resource.startsWith('$')) validateResource(path, `${nodePath}.${property}`, resource, project, files, messages);
    }
    for (const property of ['text', 'title', 'placeholder', 'tooltip'] as const) {
      const key = node[property];
      if (typeof key === 'string' && looksLikeLocalizationKey(key) && !localizationKeys.has(key)) {
        messages.push({ severity: 'warning', path, message: `${nodePath}.${property} references missing localization key ${key}.` });
      }
    }
  });
}

function validatePrototypeFile(
  path: string,
  value: unknown,
  definitions: Map<string, string>,
  project: StudioProject,
  files: Set<string>,
  localizationKeys: Set<string>,
  messages: ValidationMessage[],
): void {
  for (const document of asDocuments(value)) {
    if (!isRecord(document) || document.type !== 'entity') continue;
    const id = typeof document.id === 'string' ? document.id : '';
    if (!id.trim()) messages.push({ severity: 'error', path, message: 'Entity prototype has no ID.' });
    if (typeof document.parent === 'string' && !definitions.has(document.parent)) {
      messages.push({ severity: 'warning', path, message: `${id || 'prototype'} references unknown parent ${document.parent}.` });
    }
    for (const property of ['name', 'description'] as const) {
      const key = document[property];
      if (typeof key === 'string' && looksLikeLocalizationKey(key) && !localizationKeys.has(key)) {
        messages.push({ severity: 'warning', path, message: `${id || 'prototype'}.${property} references missing localization key ${key}.` });
      }
    }
    if (!Array.isArray(document.components)) {
      messages.push({ severity: 'warning', path, message: `${id || 'prototype'} has no component list.` });
      continue;
    }
    const components = document.components.filter(isRecord);
    const types = new Set<string>();
    for (const component of components) {
      if (typeof component.type !== 'string') {
        messages.push({ severity: 'error', path, message: `${id || 'prototype'} has a component without type.` });
        continue;
      }
      if (types.has(component.type)) messages.push({ severity: 'warning', path, message: `${id || 'prototype'} contains duplicate component ${component.type}.` });
      types.add(component.type);
      const fields = isRecord(component.fields) ? component.fields : component;
      if (component.type === 'Sprite' && Array.isArray(fields.layers)) {
        for (const [index, layer] of fields.layers.filter(isRecord).entries()) {
          const resource = layer.sprite ?? layer.texture;
          if (typeof resource === 'string') validateResource(path, `${id}.Sprite.layers[${index}]`, resource, project, files, messages);
        }
      }
    }
  }
}

function validateSchema(path: string, value: unknown, messages: ValidationMessage[]): void {
  if (!isRecord(value) || value.type !== 'component-schema' || typeof value.id !== 'string' || !isRecord(value.fields)) {
    messages.push({ severity: 'error', path, message: 'Component schema requires type, id and fields.' });
    return;
  }
  const supported = new Set(['string', 'bool', 'boolean', 'int', 'integer', 'float', 'number', 'entity', 'prototype', 'resource', 'vector2', 'vector3', 'color', 'array', 'object']);
  for (const [name, field] of Object.entries(value.fields)) {
    if (!isRecord(field) || typeof field.type !== 'string') {
      messages.push({ severity: 'error', path, message: `Schema field ${name} has no type.` });
      continue;
    }
    if (!supported.has(field.type)) messages.push({ severity: 'warning', path, message: `Schema field ${name} uses unknown type ${field.type}.` });
    if (typeof field.minimum === 'number' && typeof field.maximum === 'number' && field.minimum > field.maximum) {
      messages.push({ severity: 'error', path, message: `Schema field ${name} has minimum greater than maximum.` });
    }
  }
}

function validateMap(path: string, value: unknown, prototypes: Map<string, string>, messages: ValidationMessage[]): void {
  if (!isRecord(value) || !isRecord(value.map)) {
    messages.push({ severity: 'error', path, message: 'Map document has no map root.' });
    return;
  }
  const map = value.map;
  if (typeof map.id !== 'string' || !map.id.trim()) messages.push({ severity: 'error', path, message: 'Map has no ID.' });
  if (!Array.isArray(map.grids) || map.grids.length === 0) messages.push({ severity: 'error', path, message: 'Map has no grids.' });
  for (const [gridIndex, gridValue] of (Array.isArray(map.grids) ? map.grids : []).entries()) {
    if (!isRecord(gridValue)) continue;
    if (typeof gridValue.id !== 'string') messages.push({ severity: 'error', path, message: `Grid ${gridIndex} has no ID.` });
    for (const chunk of (Array.isArray(gridValue.chunks) ? gridValue.chunks : []).filter(isRecord)) {
      if (!Array.isArray(chunk.tiles) || chunk.tiles.length === 0) messages.push({ severity: 'error', path, message: `Grid ${String(gridValue.id ?? gridIndex)} contains an empty chunk.` });
      const widths = (Array.isArray(chunk.tiles) ? chunk.tiles : []).filter(Array.isArray).map((row) => row.length);
      if (new Set(widths).size > 1) messages.push({ severity: 'error', path, message: `Grid ${String(gridValue.id ?? gridIndex)} has inconsistent tile row widths.` });
    }
  }
  for (const [index, entity] of (Array.isArray(map.entities) ? map.entities : []).filter(isRecord).entries()) {
    if (typeof entity.prototype !== 'string') messages.push({ severity: 'error', path, message: `Map entity ${index} has no prototype.` });
    else if (!prototypes.has(entity.prototype)) messages.push({ severity: 'warning', path, message: `Map entity ${index} references unknown prototype ${entity.prototype}.` });
    if (!Array.isArray(entity.position) || entity.position.length < 2 || entity.position.some((coordinate) => typeof coordinate !== 'number')) {
      messages.push({ severity: 'error', path, message: `Map entity ${index} has an invalid position.` });
    }
  }
}

function validateBehavior(path: string, value: unknown, prototypes: Map<string, string>, messages: ValidationMessage[]): void {
  if (!isRecord(value) || typeof value.id !== 'string' || !isRecord(value.events)) {
    messages.push({ severity: 'error', path, message: 'Behavior graph requires id and events.' });
    return;
  }
  walkRecords(value.events, (node, nodePath) => {
    if (typeof node.node !== 'string') return;
    if ((node.node === 'Repeat' || node.node === 'ForEachEntity') && (typeof node.maxIterations !== 'number' || node.maxIterations < 1)) {
      messages.push({ severity: 'error', path, message: `${nodePath}.${node.node} must define a positive maxIterations.` });
    }
    if (node.node === 'SpawnEntity' && typeof node.prototype === 'string' && !prototypes.has(node.prototype)) {
      messages.push({ severity: 'warning', path, message: `${nodePath} references unknown prototype ${node.prototype}.` });
    }
  });
}

function validateStateMachine(path: string, value: unknown, messages: ValidationMessage[]): void {
  if (!isRecord(value) || typeof value.id !== 'string' || !isRecord(value.states)) {
    messages.push({ severity: 'error', path, message: 'State machine requires id and a states object.' });
    return;
  }
  const ids = new Set(Object.keys(value.states));
  if (ids.size === 0) messages.push({ severity: 'error', path, message: 'State machine must contain at least one state.' });
  if (typeof value.initial !== 'string' || !ids.has(value.initial)) messages.push({ severity: 'error', path, message: 'State machine initial state does not exist.' });
  for (const [stateId, rawState] of Object.entries(value.states)) {
    if (!isRecord(rawState)) {
      messages.push({ severity: 'error', path, message: `State ${stateId} must be an object.` });
      continue;
    }
    for (const [index, transition] of (Array.isArray(rawState.transitions) ? rawState.transitions : []).filter(isRecord).entries()) {
      if (typeof transition.target !== 'string' || !ids.has(transition.target)) messages.push({ severity: 'error', path, message: `${stateId} transition ${index} has an unknown target state.` });
      if (typeof transition.event !== 'string' || !transition.event.trim()) messages.push({ severity: 'error', path, message: `${stateId} transition ${index} has no event.` });
    }
  }
}

async function validateRsi(path: string, value: unknown, project: StudioProject, messages: ValidationMessage[]): Promise<void> {
  if (!isRecord(value) || !isRecord(value.size) || typeof value.size.x !== 'number' || typeof value.size.y !== 'number' || value.size.x < 1 || value.size.y < 1) {
    messages.push({ severity: 'error', path, message: 'RSI meta.json has no valid positive frame size.' });
    return;
  }
  const directory = path.slice(0, -'/meta.json'.length);
  const entries = await project.files.listDirectory(directory);
  const files = new Set(entries.filter((entry) => entry.kind === 'file').map((entry) => entry.name));
  const stateNames = new Set<string>();
  for (const [index, state] of (Array.isArray(value.states) ? value.states : []).filter(isRecord).entries()) {
    if (typeof state.name !== 'string' || !state.name.trim()) {
      messages.push({ severity: 'error', path, message: `RSI state ${index} has no name.` });
      continue;
    }
    if (stateNames.has(state.name)) messages.push({ severity: 'error', path, message: `Duplicate RSI state ${state.name}.` });
    stateNames.add(state.name);
    if (!files.has(`${state.name}.png`)) messages.push({ severity: 'error', path, message: `RSI state ${state.name} has no ${state.name}.png.` });
    const directions = Number(state.directions ?? 1);
    if (![1, 4, 8].includes(directions)) messages.push({ severity: 'warning', path, message: `RSI state ${state.name} uses non-standard direction count ${directions}.` });
  }
}

function validateResource(
  ownerPath: string,
  propertyPath: string,
  resource: string,
  project: StudioProject,
  files: Set<string>,
  messages: ValidationMessage[],
): void {
  if (/^(?:https?:|data:|blob:)/i.test(resource)) return;
  const resolved = normalizePath(project.resolveProjectPath(resource));
  const exists = files.has(resolved) || files.has(`${resolved}/meta.json`);
  if (!exists) messages.push({ severity: 'warning', path: ownerPath, message: `${propertyPath} references missing resource ${resource}.` });
}

function validateLocaleCoverage(locales: Map<string, Map<string, string>>, messages: ValidationMessage[]): void {
  if (locales.size < 2) return;
  const allKeys = new Set<string>();
  for (const keys of locales.values()) for (const key of keys.keys()) allKeys.add(key);
  for (const [locale, keys] of locales) {
    const missing = [...allKeys].filter((key) => !keys.has(key));
    if (missing.length > 0) messages.push({
      severity: 'warning',
      path: `localization/${locale}`,
      message: `${missing.length} localization keys are missing: ${missing.slice(0, 8).join(', ')}${missing.length > 8 ? '…' : ''}`,
    });
  }
}

function walkRecords(value: unknown, visitor: (record: Record<string, unknown>, path: string) => void, path = 'root'): void {
  if (Array.isArray(value)) {
    value.forEach((entry, index) => walkRecords(entry, visitor, `${path}[${index}]`));
    return;
  }
  if (!isRecord(value)) return;
  visitor(value, path);
  for (const [key, child] of Object.entries(value)) walkRecords(child, visitor, `${path}.${key}`);
}

function asDocuments(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [value];
}

function localeFromPath(path: string): string {
  const segments = normalizePath(path).split('/');
  const index = segments.findIndex((segment) => segment.toLowerCase() === 'localization');
  const locale = index >= 0 ? segments[index + 1] : undefined;
  return locale ?? 'default';
}

function looksLikeLocalizationKey(value: string): boolean {
  return !value.startsWith('$') && !value.startsWith('/') && /^[A-Za-z0-9_.-]+$/.test(value) && value.includes('-');
}

function normalizePath(path: string): string {
  return path.replaceAll('\\', '/').replace(/^\.\//, '').replace(/^\//, '');
}

function formatError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

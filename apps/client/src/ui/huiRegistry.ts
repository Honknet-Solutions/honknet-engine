import YAML from 'yaml';

import {
  validateHuiDocument,
  type HuiNode,
  type HuiValidationIssue,
} from './hui';

type HuiManifest = {
  version: number;
  entries: Array<{ id: string; path: string }>;
};

export class HuiRegistry {
  private readonly paths = new Map<string, string>();
  private readonly cache = new Map<string, Promise<HuiNode>>();

  public async initialize(): Promise<void> {
    const response = await fetch('/Resources/UI/manifest.json', { cache: 'no-cache' });
    if (!response.ok) {
      return;
    }
    const manifest = await response.json() as HuiManifest;
    for (const entry of manifest.entries) {
      this.paths.set(entry.id, entry.path);
      const fileId = entry.path
        .split('/')
        .at(-1)
        ?.replace(/\.hui(?:\.ya?ml)?$/i, '');
      if (fileId) this.paths.set(fileId, entry.path);
    }
  }

  public load(key: string): Promise<HuiNode> {
    const existing = this.cache.get(key);
    if (existing) return existing;
    const promise = this.loadUncached(key);
    this.cache.set(key, promise);
    return promise;
  }

  public invalidate(key?: string): void {
    if (key) this.cache.delete(key);
    else this.cache.clear();
  }

  private async loadUncached(key: string): Promise<HuiNode> {
    const path = this.paths.get(key) ?? key;
    const response = await fetch(path, { cache: 'no-cache' });
    if (!response.ok) {
      throw new Error(`Failed to load HUI '${key}': HTTP ${response.status}`);
    }
    const parsed = YAML.parse(await response.text()) as unknown;
    if (!isHuiNode(parsed)) {
      throw new Error(`HUI '${key}' is not a valid document root`);
    }
    const issues = validateHuiDocument(parsed);
    const errors = issues.filter((issue) => issue.severity === 'error');
    if (errors.length > 0) {
      throw new Error(formatIssues(key, errors));
    }
    return parsed;
  }
}

function isHuiNode(value: unknown): value is HuiNode {
  return typeof value === 'object'
    && value !== null
    && typeof (value as { type?: unknown }).type === 'string';
}

function formatIssues(key: string, issues: readonly HuiValidationIssue[]): string {
  return `HUI '${key}' failed validation:\n${issues
    .map((issue) => `${issue.path}: ${issue.message}`)
    .join('\n')}`;
}

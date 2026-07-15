import YAML from 'yaml';

import { BrowserProjectFileSystem, detectFileKind, joinPath } from './fileSystem';
import type {
  AssetSummary,
  ComponentSchemaSummary,
  ProjectMetadata,
  ProjectTreeNode,
  PrototypeSummary,
  RsiMeta,
  RsiStateSummary,
  TileDefinitionSummary,
} from './types';

const FALLBACK_TILES: TileDefinitionSummary[] = [
  { id: 'floor', label: 'Floor', color: '#283843', collision: false, category: 'Floors' },
  { id: 'wall', label: 'Wall', color: '#7a8992', collision: true, category: 'Structures' },
  { id: 'road', label: 'Road', color: '#20252b', collision: false, category: 'Exterior' },
  { id: 'sidewalk', label: 'Sidewalk', color: '#596269', collision: false, category: 'Exterior' },
  { id: 'grass', label: 'Grass', color: '#294d35', collision: false, category: 'Nature' },
  { id: 'water', label: 'Water', color: '#164d70', collision: true, category: 'Nature' },
  { id: 'sand', label: 'Sand', color: '#826d3d', collision: false, category: 'Nature' },
  { id: 'void', label: 'Void', color: '#07090c', collision: true, category: 'Utility' },
];

export class StudioProject {
  public readonly files = new BrowserProjectFileSystem();

  private tree: ProjectTreeNode | null = null;
  private metadata: ProjectMetadata = emptyMetadata();
  private readonly objectUrls = new Map<string, string>();

  public get name(): string {
    return this.files.projectName;
  }

  public get isOpen(): boolean {
    return this.files.isOpen;
  }

  public get projectTree(): ProjectTreeNode | null {
    return this.tree;
  }

  public get projectMetadata(): ProjectMetadata {
    return this.metadata;
  }

  public async hasRecentProject(): Promise<boolean> {
    return this.files.hasRecentProject();
  }

  public async open(): Promise<void> {
    this.releaseObjectUrls();
    await this.files.openProject();
    await this.refresh();
  }

  public async reopenRecent(): Promise<void> {
    this.releaseObjectUrls();
    await this.files.reopenRecentProject();
    await this.refresh();
  }

  public async refresh(): Promise<void> {
    // Keep asset object URLs stable while an editor is open. Saving an unrelated
    // YAML/FTL document refreshes project metadata; revoking every image URL at
    // that point would blank RSI, prototype and HUI previews until the editor is
    // reopened. URLs are released when another project is opened.
    this.tree = await this.files.buildTree();
    this.metadata = await this.loadMetadata();
  }

  public async readText(path: string): Promise<string> {
    return this.files.readText(path);
  }

  public async readFile(path: string): Promise<File> {
    return this.files.readFile(path);
  }

  public async writeText(path: string, content: string): Promise<void> {
    await this.files.writeText(path, content);
  }

  public async getObjectUrl(path: string): Promise<string> {
    const normalized = this.resolveProjectPath(path);
    const cached = this.objectUrls.get(normalized);
    if (cached) return cached;
    const file = await this.files.readFile(normalized);
    const url = URL.createObjectURL(file);
    this.objectUrls.set(normalized, url);
    return url;
  }

  public releaseObjectUrls(): void {
    for (const url of this.objectUrls.values()) URL.revokeObjectURL(url);
    this.objectUrls.clear();
  }

  public resolveProjectPath(path: string): string {
    const normalized = path.replaceAll('\\', '/').replace(/^\.\//, '');
    if (normalized.startsWith('/Resources/')) {
      return joinPath('examples/minimal-game/resources', normalized.slice('/Resources/'.length));
    }
    if (normalized.startsWith('Resources/')) {
      return joinPath('examples/minimal-game/resources', normalized.slice('Resources/'.length));
    }
    return normalized.replace(/^\//, '');
  }

  public async readRsi(path: string): Promise<{ meta: RsiMeta; states: RsiStateSummary[] }> {
    const normalized = path.replace(/\/$/, '');
    const meta = await this.files.readJson<RsiMeta>(joinPath(normalized, 'meta.json'));
    const entries = await this.files.listDirectory(normalized);
    const images = new Set(
      entries
        .filter((entry) => entry.kind === 'file' && /\.png$/i.test(entry.name))
        .map((entry) => entry.name),
    );
    const states = (meta.states ?? []).map((state) => ({
      name: state.name,
      imagePath: joinPath(normalized, images.has(`${state.name}.png`) ? `${state.name}.png` : [...images][0] ?? `${state.name}.png`),
      directions: Math.max(1, state.directions ?? 1),
      delays: normalizeDelays(state.delays),
    }));
    return { meta, states };
  }

  private async loadMetadata(): Promise<ProjectMetadata> {
    const files = await this.files.listFiles();
    const prototypeSummaries = new Map<string, PrototypeSummary>();
    const schemas = new Map<string, ComponentSchemaSummary>();
    const localizationKeys = new Set<string>();
    const assetSummaries: AssetSummary[] = [];
    const rsiDirectories = new Set<string>();
    const tiles = new Map<string, TileDefinitionSummary>();

    for (const fallback of FALLBACK_TILES) tiles.set(fallback.id, { ...fallback });

    await Promise.all(
      files.map(async (path) => {
        const lower = path.toLowerCase();
        const rsiMatch = /^(.*?\.rsi)\//i.exec(path);
        if (rsiMatch?.[1]) rsiDirectories.add(rsiMatch[1]);

        const asset = classifyAsset(path);
        if (asset) assetSummaries.push(asset);

        const kind = detectFileKind(path);
        if (kind !== 'prototype' && kind !== 'component-schema' && kind !== 'localization') return;

        try {
          const text = await this.files.readText(path);
          if (kind === 'prototype') {
            const parsed = YAML.parse(text) as unknown;
            const documents = Array.isArray(parsed) ? parsed : [parsed];
            for (const document of documents) {
              if (!isRecord(document)) continue;
              collectPrototype(document, prototypeSummaries, tiles);
            }
          } else if (kind === 'component-schema') {
            const parsed = YAML.parse(text) as unknown;
            if (!isRecord(parsed) || parsed.type !== 'component-schema' || typeof parsed.id !== 'string') return;
            const fields: ComponentSchemaSummary['fields'] = {};
            if (isRecord(parsed.fields)) {
              for (const [fieldName, rawField] of Object.entries(parsed.fields)) {
                if (!isRecord(rawField) || typeof rawField.type !== 'string') continue;
                const fieldSummary: ComponentSchemaSummary['fields'][string] = {
                  type: rawField.type,
                  defaultValue: rawField.default,
                  required: rawField.required === true,
                };
                if (typeof rawField.minimum === 'number') fieldSummary.minimum = rawField.minimum;
                if (typeof rawField.maximum === 'number') fieldSummary.maximum = rawField.maximum;
                if (Array.isArray(rawField.options)) fieldSummary.options = rawField.options.map(String);
                if (typeof rawField.description === 'string') fieldSummary.description = rawField.description;
                fields[fieldName] = fieldSummary;
              }
            }
            const replication = isRecord(parsed.replication) && typeof parsed.replication.mode === 'string'
              ? parsed.replication.mode
              : undefined;
            const summary: ComponentSchemaSummary = { id: parsed.id, fields };
            if (replication !== undefined) summary.replicationMode = replication;
            schemas.set(parsed.id, summary);
          } else {
            for (const line of text.split(/\r?\n/)) {
              const match = /^\s*([A-Za-z0-9_.-]+)\s*=/.exec(line);
              if (match?.[1]) localizationKeys.add(match[1]);
            }
          }
        } catch {
          // Invalid content is reported by its editor and project validation.
        }

        // Keep TypeScript from treating lower as intentionally unused after future filters.
        void lower;
      }),
    );

    assetSummaries.sort((left, right) => left.path.localeCompare(right.path, undefined, { numeric: true }));
    const sortedPrototypes = [...prototypeSummaries.values()].sort((left, right) => left.id.localeCompare(right.id));
    return {
      prototypes: sortedPrototypes.filter((prototype) => !prototype.abstract).map((prototype) => prototype.id),
      prototypeSummaries: sortedPrototypes,
      componentSchemas: [...schemas.values()].sort((left, right) => left.id.localeCompare(right.id)),
      localizationKeys: [...localizationKeys].sort((left, right) => left.localeCompare(right)),
      assets: assetSummaries.map((asset) => asset.path),
      assetSummaries,
      rsiDirectories: [...rsiDirectories].sort((left, right) => left.localeCompare(right)),
      tiles: [...tiles.values()].sort((left, right) => left.category.localeCompare(right.category) || left.label.localeCompare(right.label)),
    };
  }
}

function emptyMetadata(): ProjectMetadata {
  return {
    prototypes: [],
    prototypeSummaries: [],
    componentSchemas: [],
    localizationKeys: [],
    assets: [],
    assetSummaries: [],
    rsiDirectories: [],
    tiles: FALLBACK_TILES.map((tile) => ({ ...tile })),
  };
}

function collectPrototype(
  document: Record<string, unknown>,
  prototypes: Map<string, PrototypeSummary>,
  tiles: Map<string, TileDefinitionSummary>,
): void {
  const id = typeof document.id === 'string' ? document.id : null;
  if (!id) return;

  const components = Array.isArray(document.components) ? document.components.filter(isRecord) : [];
  const spriteComponent = components.find((component) => component.type === 'Sprite');
  const layer = isRecord(spriteComponent) && Array.isArray(spriteComponent.layers)
    ? spriteComponent.layers.find(isRecord)
    : undefined;
  const fields = isRecord(spriteComponent) && isRecord(spriteComponent.fields) ? spriteComponent.fields : undefined;
  const fieldLayer = fields && Array.isArray(fields.layers) ? fields.layers.find(isRecord) : undefined;
  const selectedLayer = layer ?? fieldLayer;
  const sprite = selectedLayer && typeof (selectedLayer.sprite ?? selectedLayer.texture) === 'string'
    ? String(selectedLayer.sprite ?? selectedLayer.texture)
    : undefined;
  const state = selectedLayer && typeof selectedLayer.state === 'string' ? selectedLayer.state : undefined;

  if (document.type === 'entity') {
    const summary: PrototypeSummary = {
      id,
      abstract: document.abstract === true,
    };
    if (typeof document.parent === 'string') summary.parent = document.parent;
    if (typeof document.name === 'string') summary.name = document.name;
    if (sprite !== undefined) summary.sprite = sprite;
    if (state !== undefined) summary.state = state;
    prototypes.set(id, summary);
  }

  const tileComponent = components.find((component) => component.type === 'Tile' || component.type === 'TileDefinition');
  if (document.type === 'tile' || document.type === 'tile-definition' || tileComponent) {
    const tileFields = isRecord(tileComponent?.fields) ? tileComponent.fields : tileComponent;
    const color = typeof document.color === 'string'
      ? document.color
      : isRecord(tileFields) && typeof tileFields.color === 'string'
        ? tileFields.color
        : hashColor(id);
    const tile: TileDefinitionSummary = {
      id,
      label: typeof document.name === 'string' ? document.name : id,
      color,
      collision: document.collision === true || (isRecord(tileFields) && tileFields.collision === true),
      category: typeof document.category === 'string' ? document.category : 'Project',
    };
    if (sprite !== undefined) tile.sprite = sprite;
    if (state !== undefined) tile.state = state;
    tiles.set(id, tile);
  }
}

function classifyAsset(path: string): AssetSummary | null {
  const extension = path.split('.').at(-1)?.toLowerCase() ?? '';
  let kind: AssetSummary['kind'] | null = null;
  if (['png', 'webp', 'jpg', 'jpeg', 'gif', 'svg'].includes(extension)) kind = 'image';
  else if (['ogg', 'wav', 'mp3', 'flac'].includes(extension)) kind = 'audio';
  else if (['woff', 'woff2', 'ttf', 'otf'].includes(extension)) kind = 'font';
  else if (path.toLowerCase().includes('.rsi/')) kind = 'rsi';
  else if (['json'].includes(extension)) kind = 'other';
  if (!kind) return null;
  return { path, kind, extension };
}

function normalizeDelays(value: unknown): number[][] {
  if (!Array.isArray(value)) return [[1]];
  if (value.length === 0) return [[1]];
  if (value.every((entry) => typeof entry === 'number')) return [value.map((entry) => Math.max(0.01, Number(entry)))];
  return value
    .filter(Array.isArray)
    .map((direction) => direction.map((entry) => Math.max(0.01, Number(entry) || 0.1)));
}

function hashColor(value: string): string {
  let hash = 0;
  for (const character of value) hash = ((hash << 5) - hash + character.charCodeAt(0)) | 0;
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue} 28% 38%)`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

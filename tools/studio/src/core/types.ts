export type StudioFileKind =
  | 'map'
  | 'hui'
  | 'prototype'
  | 'component-schema'
  | 'behavior'
  | 'state-machine'
  | 'localization'
  | 'rsi'
  | 'asset'
  | 'script'
  | 'unknown';

export type ProjectTreeNode = {
  name: string;
  path: string;
  kind: 'file' | 'directory';
  fileKind: StudioFileKind;
  children: ProjectTreeNode[];
};

export type ValidationMessage = {
  severity: 'info' | 'warning' | 'error';
  message: string;
  path?: string;
  line?: number;
  column?: number;
};

export type EditorCommandState = {
  canUndo: boolean;
  canRedo: boolean;
  dirty: boolean;
};

export type PrototypeSummary = {
  id: string;
  parent?: string;
  name?: string;
  abstract: boolean;
  sprite?: string;
  state?: string;
};

export type TileDefinitionSummary = {
  id: string;
  label: string;
  color: string;
  sprite?: string;
  state?: string;
  collision: boolean;
  category: string;
};

export type AssetSummary = {
  path: string;
  kind: 'image' | 'audio' | 'rsi' | 'font' | 'other';
  extension: string;
};

export type ProjectMetadata = {
  prototypes: string[];
  prototypeSummaries: PrototypeSummary[];
  componentSchemas: ComponentSchemaSummary[];
  localizationKeys: string[];
  assets: string[];
  assetSummaries: AssetSummary[];
  rsiDirectories: string[];
  tiles: TileDefinitionSummary[];
};

export type ComponentSchemaSummary = {
  id: string;
  fields: Record<string, ComponentSchemaFieldSummary>;
  replicationMode?: string;
};

export type ComponentSchemaFieldSummary = {
  type: string;
  defaultValue: unknown;
  minimum?: number;
  maximum?: number;
  options?: string[];
  required: boolean;
  description?: string;
};

export type RsiMeta = {
  version?: number;
  size?: { x: number; y: number };
  states?: Array<{
    name: string;
    directions?: number;
    delays?: number[][];
  }>;
  license?: string;
  copyright?: string;
};

export type RsiStateSummary = {
  name: string;
  imagePath: string;
  directions: number;
  delays: number[][];
};

export type RuntimeEntity = {
  netId: number;
  prototype: string;
  position: { x: number; y: number; z: number };
  components: unknown[];
};

export interface StudioEditor {
  readonly kind: StudioFileKind;
  readonly title: string;
  mount(container: HTMLElement, inspector: HTMLElement): void;
  unmount(): void;
  serialize(): string;
  validate(): ValidationMessage[];
  isDirty(): boolean;
  markSaved(): void;
  undo(): void;
  redo(): void;
  getCommandState(): EditorCommandState;
  showSource?(enabled: boolean): void;
}

export type CreateDocumentRequest = {
  kind: Exclude<StudioFileKind, 'rsi' | 'asset' | 'script' | 'unknown'>;
  name: string;
  directory: string;
  width?: number;
  height?: number;
};

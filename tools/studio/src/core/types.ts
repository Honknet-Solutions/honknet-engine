export type StudioFileKind =
  | 'map'
  | 'hui'
  | 'prototype'
  | 'component-schema'
  | 'behavior'
  | 'localization'
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
};

export type EditorCommandState = {
  canUndo: boolean;
  canRedo: boolean;
  dirty: boolean;
};

export type ProjectMetadata = {
  prototypes: string[];
  componentSchemas: ComponentSchemaSummary[];
  localizationKeys: string[];
};

export type ComponentSchemaSummary = {
  id: string;
  fields: Record<string, ComponentSchemaFieldSummary>;
};

export type ComponentSchemaFieldSummary = {
  type: string;
  defaultValue: unknown;
  minimum?: number;
  maximum?: number;
  required: boolean;
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
  kind: Exclude<StudioFileKind, 'asset' | 'script' | 'unknown'>;
  name: string;
  directory: string;
  width?: number;
  height?: number;
};

import YAML from 'yaml';

import { BrowserProjectFileSystem, detectFileKind } from './fileSystem';
import type {
  ComponentSchemaSummary,
  ProjectMetadata,
  ProjectTreeNode,
} from './types';

export class StudioProject {
  public readonly files = new BrowserProjectFileSystem();

  private tree: ProjectTreeNode | null = null;
  private metadata: ProjectMetadata = {
    prototypes: [],
    componentSchemas: [],
    localizationKeys: [],
  };

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

  public async open(): Promise<void> {
    await this.files.openProject();
    await this.refresh();
  }

  public async refresh(): Promise<void> {
    this.tree = await this.files.buildTree();
    this.metadata = await this.loadMetadata();
  }

  public async readText(path: string): Promise<string> {
    return this.files.readText(path);
  }

  public async writeText(path: string, content: string): Promise<void> {
    await this.files.writeText(path, content);
  }

  private async loadMetadata(): Promise<ProjectMetadata> {
    const files = await this.files.listFiles();
    const prototypeIds = new Set<string>();
    const schemas = new Map<string, ComponentSchemaSummary>();
    const localizationKeys = new Set<string>();

    await Promise.all(
      files.map(async (path) => {
        const kind = detectFileKind(path);
        if (kind !== 'prototype' && kind !== 'component-schema' && kind !== 'localization') {
          return;
        }

        try {
          const text = await this.files.readText(path);
          if (kind === 'prototype') {
            const parsed = YAML.parse(text) as unknown;
            const documents = Array.isArray(parsed) ? parsed : [parsed];
            for (const document of documents) {
              if (isRecord(document) && document.type === 'entity' && typeof document.id === 'string') {
                prototypeIds.add(document.id);
              }
            }
          } else if (kind === 'component-schema') {
            const parsed = YAML.parse(text) as unknown;
            if (!isRecord(parsed) || parsed.type !== 'component-schema' || typeof parsed.id !== 'string') {
              return;
            }
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
                fields[fieldName] = fieldSummary;
              }
            }
            schemas.set(parsed.id, { id: parsed.id, fields });
          } else {
            for (const line of text.split(/\r?\n/)) {
              const match = /^\s*([A-Za-z0-9_.-]+)\s*=/.exec(line);
              if (match?.[1]) localizationKeys.add(match[1]);
            }
          }
        } catch {
          // Invalid content is shown when the user opens the file. Metadata loading stays resilient.
        }
      }),
    );

    return {
      prototypes: [...prototypeIds].sort((left, right) => left.localeCompare(right)),
      componentSchemas: [...schemas.values()].sort((left, right) => left.id.localeCompare(right.id)),
      localizationKeys: [...localizationKeys].sort((left, right) => left.localeCompare(right)),
    };
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

import YAML from 'yaml';

import {
  button,
  checkboxInput,
  clear,
  element,
  field,
  numberInput,
  selectInput,
  textInput,
} from '../core/dom';
import type { ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type SchemaField = {
  name: string;
  type: string;
  defaultValue: unknown;
  minimum?: number;
  maximum?: number;
  required: boolean;
};

type SchemaModel = {
  id: string;
  replicationMode: string;
  fields: SchemaField[];
};

const FIELD_TYPES = ['float', 'integer', 'boolean', 'string', 'entity', 'vector2', 'color', 'enum', 'array', 'object'];

export class SchemaEditor extends ModelEditor<SchemaModel> {
  public readonly kind = 'component-schema' as const;
  public readonly title: string;
  private selectedFieldIndex: number | null = null;

  public constructor(source: string, path: string) {
    super(parseSchema(source));
    this.title = path.split('/').at(-1) ?? 'Component schema';
  }

  public static create(name: string): SchemaEditor {
    const id = name.replace(/\.[^.]+$/, '').replace(/[^A-Za-z0-9_]+/g, '') || 'NewComponent';
    return new SchemaEditor(YAML.stringify({
      type: 'component-schema',
      id,
      replication: { mode: 'server-to-client' },
      fields: {
        value: { type: 'float', default: 0 },
      },
    }), `${name}.yml`);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'schema-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Component Schema Editor' }),
      element('span', { className: 'toolbar-spacer' }),
      button('+ Field', () => this.addField(), 'tool-button'),
    );
    const body = element('div', { className: 'schema-editor-body' });
    const table = element('div', { className: 'schema-fields-table' });
    const header = element('div', { className: 'schema-field-row header' });
    for (const label of ['Field', 'Type', 'Default', 'Required']) header.append(element('strong', { text: label }));
    table.append(header);
    for (const [index, schemaField] of this.model.fields.entries()) {
      const row = element('button', { className: `schema-field-row ${index === this.selectedFieldIndex ? 'selected' : ''}` });
      row.type = 'button';
      row.append(
        element('span', { text: schemaField.name }),
        element('span', { text: schemaField.type }),
        element('code', { text: formatValue(schemaField.defaultValue) }),
        element('span', { text: schemaField.required ? 'Yes' : 'No' }),
      );
      row.addEventListener('click', () => {
        this.selectedFieldIndex = index;
        this.render();
      });
      table.append(row);
    }
    if (this.model.fields.length === 0) table.append(element('p', { className: 'empty-state', text: 'No fields. Add a field to generate TypeScript types, serializers and inspector controls.' }));
    const generated = element('section', { className: 'schema-generated-preview' });
    generated.append(
      element('h3', { text: 'Generated TypeScript & descriptors' }),
      element('pre', { text: generateSchemaPreview(this.model) }),
    );
    body.append(table, generated);
    shell.append(toolbar, body);
    this.container.append(shell);
    this.renderInspector();
  }

  protected serializeModel(model: SchemaModel): string {
    const fields: Record<string, unknown> = {};
    for (const schemaField of model.fields) {
      const value: Record<string, unknown> = {
        type: schemaField.type,
        default: schemaField.defaultValue,
      };
      if (schemaField.minimum !== undefined) value.minimum = schemaField.minimum;
      if (schemaField.maximum !== undefined) value.maximum = schemaField.maximum;
      if (schemaField.required) value.required = true;
      fields[schemaField.name] = value;
    }
    return YAML.stringify({
      type: 'component-schema',
      id: model.id,
      replication: { mode: model.replicationMode },
      fields,
    }, { lineWidth: 0, indent: 2 });
  }

  protected parseSource(source: string): SchemaModel {
    return parseSchema(source);
  }

  protected validateModel(model: SchemaModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(model.id)) messages.push({ severity: 'error', message: 'Component ID must be a valid TypeScript identifier.' });
    const names = new Set<string>();
    for (const schemaField of model.fields) {
      if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(schemaField.name)) messages.push({ severity: 'error', message: `Invalid field name: ${schemaField.name}` });
      if (names.has(schemaField.name)) messages.push({ severity: 'error', message: `Duplicate field: ${schemaField.name}` });
      names.add(schemaField.name);
      if (schemaField.minimum !== undefined && schemaField.maximum !== undefined && schemaField.minimum > schemaField.maximum) {
        messages.push({ severity: 'error', message: `${schemaField.name}: minimum exceeds maximum.` });
      }
    }
    if (messages.length === 0) messages.push({ severity: 'info', message: `Schema is valid: ${model.fields.length} fields.` });
    return messages;
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    if (this.selectedFieldIndex === null) {
      this.inspector.append(element('h2', { text: 'Schema Inspector' }));
      this.inspector.append(
        field('Component ID', textInput(this.model.id, (value) => this.commit((model) => { model.id = value; }))),
        field('Replication', selectInput(this.model.replicationMode, ['none', 'server-to-client', 'owner-only'], (value) => this.commit((model) => { model.replicationMode = value; }))),
      );
      this.inspector.append(element('p', { className: 'inspector-note', text: 'This schema generates the TypeScript component type, YAML validation, replication descriptor, save serializer and Prototype Studio fields.' }));
      return;
    }
    const schemaField = this.model.fields[this.selectedFieldIndex];
    if (!schemaField) {
      this.selectedFieldIndex = null;
      this.renderInspector();
      return;
    }
    this.inspector.append(element('h2', { text: 'Field Inspector' }));
    this.inspector.append(
      field('Name', textInput(schemaField.name, (value) => this.updateField((target) => { target.name = value; }))),
      field('Type', selectInput(schemaField.type, FIELD_TYPES, (value) => this.updateField((target) => { target.type = value; target.defaultValue = defaultForType(value); }))),
      field('Default', textInput(formatValue(schemaField.defaultValue), (value) => this.updateField((target) => { target.defaultValue = parseValue(value); }))),
      field('Minimum', numberInput(schemaField.minimum ?? 0, (value) => this.updateField((target) => { target.minimum = value; }), { step: 0.1 })),
      field('Maximum', numberInput(schemaField.maximum ?? 100, (value) => this.updateField((target) => { target.maximum = value; }), { step: 0.1 })),
      field('Required', checkboxInput(schemaField.required, (value) => this.updateField((target) => { target.required = value; }))),
      button('Clear numeric limits', () => this.updateField((target) => { delete target.minimum; delete target.maximum; }), 'secondary-button'),
      button('Delete field', () => this.deleteField(), 'danger-button'),
    );
  }

  private addField(): void {
    let index = 1;
    let name = 'newField';
    while (this.model.fields.some((schemaField) => schemaField.name === name)) name = `newField${index++}`;
    this.commit((model) => model.fields.push({ name, type: 'float', defaultValue: 0, required: false }));
    this.selectedFieldIndex = this.model.fields.length - 1;
    this.render();
  }

  private deleteField(): void {
    const index = this.selectedFieldIndex;
    if (index === null) return;
    this.commit((model) => model.fields.splice(index, 1));
    this.selectedFieldIndex = null;
    this.render();
  }

  private updateField(mutator: (field: SchemaField) => void): void {
    this.commit((model) => {
      const target = model.fields[this.selectedFieldIndex ?? -1];
      if (target) mutator(target);
    });
  }
}


function generateSchemaPreview(model: SchemaModel): string {
  const lines = [`export type ${model.id}Component = {`];
  for (const field of model.fields) {
    lines.push(`  ${field.name}${field.required ? '' : '?'}: ${typescriptType(field.type)};`);
  }
  lines.push('};', '', `export const ${model.id}Descriptor = {`, `  id: '${model.id}',`, `  replication: '${model.replicationMode}',`, '  fields: {');
  for (const field of model.fields) {
    lines.push(`    ${field.name}: { type: '${field.type}', default: ${JSON.stringify(field.defaultValue)} },`);
  }
  lines.push('  },', '} as const;');
  return lines.join('\n');
}

function typescriptType(type: string): string {
  switch (type) {
    case 'float':
    case 'integer': return 'number';
    case 'boolean': return 'boolean';
    case 'entity': return 'EntityId';
    case 'vector2': return 'readonly [number, number]';
    case 'color': return 'string';
    case 'array': return 'readonly unknown[]';
    case 'object': return 'Readonly<Record<string, unknown>>';
    default: return 'string';
  }
}

function parseSchema(source: string): SchemaModel {
  const parsed = YAML.parse(source) as unknown;
  if (!isRecord(parsed) || parsed.type !== 'component-schema' || typeof parsed.id !== 'string') {
    throw new Error('Expected a component-schema YAML document.');
  }
  const replication = isRecord(parsed.replication) && typeof parsed.replication.mode === 'string' ? parsed.replication.mode : 'none';
  const fields: SchemaField[] = [];
  if (isRecord(parsed.fields)) {
    for (const [name, raw] of Object.entries(parsed.fields)) {
      if (!isRecord(raw) || typeof raw.type !== 'string') continue;
      const schemaField: SchemaField = {
        name,
        type: raw.type,
        defaultValue: raw.default,
        required: raw.required === true,
      };
      if (typeof raw.minimum === 'number') schemaField.minimum = raw.minimum;
      if (typeof raw.maximum === 'number') schemaField.maximum = raw.maximum;
      fields.push(schemaField);
    }
  }
  return { id: parsed.id, replicationMode: replication, fields };
}

function defaultForType(type: string): unknown {
  switch (type) {
    case 'float':
    case 'integer': return 0;
    case 'boolean': return false;
    case 'array': return [];
    case 'object': return {};
    case 'vector2': return [0, 0];
    case 'color': return '#ffffff';
    default: return '';
  }
}

function formatValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (value === undefined) return '';
  return JSON.stringify(value);
}

function parseValue(value: string): unknown {
  const trimmed = value.trim();
  if (trimmed === '') return '';
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const numeric = Number(trimmed);
  if (Number.isFinite(numeric)) return numeric;
  if (trimmed.startsWith('[') || trimmed.startsWith('{')) {
    try { return JSON.parse(trimmed); } catch { return trimmed; }
  }
  return trimmed;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

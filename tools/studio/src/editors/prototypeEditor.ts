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
import type { ComponentSchemaSummary, ProjectMetadata, ValidationMessage } from '../core/types';
import { StudioProject } from '../core/project';
import { ModelEditor } from './baseEditor';

type ComponentModel = {
  type: string;
  fields: Record<string, unknown>;
};

type PrototypeModel = {
  type: 'entity';
  id: string;
  parent?: string;
  abstract: boolean;
  name?: string;
  description?: string;
  categories: string[];
  tags: string[];
  components: ComponentModel[];
};

type PrototypeDocument = {
  prototypes: PrototypeModel[];
};

const NATIVE_COMPONENTS = [
  'Transform',
  'NetworkIdentity',
  'PhysicsBody',
  'Collider',
  'Sprite',
  'Player',
  'Inventory',
  'Item',
  'Door',
  'MapGrid',
  'SpatialIndex',
];

const COMPONENT_PRESETS: Record<string, Record<string, unknown>> = {
  PhysicsBody: { bodyType: 'Kinematic' },
  Collider: { shapes: [{ type: 'Circle', radius: 0.32 }] },
  Sprite: {
    drawDepth: 'Objects',
    layers: [{ map: 'base', sprite: '/Resources/Textures/error.rsi', state: 'error' }],
  },
  Inventory: { capacity: 24 },
  Item: { size: 'Small' },
  Door: { open: false },
};

export class PrototypeEditor extends ModelEditor<PrototypeDocument> {
  public readonly kind = 'prototype' as const;
  public readonly title: string;

  private readonly metadata: ProjectMetadata;
  private selectedCollisionShape = 0;
  private selectedPrototypeIndex = 0;
  private selectedComponentIndex: number | null = null;

  public constructor(source: string, path: string, private readonly project: StudioProject) {
    super(parsePrototypes(source));
    this.title = path.split('/').at(-1) ?? 'Prototypes';
    this.metadata = project.projectMetadata;
  }

  public static create(name: string, project: StudioProject): PrototypeEditor {
    const id = sanitizeId(name, 'NewEntity');
    const document: PrototypeDocument = {
      prototypes: [{
        type: 'entity',
        id,
        parent: 'BaseEntity',
        abstract: false,
        name: `${toKebabCase(id)}-name`,
        description: `${toKebabCase(id)}-description`,
        categories: [],
        tags: [],
        components: [
          { type: 'Transform', fields: {} },
          { type: 'Sprite', fields: structuredClone(COMPONENT_PRESETS.Sprite ?? {}) },
        ],
      }],
    };
    return new PrototypeEditor(YAML.stringify(toPrototypeYaml(document), { lineWidth: 0 }), `${name}.yml`, project);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'prototype-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Prototype Studio' }),
      element('span', { className: 'toolbar-spacer' }),
      button('New prototype', () => this.addPrototype(), 'tool-button'),
      button('Duplicate', () => this.duplicatePrototype(), 'tool-button'),
      button('Delete', () => this.deletePrototype(), 'danger-button'),
    );

    const body = element('div', { className: 'prototype-editor-body' });
    const list = this.renderPrototypeList();
    const components = this.renderComponents();
    const preview = this.renderPrototypePreview();
    body.append(list, components, preview);
    shell.append(toolbar, body);
    this.container.append(shell);
    this.renderInspector();
  }

  protected serializeModel(model: PrototypeDocument): string {
    return YAML.stringify(toPrototypeYaml(model), { lineWidth: 0, indent: 2 });
  }

  protected parseSource(source: string): PrototypeDocument {
    return parsePrototypes(source);
  }

  protected validateModel(model: PrototypeDocument): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    const ids = new Set<string>();
    const known = new Set([...this.metadata.prototypes, ...model.prototypes.map((prototype) => prototype.id)]);
    for (const prototype of model.prototypes) {
      if (!prototype.id.trim()) messages.push({ severity: 'error', message: 'Prototype ID is required.' });
      if (ids.has(prototype.id)) messages.push({ severity: 'error', message: `Duplicate prototype ID: ${prototype.id}` });
      ids.add(prototype.id);
      if (prototype.parent && !known.has(prototype.parent)) {
        messages.push({ severity: 'warning', message: `${prototype.id} references unknown parent ${prototype.parent}.` });
      }
      const componentTypes = new Set<string>();
      for (const component of prototype.components) {
        if (componentTypes.has(component.type)) messages.push({ severity: 'warning', message: `${prototype.id} contains duplicate component ${component.type}.` });
        componentTypes.add(component.type);
      }
    }
    if (messages.length === 0) messages.push({ severity: 'info', message: `Prototype file is valid: ${model.prototypes.length} prototypes.` });
    return messages;
  }

  private get selectedPrototype(): PrototypeModel {
    return this.model.prototypes[this.selectedPrototypeIndex] ?? this.model.prototypes[0] ?? createEmptyPrototype('NewEntity');
  }

  private renderPrototypeList(): HTMLElement {
    const panel = element('aside', { className: 'prototype-list-panel' });
    panel.append(element('h3', { text: 'Prototypes' }));
    const search = element('input', { attrs: { placeholder: 'Search prototypes…' } });
    const list = element('div', { className: 'prototype-list' });
    const render = (): void => {
      clear(list);
      const query = search.value.trim().toLowerCase();
      for (const [index, prototype] of this.model.prototypes.entries()) {
        if (query && !prototype.id.toLowerCase().includes(query) && !prototype.name?.toLowerCase().includes(query)) continue;
        const item = button('', () => {
          this.selectedPrototypeIndex = index;
          this.selectedComponentIndex = null;
          this.render();
        }, `prototype-list-item ${index === this.selectedPrototypeIndex ? 'selected' : ''}`);
        item.append(
          element('strong', { text: prototype.id }),
          element('small', { text: prototype.abstract ? 'Abstract' : prototype.parent ?? 'No parent' }),
        );
        list.append(item);
      }
    };
    search.addEventListener('input', render);
    render();
    panel.append(search, list);
    return panel;
  }

  private renderComponents(): HTMLElement {
    const panel = element('section', { className: 'component-list-panel' });
    const heading = element('div', { className: 'panel-heading' });
    heading.append(element('h3', { text: 'Components' }), button('+ Add', () => this.openAddComponentMenu(), 'mini-button'));
    panel.append(heading);
    const list = element('div', { className: 'component-list' });
    for (const [index, component] of this.selectedPrototype.components.entries()) {
      const item = button('', () => {
        this.selectedComponentIndex = index;
        this.render();
      }, `component-card ${index === this.selectedComponentIndex ? 'selected' : ''}`);
      item.append(
        element('span', { className: 'component-icon', text: component.type.slice(0, 2).toUpperCase() }),
        element('span', { className: 'component-name', text: component.type }),
      );
      list.append(item);
    }
    if (this.selectedPrototype.components.length === 0) {
      list.append(element('p', { className: 'empty-state', text: 'No components. Add one from the schema library.' }));
    }
    panel.append(list);
    return panel;
  }

  private renderPrototypePreview(): HTMLElement {
    const prototype = this.selectedPrototype;
    const panel = element('section', { className: 'prototype-preview-panel' });
    panel.append(element('h3', { text: 'Entity Preview' }));
    const card = element('div', { className: 'entity-preview-card' });
    const sprite = element('div', { className: 'entity-preview-sprite' });
    const spriteComponent = prototype.components.find((component) => component.type === 'Sprite');
    const layer = Array.isArray(spriteComponent?.fields.layers) ? spriteComponent.fields.layers.find(isRecord) : undefined;
    const resource = isRecord(layer) ? String(layer.sprite ?? layer.texture ?? '') : '';
    const state = isRecord(layer) ? String(layer.state ?? '') : '';
    sprite.append(element('span', { className: 'entity-preview-loading', text: resource ? 'Loading sprite…' : 'No sprite' }));
    if (resource) void this.renderActualSprite(sprite, resource, state);

    const info = element('div', { className: 'entity-preview-info' });
    info.append(
      element('h2', { text: prototype.id }),
      element('p', { text: prototype.name ?? 'No localization name' }),
      element('p', { text: prototype.description ?? 'No description' }),
      element('div', { className: 'chip-row' }),
    );
    const chips = info.querySelector('.chip-row');
    for (const component of prototype.components) chips?.append(element('span', { className: 'chip', text: component.type }));
    card.append(sprite, info);
    panel.append(card);
    return panel;
  }

  private async renderActualSprite(host: HTMLElement, resource: string, stateName: string): Promise<void> {
    try {
      clear(host);
      if (resource.toLowerCase().endsWith('.rsi')) {
        const projectPath = this.project.resolveProjectPath(resource);
        const rsi = await this.project.readRsi(projectPath);
        const state = rsi.states.find((candidate) => candidate.name === stateName) ?? rsi.states[0];
        if (!state) throw new Error('RSI has no states.');
        const url = await this.project.getObjectUrl(state.imagePath);
        const image = new Image();
        image.src = url;
        await image.decode();
        const width = rsi.meta.size?.x ?? 32;
        const height = rsi.meta.size?.y ?? 32;
        const preview = element('div', { className: 'entity-preview-rsi' });
        preview.style.width = `${width}px`;
        preview.style.height = `${height}px`;
        preview.style.backgroundImage = `url(${url})`;
        preview.style.backgroundPosition = '0 0';
        preview.style.transform = 'scale(4)';
        preview.style.transformOrigin = 'center';
        host.append(preview, element('small', { text: `${state.name} · ${width}×${height}` }));
      } else {
        const url = await this.project.getObjectUrl(resource);
        const image = element('img', { className: 'entity-preview-image' });
        image.src = url;
        image.alt = this.selectedPrototype.id;
        host.append(image, element('small', { text: resource.split('/').at(-1) ?? resource }));
      }
    } catch (error) {
      clear(host);
      host.append(
        element('span', { className: 'entity-preview-glyph', text: this.selectedPrototype.id.slice(0, 2).toUpperCase() }),
        element('small', { text: error instanceof Error ? error.message : String(error) }),
      );
    }
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    const prototype = this.selectedPrototype;

    if (this.selectedComponentIndex === null) {
      this.inspector.append(element('h2', { text: 'Prototype Inspector' }));
      this.inspector.append(
        field('ID', textInput(prototype.id, (value) => this.updatePrototype((target) => { target.id = value; }))),
        field('Parent', selectInput(prototype.parent ?? '', ['', ...this.metadata.prototypes.filter((id) => id !== prototype.id)], (value) => this.updatePrototype((target) => {
          if (value) target.parent = value;
          else delete target.parent;
        }))),
        field('Localization name', textInput(prototype.name ?? '', (value) => this.updatePrototype((target) => {
          if (value) target.name = value;
          else delete target.name;
        }))),
        field('Localization description', textInput(prototype.description ?? '', (value) => this.updatePrototype((target) => {
          if (value) target.description = value;
          else delete target.description;
        }))),
        field('Abstract', checkboxInput(prototype.abstract, (value) => this.updatePrototype((target) => { target.abstract = value; }))),
        field('Categories', textInput(prototype.categories.join(', '), (value) => this.updatePrototype((target) => { target.categories = splitCsv(value); }))),
        field('Tags', textInput(prototype.tags.join(', '), (value) => this.updatePrototype((target) => { target.tags = splitCsv(value); }))),
      );
      this.inspector.append(element('p', { className: 'inspector-note', text: 'Select a component to edit its properties. Component fields are generated from schemas when available.' }));
      return;
    }

    const component = prototype.components[this.selectedComponentIndex];
    if (!component) {
      this.selectedComponentIndex = null;
      this.renderInspector();
      return;
    }
    this.inspector.append(element('h2', { text: `${component.type} Inspector` }));
    const schema = this.metadata.componentSchemas.find((candidate) => candidate.id === component.type);
    this.renderComponentFields(component, schema);
    this.inspector.append(button('Remove component', () => {
      const index = this.selectedComponentIndex;
      if (index === null) return;
      this.commit((model) => model.prototypes[this.selectedPrototypeIndex]?.components.splice(index, 1));
      this.selectedComponentIndex = null;
      this.render();
    }, 'danger-button'));
  }

  private renderComponentFields(component: ComponentModel, schema: ComponentSchemaSummary | undefined): void {
    if (!this.inspector) return;
    const commonFields = componentFieldDefinitions(component.type, schema);
    const renderedKeys = new Set<string>();

    for (const definition of commonFields) {
      renderedKeys.add(definition.key);
      const current = component.fields[definition.key] ?? definition.defaultValue;
      let control: HTMLElement;
      if (definition.type === 'boolean') {
        control = checkboxInput(Boolean(current), (value) => this.updateComponentField(definition.key, value));
      } else if (definition.type === 'number') {
        const numberOptions: { min?: number; max?: number; step?: number } = {
          step: definition.step ?? 0.1,
        };
        if (definition.minimum !== undefined) numberOptions.min = definition.minimum;
        if (definition.maximum !== undefined) numberOptions.max = definition.maximum;
        control = numberInput(Number(current ?? 0), (value) => this.updateComponentField(definition.key, value), numberOptions);
      } else if (definition.options) {
        control = selectInput(String(current ?? ''), definition.options, (value) => this.updateComponentField(definition.key, value));
      } else {
        control = textInput(formatFieldValue(current), (value) => this.updateComponentField(definition.key, parseFieldValue(value)));
      }
      this.inspector.append(field(definition.label, control));
    }

    if (component.type === 'Sprite') {
      this.renderSpriteLayers(component);
    }
    if (component.type === 'Collider') {
      this.renderCollisionEditor(component);
    }

    const advanced = element('details', { className: 'inspector-section advanced-properties' });
    const summary = element('summary', { text: 'Advanced properties' });
    advanced.append(summary);
    for (const [key, value] of Object.entries(component.fields)) {
      if (renderedKeys.has(key) || (component.type === 'Sprite' && key === 'layers')) continue;
      const row = element('div', { className: 'key-value-row' });
      const keyInput = textInput(key, (nextKey) => {
        if (!nextKey || nextKey === key) return;
        this.commit((model) => {
          const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
          if (!target) return;
          target.fields[nextKey] = target.fields[key];
          delete target.fields[key];
        });
      });
      const valueInput = textInput(formatFieldValue(value), (next) => this.updateComponentField(key, parseFieldValue(next)));
      row.append(keyInput, valueInput, button('×', () => {
        this.commit((model) => {
          const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
          if (target) delete target.fields[key];
        });
      }, 'icon-button'));
      advanced.append(row);
    }
    advanced.append(button('+ Property', () => {
      const key = window.prompt('Property name');
      if (key) this.updateComponentField(key, '');
    }, 'secondary-button'));
    this.inspector.append(advanced);
  }

  private renderSpriteLayers(component: ComponentModel): void {
    if (!this.inspector) return;
    const section = element('section', { className: 'inspector-section' });
    const heading = element('div', { className: 'panel-heading' });
    heading.append(element('h3', { text: 'Sprite layers' }), button('+ Layer', () => {
      this.commit((model) => {
        const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
        if (!target) return;
        const layers = Array.isArray(target.fields.layers) ? target.fields.layers : [];
        layers.push({ map: `layer-${layers.length + 1}`, sprite: '/Resources/Textures/error.rsi', state: 'error' });
        target.fields.layers = layers;
      });
    }, 'mini-button'));
    section.append(heading);
    const layers = Array.isArray(component.fields.layers) ? component.fields.layers.filter(isRecord) : [];
    for (const [index, layer] of layers.entries()) {
      const card = element('div', { className: 'sprite-layer-card' });
      card.append(
        field('Map key', textInput(String(layer.map ?? ''), (value) => this.updateSpriteLayer(index, 'map', value))),
        field('RSI / texture', textInput(String(layer.sprite ?? layer.texture ?? ''), (value) => this.updateSpriteLayer(index, value.endsWith('.png') || value.endsWith('.webp') ? 'texture' : 'sprite', value))),
        field('State', textInput(String(layer.state ?? ''), (value) => this.updateSpriteLayer(index, 'state', value))),
        button('Remove layer', () => {
          this.commit((model) => {
            const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
            const targetLayers = Array.isArray(target?.fields.layers) ? target.fields.layers : [];
            targetLayers.splice(index, 1);
          });
        }, 'danger-link'),
      );
      section.append(card);
    }
    this.inspector.append(section);
  }

  private renderCollisionEditor(component: ComponentModel): void {
    if (!this.inspector) return;
    const section = element('section', { className: 'inspector-section collision-editor-section' });
    const heading = element('div', { className: 'panel-heading' });
    heading.append(
      element('h3', { text: 'Collision shapes' }),
      button('+ Circle', () => this.addCollisionShape({ type: 'Circle', radius: 0.32, offset: [0, 0] }), 'mini-button'),
      button('+ Box', () => this.addCollisionShape({ type: 'Box', width: 0.64, height: 0.64, offset: [0, 0] }), 'mini-button'),
    );
    section.append(heading);
    const canvas = element('canvas', { className: 'collision-preview' });
    canvas.width = 280;
    canvas.height = 280;
    section.append(canvas);
    const shapes = Array.isArray(component.fields.shapes) ? component.fields.shapes.filter(isRecord) : [];
    requestAnimationFrame(() => drawCollisionPreview(canvas, shapes, this.selectedCollisionShape));

    shapes.forEach((shape, index) => {
      const card = element('div', { className: `collision-shape-card ${index === this.selectedCollisionShape ? 'selected' : ''}` });
      const type = String(shape.type ?? 'Circle');
      card.addEventListener('click', () => { this.selectedCollisionShape = index; this.render(); });
      card.append(element('strong', { text: `${index + 1}. ${type}` }));
      if (type === 'Circle') {
        card.append(field('Radius', numberInput(Number(shape.radius ?? 0.32), (value) => this.updateCollisionShape(index, 'radius', Math.max(0.01, value)), { min: 0.01, max: 32, step: 0.01 })));
      } else {
        card.append(
          field('Width', numberInput(Number(shape.width ?? 0.64), (value) => this.updateCollisionShape(index, 'width', Math.max(0.01, value)), { min: 0.01, max: 32, step: 0.01 })),
          field('Height', numberInput(Number(shape.height ?? 0.64), (value) => this.updateCollisionShape(index, 'height', Math.max(0.01, value)), { min: 0.01, max: 32, step: 0.01 })),
        );
      }
      const offset = Array.isArray(shape.offset) ? shape.offset : [0, 0];
      card.append(
        field('Offset X', numberInput(Number(offset[0] ?? 0), (value) => this.updateCollisionOffset(index, 0, value), { min: -32, max: 32, step: 0.01 })),
        field('Offset Y', numberInput(Number(offset[1] ?? 0), (value) => this.updateCollisionOffset(index, 1, value), { min: -32, max: 32, step: 0.01 })),
        button('Remove', () => this.removeCollisionShape(index), 'danger-link'),
      );
      section.append(card);
    });
    if (shapes.length === 0) section.append(element('p', { className: 'empty-state', text: 'No collision shapes.' }));
    this.inspector.append(section);
  }

  private addCollisionShape(shape: Record<string, unknown>): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
      if (!target) return;
      const shapes = Array.isArray(target.fields.shapes) ? target.fields.shapes : [];
      shapes.push(shape);
      target.fields.shapes = shapes;
    });
    const component = this.selectedPrototype.components[this.selectedComponentIndex ?? -1];
    const shapes = Array.isArray(component?.fields.shapes) ? component.fields.shapes : [];
    this.selectedCollisionShape = Math.max(0, shapes.length - 1);
  }

  private updateCollisionShape(index: number, key: string, value: unknown): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
      const shapes = Array.isArray(target?.fields.shapes) ? target.fields.shapes : [];
      const shape = shapes[index];
      if (isRecord(shape)) shape[key] = value;
    });
  }

  private updateCollisionOffset(index: number, axis: 0 | 1, value: number): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
      const shapes = Array.isArray(target?.fields.shapes) ? target.fields.shapes : [];
      const shape = shapes[index];
      if (!isRecord(shape)) return;
      const offset = Array.isArray(shape.offset) ? [...shape.offset] : [0, 0];
      offset[axis] = value;
      shape.offset = offset;
    });
  }

  private removeCollisionShape(index: number): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
      const shapes = Array.isArray(target?.fields.shapes) ? target.fields.shapes : [];
      shapes.splice(index, 1);
    });
    this.selectedCollisionShape = Math.max(0, this.selectedCollisionShape - 1);
  }

  private openAddComponentMenu(): void {
    const available = [...new Set([...NATIVE_COMPONENTS, ...this.metadata.componentSchemas.map((schema) => schema.id)])]
      .filter((type) => !this.selectedPrototype.components.some((component) => component.type === type))
      .sort();
    const type = window.prompt(`Component type:\n${available.join(', ')}`, available[0] ?? 'NewComponent');
    if (!type) return;
    const schema = this.metadata.componentSchemas.find((candidate) => candidate.id === type);
    const fields: Record<string, unknown> = structuredClone(COMPONENT_PRESETS[type] ?? {});
    if (schema) {
      for (const [key, definition] of Object.entries(schema.fields)) {
        if (definition.defaultValue !== undefined) fields[key] = structuredClone(definition.defaultValue);
      }
    }
    this.commit((model) => model.prototypes[this.selectedPrototypeIndex]?.components.push({ type, fields }));
    this.selectedComponentIndex = this.selectedPrototype.components.length - 1;
    this.render();
  }

  private addPrototype(): void {
    const id = window.prompt('Prototype ID', 'NewEntity');
    if (!id) return;
    this.commit((model) => model.prototypes.push(createEmptyPrototype(sanitizeId(id, 'NewEntity'))));
    this.selectedPrototypeIndex = this.model.prototypes.length - 1;
    this.selectedComponentIndex = null;
    this.render();
  }

  private duplicatePrototype(): void {
    const source = this.selectedPrototype;
    const id = window.prompt('New prototype ID', `${source.id}Copy`);
    if (!id) return;
    const clone = structuredClone(source);
    clone.id = sanitizeId(id, `${source.id}Copy`);
    this.commit((model) => model.prototypes.push(clone));
    this.selectedPrototypeIndex = this.model.prototypes.length - 1;
    this.selectedComponentIndex = null;
    this.render();
  }

  private deletePrototype(): void {
    if (this.model.prototypes.length <= 1) return;
    const prototype = this.selectedPrototype;
    if (!window.confirm(`Delete prototype ${prototype.id}?`)) return;
    this.commit((model) => model.prototypes.splice(this.selectedPrototypeIndex, 1));
    this.selectedPrototypeIndex = Math.max(0, this.selectedPrototypeIndex - 1);
    this.selectedComponentIndex = null;
    this.render();
  }

  private updatePrototype(mutator: (prototype: PrototypeModel) => void): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex];
      if (target) mutator(target);
    });
  }

  private updateComponentField(key: string, value: unknown): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
      if (!target) return;
      if (value === undefined) delete target.fields[key];
      else target.fields[key] = value;
    });
  }

  private updateSpriteLayer(index: number, key: string, value: unknown): void {
    this.commit((model) => {
      const target = model.prototypes[this.selectedPrototypeIndex]?.components[this.selectedComponentIndex ?? -1];
      if (!target) return;
      const layers = Array.isArray(target.fields.layers) ? target.fields.layers : [];
      const layer = layers[index];
      if (!isRecord(layer)) return;
      if (key === 'sprite') delete layer.texture;
      if (key === 'texture') delete layer.sprite;
      layer[key] = value;
    });
  }
}

type FieldDefinition = {
  key: string;
  label: string;
  type: 'string' | 'number' | 'boolean';
  defaultValue?: unknown;
  minimum?: number;
  maximum?: number;
  step?: number;
  options?: string[];
};

function componentFieldDefinitions(type: string, schema: ComponentSchemaSummary | undefined): FieldDefinition[] {
  const native: Record<string, FieldDefinition[]> = {
    PhysicsBody: [{ key: 'bodyType', label: 'Body type', type: 'string', defaultValue: 'Kinematic', options: ['Static', 'Kinematic', 'Dynamic'] }],
    Inventory: [{ key: 'capacity', label: 'Capacity', type: 'number', defaultValue: 24, minimum: 0, maximum: 1000, step: 1 }],
    Item: [{ key: 'size', label: 'Size', type: 'string', defaultValue: 'Small', options: ['Tiny', 'Small', 'Normal', 'Large', 'Huge'] }],
    Door: [{ key: 'open', label: 'Initially open', type: 'boolean', defaultValue: false }],
    Sprite: [{ key: 'drawDepth', label: 'Draw depth', type: 'string', defaultValue: 'Objects', options: ['Floor', 'Underfloor', 'Structures', 'Objects', 'Items', 'Mobs', 'Effects', 'UI'] }],
  };
  const definitions = [...(native[type] ?? [])];
  if (schema) {
    for (const [key, fieldSchema] of Object.entries(schema.fields)) {
      if (definitions.some((definition) => definition.key === key)) continue;
      definitions.push({
        key,
        label: humanize(key),
        type: fieldSchema.type === 'bool' || fieldSchema.type === 'boolean' ? 'boolean' : ['float', 'double', 'int', 'integer', 'number'].includes(fieldSchema.type) ? 'number' : 'string',
        defaultValue: fieldSchema.defaultValue,
        ...(fieldSchema.minimum !== undefined ? { minimum: fieldSchema.minimum } : {}),
        ...(fieldSchema.maximum !== undefined ? { maximum: fieldSchema.maximum } : {}),
        ...(fieldSchema.options !== undefined ? { options: fieldSchema.options } : {}),
      });
    }
  }
  return definitions;
}


function drawCollisionPreview(canvas: HTMLCanvasElement, shapes: Record<string, unknown>[], selectedIndex: number): void {
  const context = canvas.getContext('2d');
  if (!context) return;
  const scale = 96;
  const centerX = canvas.width / 2;
  const centerY = canvas.height / 2;
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = '#071015';
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.strokeStyle = 'rgba(130, 210, 230, 0.18)';
  context.lineWidth = 1;
  for (let x = centerX % (scale / 4); x < canvas.width; x += scale / 4) { context.beginPath(); context.moveTo(x, 0); context.lineTo(x, canvas.height); context.stroke(); }
  for (let y = centerY % (scale / 4); y < canvas.height; y += scale / 4) { context.beginPath(); context.moveTo(0, y); context.lineTo(canvas.width, y); context.stroke(); }
  context.strokeStyle = '#63ffd0';
  context.beginPath(); context.moveTo(centerX, 0); context.lineTo(centerX, canvas.height); context.moveTo(0, centerY); context.lineTo(canvas.width, centerY); context.stroke();
  shapes.forEach((shape, index) => {
    const offset = Array.isArray(shape.offset) ? shape.offset : [0, 0];
    const x = centerX + Number(offset[0] ?? 0) * scale;
    const y = centerY + Number(offset[1] ?? 0) * scale;
    context.fillStyle = index === selectedIndex ? 'rgba(99, 255, 208, 0.25)' : 'rgba(101, 215, 255, 0.18)';
    context.strokeStyle = index === selectedIndex ? '#63ffd0' : '#65d7ff';
    context.lineWidth = index === selectedIndex ? 3 : 2;
    context.beginPath();
    if (String(shape.type ?? 'Circle') === 'Circle') {
      context.arc(x, y, Math.max(1, Number(shape.radius ?? 0.32) * scale), 0, Math.PI * 2);
    } else {
      const width = Number(shape.width ?? 0.64) * scale;
      const height = Number(shape.height ?? 0.64) * scale;
      context.rect(x - width / 2, y - height / 2, width, height);
    }
    context.fill();
    context.stroke();
  });
}

function parsePrototypes(source: string): PrototypeDocument {
  const parsed = YAML.parse(source) as unknown;
  const documents = Array.isArray(parsed) ? parsed : [parsed];
  const prototypes: PrototypeModel[] = [];
  for (const raw of documents) {
    if (!isRecord(raw) || raw.type !== 'entity' || typeof raw.id !== 'string') continue;
    const prototype: PrototypeModel = {
      type: 'entity',
      id: raw.id,
      abstract: raw.abstract === true,
      categories: Array.isArray(raw.categories) ? raw.categories.map(String) : [],
      tags: Array.isArray(raw.tags) ? raw.tags.map(String) : [],
      components: [],
    };
    if (typeof raw.parent === 'string') prototype.parent = raw.parent;
    if (typeof raw.name === 'string') prototype.name = raw.name;
    if (typeof raw.description === 'string') prototype.description = raw.description;
    if (Array.isArray(raw.components)) {
      for (const rawComponent of raw.components) {
        if (!isRecord(rawComponent) || typeof rawComponent.type !== 'string') continue;
        const fields: Record<string, unknown> = {};
        for (const [key, value] of Object.entries(rawComponent)) if (key !== 'type') fields[key] = value;
        prototype.components.push({ type: rawComponent.type, fields });
      }
    }
    prototypes.push(prototype);
  }
  if (prototypes.length === 0) throw new Error('No entity prototypes found in the YAML document.');
  return { prototypes };
}

function toPrototypeYaml(model: PrototypeDocument): Record<string, unknown>[] {
  return model.prototypes.map((prototype) => {
    const result: Record<string, unknown> = { type: 'entity', id: prototype.id };
    if (prototype.parent) result.parent = prototype.parent;
    if (prototype.abstract) result.abstract = true;
    if (prototype.name) result.name = prototype.name;
    if (prototype.description) result.description = prototype.description;
    if (prototype.categories.length > 0) result.categories = prototype.categories;
    if (prototype.tags.length > 0) result.tags = prototype.tags;
    result.components = prototype.components.map((component) => ({ type: component.type, ...component.fields }));
    return result;
  });
}

function createEmptyPrototype(id: string): PrototypeModel {
  return {
    type: 'entity',
    id,
    parent: 'BaseEntity',
    abstract: false,
    name: `${toKebabCase(id)}-name`,
    description: `${toKebabCase(id)}-description`,
    categories: [],
    tags: [],
    components: [],
  };
}

function splitCsv(value: string): string[] {
  return value.split(',').map((part) => part.trim()).filter(Boolean);
}

function formatFieldValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (value === undefined) return '';
  return JSON.stringify(value);
}

function parseFieldValue(value: string): unknown {
  const trimmed = value.trim();
  if (!trimmed) return '';
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const number = Number(trimmed);
  if (Number.isFinite(number)) return number;
  if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
    try { return JSON.parse(trimmed); } catch { return trimmed; }
  }
  return trimmed;
}

function humanize(value: string): string {
  return value.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/[-_]/g, ' ').replace(/^./, (character) => character.toUpperCase());
}

function sanitizeId(value: string, fallback: string): string {
  const cleaned = value.replace(/[^A-Za-z0-9_.-]+/g, '');
  return cleaned || fallback;
}

function toKebabCase(value: string): string {
  return value.replace(/([a-z0-9])([A-Z])/g, '$1-$2').replace(/[_.\s]+/g, '-').toLowerCase();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

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
import type { ProjectMetadata, ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type UiNode = {
  _editorId: string;
  type: string;
  id?: string;
  text?: string;
  title?: string;
  source?: string;
  width?: number | string;
  height?: number | string;
  gap?: number;
  grow?: number;
  columns?: number;
  visible?: boolean | string;
  enabled?: boolean | string;
  value?: unknown;
  items?: unknown;
  onClick?: string | Record<string, unknown>;
  children?: UiNode[];
  [key: string]: unknown;
};

type UiModel = {
  root: UiNode;
};

const CONTAINER_TYPES = new Set(['Window', 'Row', 'Column', 'Grid', 'ScrollContainer', 'SplitContainer', 'TabContainer', 'Panel']);
const PALETTE_GROUPS: Record<string, string[]> = {
  Layout: ['Row', 'Column', 'Grid', 'Panel', 'ScrollContainer', 'SplitContainer', 'TabContainer', 'Spacer'],
  Controls: ['Label', 'Button', 'Image', 'TextInput', 'Checkbox', 'Slider', 'List', 'ProgressBar', 'Dropdown'],
  Game: ['EntityView', 'InventoryGrid', 'PaperDoll', 'MapView', 'ChatBox'],
};

const DEFAULTS: Record<string, Partial<UiNode>> = {
  Row: { gap: 8, children: [] },
  Column: { gap: 8, children: [] },
  Grid: { gap: 8, columns: 2, children: [] },
  Panel: { children: [] },
  ScrollContainer: { children: [] },
  SplitContainer: { children: [] },
  TabContainer: { children: [] },
  Spacer: { grow: 1 },
  Label: { text: 'Label' },
  Button: { text: 'Button', onClick: 'action' },
  Image: { source: '/Resources/Textures/error.rsi' },
  TextInput: { value: '' },
  Checkbox: { text: 'Checkbox', value: false },
  Slider: { value: 50 },
  List: { items: '$state.items' },
  ProgressBar: { value: 50 },
  Dropdown: { items: '$state.options' },
  EntityView: { value: '$state.entity' },
  InventoryGrid: { items: '$state.inventory' },
  PaperDoll: { value: '$state.character' },
  MapView: { value: '$state.map' },
  ChatBox: { items: '$state.messages' },
};

export class UiEditor extends ModelEditor<UiModel> {
  public readonly kind = 'hui' as const;
  public readonly title: string;

  private readonly metadata: ProjectMetadata;
  private selectedId: string;
  private previewWidth = 1280;
  private previewHeight = 720;
  private hierarchyFilter = '';

  public constructor(source: string, path: string, metadata: ProjectMetadata) {
    const model = parseUi(source);
    super(model);
    this.title = path.split('/').at(-1) ?? 'UI';
    this.metadata = metadata;
    this.selectedId = model.root._editorId;
  }

  public static create(name: string, metadata: ProjectMetadata): UiEditor {
    const root: UiNode = {
      _editorId: crypto.randomUUID(),
      type: 'Window',
      id: sanitizeId(name),
      title: `${sanitizeId(name)}-title`,
      width: 640,
      height: 420,
      children: [
        {
          _editorId: crypto.randomUUID(),
          type: 'Column',
          gap: 12,
          children: [],
        },
      ],
    };
    return new UiEditor(YAML.stringify(stripEditorIds(root), { lineWidth: 0 }), `${name}.hui.yml`, metadata);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'ui-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Visual UI Designer' }),
      element('span', { className: 'toolbar-spacer' }),
      element('span', { className: 'toolbar-label', text: 'Preview' }),
      selectInput(`${this.previewWidth}×${this.previewHeight}`, ['1920×1080', '1600×900', '1366×768', '1280×720', '1024×768', '800×600'], (value) => {
        const [width, height] = value.split('×').map(Number);
        if (width && height) {
          this.previewWidth = width;
          this.previewHeight = height;
          this.render();
        }
      }),
      button('Add child', () => this.addNode('Column'), 'tool-button'),
      button('Delete', () => this.deleteSelected(), 'danger-button'),
    );

    const body = element('div', { className: 'ui-editor-body' });
    const palette = this.renderPalette();
    const center = element('div', { className: 'ui-center' });
    center.append(this.renderCanvas());
    const hierarchy = this.renderHierarchy();
    body.append(palette, center, hierarchy);
    shell.append(toolbar, body);
    this.container.append(shell);
    this.renderInspector();
  }

  protected serializeModel(model: UiModel): string {
    return YAML.stringify(stripEditorIds(model.root), { lineWidth: 0, indent: 2 });
  }

  protected parseSource(source: string): UiModel {
    return parseUi(source);
  }

  protected validateModel(model: UiModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    const ids = new Set<string>();
    walkUi(model.root, (node) => {
      if (!node.type) messages.push({ severity: 'error', message: 'UI node has no type.' });
      if (node.id) {
        if (ids.has(node.id)) messages.push({ severity: 'error', message: `Duplicate UI id: ${node.id}` });
        ids.add(node.id);
      }
      if (node.children && !CONTAINER_TYPES.has(node.type)) {
        messages.push({ severity: 'warning', message: `${node.type} contains children but is not a container.` });
      }
      for (const property of ['text', 'title'] as const) {
        const value = node[property];
        if (typeof value === 'string' && !value.startsWith('$') && value.includes(' ') && !value.includes('-')) {
          messages.push({ severity: 'warning', message: `${node.type}.${property} looks like raw text; use an FTL key for localization.` });
        }
      }
    });
    if (messages.length === 0) messages.push({ severity: 'info', message: `UI is valid: ${countNodes(model.root)} nodes.` });
    return messages;
  }

  private renderPalette(): HTMLElement {
    const palette = element('aside', { className: 'ui-palette' });
    palette.append(element('h3', { text: 'Components' }));
    const hint = element('p', { className: 'palette-hint', text: 'Drag a component onto the canvas or click to add it.' });
    palette.append(hint);
    for (const [groupName, types] of Object.entries(PALETTE_GROUPS)) {
      const group = element('section', { className: 'palette-group' });
      group.append(element('h4', { text: groupName }));
      for (const type of types) {
        const item = button(type, () => this.addNode(type), 'component-palette-item');
        item.draggable = true;
        item.addEventListener('dragstart', (event) => {
          event.dataTransfer?.setData('application/x-honknet-ui-node', type);
          if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
        });
        group.append(item);
      }
      palette.append(group);
    }
    return palette;
  }

  private renderCanvas(): HTMLElement {
    const viewport = element('div', { className: 'ui-preview-viewport' });
    const scale = Math.min(1, 900 / this.previewWidth, 620 / this.previewHeight);
    const device = element('div', { className: 'ui-preview-device' });
    device.style.width = `${this.previewWidth}px`;
    device.style.height = `${this.previewHeight}px`;
    device.style.transform = `scale(${scale})`;
    device.style.transformOrigin = 'top left';
    viewport.style.setProperty('--preview-width', `${this.previewWidth * scale}px`);
    viewport.style.setProperty('--preview-height', `${this.previewHeight * scale}px`);

    const rootPreview = this.renderPreviewNode(this.model.root);
    device.append(rootPreview);
    viewport.append(device);
    viewport.addEventListener('dragover', (event) => {
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    });
    viewport.addEventListener('drop', (event) => {
      event.preventDefault();
      const type = event.dataTransfer?.getData('application/x-honknet-ui-node');
      if (type) this.addNode(type);
    });
    return viewport;
  }

  private renderPreviewNode(node: UiNode): HTMLElement {
    const wrapper = element('div', { className: `hui-node hui-${node.type.toLowerCase()} ${node._editorId === this.selectedId ? 'selected' : ''}` });
    wrapper.dataset.editorId = node._editorId;
    wrapper.title = `${node.type}${node.id ? ` #${node.id}` : ''}`;
    applyNodeLayout(wrapper, node);
    wrapper.addEventListener('click', (event) => {
      event.stopPropagation();
      this.selectedId = node._editorId;
      this.render();
    });
    wrapper.addEventListener('dragover', (event) => {
      if (CONTAINER_TYPES.has(node.type)) {
        event.preventDefault();
        event.stopPropagation();
      }
    });
    wrapper.addEventListener('drop', (event) => {
      if (!CONTAINER_TYPES.has(node.type)) return;
      event.preventDefault();
      event.stopPropagation();
      const type = event.dataTransfer?.getData('application/x-honknet-ui-node');
      if (type) {
        this.selectedId = node._editorId;
        this.addNode(type);
      }
    });

    switch (node.type) {
      case 'Window': {
        wrapper.classList.add('hui-window');
        const titlebar = element('div', { className: 'hui-window-title', text: displayText(node.title ?? node.id ?? 'Window') });
        const content = element('div', { className: 'hui-window-content' });
        for (const child of node.children ?? []) content.append(this.renderPreviewNode(child));
        wrapper.append(titlebar, content);
        return wrapper;
      }
      case 'Button':
        wrapper.append(element('button', { text: displayText(node.text ?? 'Button') }));
        return wrapper;
      case 'Label':
        wrapper.append(element('span', { text: displayText(node.text ?? 'Label') }));
        return wrapper;
      case 'Image': {
        const image = element('div', { className: 'hui-image-placeholder', text: node.source ? node.source.split('/').at(-1) ?? 'Image' : 'Image' });
        wrapper.append(image);
        return wrapper;
      }
      case 'TextInput': {
        const input = element('input');
        input.placeholder = typeof node.value === 'string' ? displayText(node.value) : 'Text input';
        input.disabled = true;
        wrapper.append(input);
        return wrapper;
      }
      case 'Checkbox': {
        const label = element('label', { className: 'hui-checkbox-control' });
        const input = element('input');
        input.type = 'checkbox';
        input.disabled = true;
        input.checked = node.value === true;
        label.append(input, element('span', { text: displayText(node.text ?? 'Checkbox') }));
        wrapper.append(label);
        return wrapper;
      }
      case 'Slider': {
        const input = element('input');
        input.type = 'range';
        input.disabled = true;
        input.value = String(typeof node.value === 'number' ? node.value : 50);
        wrapper.append(input);
        return wrapper;
      }
      case 'ProgressBar': {
        const bar = element('div', { className: 'hui-progress-track' });
        const fill = element('div', { className: 'hui-progress-fill' });
        fill.style.width = `${Math.max(0, Math.min(100, Number(node.value ?? 50)))}%`;
        bar.append(fill);
        wrapper.append(bar);
        return wrapper;
      }
      case 'List':
      case 'Dropdown':
      case 'InventoryGrid':
      case 'ChatBox':
        wrapper.append(element('div', { className: 'hui-list-placeholder', text: `${node.type}: ${String(node.items ?? '$state.items')}` }));
        return wrapper;
      case 'EntityView':
      case 'PaperDoll':
      case 'MapView':
        wrapper.append(element('div', { className: 'hui-game-placeholder', text: node.type }));
        return wrapper;
      case 'Spacer':
        wrapper.append(element('span', { text: 'Spacer' }));
        return wrapper;
      default:
        for (const child of node.children ?? []) wrapper.append(this.renderPreviewNode(child));
        if ((node.children?.length ?? 0) === 0 && CONTAINER_TYPES.has(node.type)) {
          wrapper.append(element('div', { className: 'hui-drop-hint', text: `Drop controls into ${node.type}` }));
        }
        return wrapper;
    }
  }

  private renderHierarchy(): HTMLElement {
    const panel = element('aside', { className: 'ui-hierarchy' });
    panel.append(element('h3', { text: 'Hierarchy' }));
    const search = element('input', { attrs: { placeholder: 'Filter hierarchy…' } });
    search.value = this.hierarchyFilter;
    search.addEventListener('input', () => {
      this.hierarchyFilter = search.value;
      this.render();
    });
    panel.append(search);
    const tree = element('div', { className: 'hierarchy-tree' });
    this.renderHierarchyNode(this.model.root, tree, 0);
    panel.append(tree);
    return panel;
  }

  private renderHierarchyNode(node: UiNode, host: HTMLElement, depth: number): void {
    const query = this.hierarchyFilter.trim().toLowerCase();
    const matches = !query || node.type.toLowerCase().includes(query) || node.id?.toLowerCase().includes(query);
    const childMatches = (node.children ?? []).some((child) => hierarchyContains(child, query));
    if (!matches && !childMatches) return;

    const row = element('button', { className: `hierarchy-row ${node._editorId === this.selectedId ? 'selected' : ''}` });
    row.type = 'button';
    row.style.paddingLeft = `${10 + depth * 16}px`;
    row.append(
      element('span', { className: 'hierarchy-type', text: node.type }),
      element('span', { className: 'hierarchy-id', text: node.id ? `#${node.id}` : '' }),
    );
    row.draggable = node !== this.model.root;
    row.addEventListener('click', () => {
      this.selectedId = node._editorId;
      this.render();
    });
    row.addEventListener('dragstart', (event) => {
      event.dataTransfer?.setData('application/x-honknet-ui-existing', node._editorId);
    });
    row.addEventListener('dragover', (event) => {
      if (CONTAINER_TYPES.has(node.type)) event.preventDefault();
    });
    row.addEventListener('drop', (event) => {
      if (!CONTAINER_TYPES.has(node.type)) return;
      event.preventDefault();
      const existingId = event.dataTransfer?.getData('application/x-honknet-ui-existing');
      const newType = event.dataTransfer?.getData('application/x-honknet-ui-node');
      if (existingId) this.moveNode(existingId, node._editorId);
      else if (newType) {
        this.selectedId = node._editorId;
        this.addNode(newType);
      }
    });
    host.append(row);
    for (const child of node.children ?? []) this.renderHierarchyNode(child, host, depth + 1);
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    const node = findNode(this.model.root, this.selectedId) ?? this.model.root;
    this.inspector.append(element('h2', { text: `${node.type} Inspector` }));
    this.inspector.append(field('Type', selectInput(node.type, allComponentTypes(), (value) => {
      this.commit((model) => {
        const target = findNode(model.root, this.selectedId);
        if (!target) return;
        target.type = value;
        if (CONTAINER_TYPES.has(value)) target.children ??= [];
        else delete target.children;
      });
    })));
    this.inspector.append(field('ID', textInput(node.id ?? '', (value) => this.updateSelected('id', value || undefined)), 'Optional stable ID for bindings and controller access.'));

    if (node.type === 'Window') {
      this.inspector.append(field('Title / FTL key', textInput(node.title ?? '', (value) => this.updateSelected('title', value || undefined))));
    }
    if (['Label', 'Button', 'Checkbox'].includes(node.type)) {
      this.inspector.append(this.bindingField('Text / FTL key', 'text', node.text));
    }
    if (node.type === 'Image') {
      this.inspector.append(field('Image source', textInput(node.source ?? '', (value) => this.updateSelected('source', value || undefined))));
    }
    if (['List', 'Dropdown', 'InventoryGrid', 'ChatBox'].includes(node.type)) {
      this.inspector.append(this.bindingField('Items', 'items', typeof node.items === 'string' ? node.items : ''));
    }
    if (['ProgressBar', 'Slider', 'Checkbox', 'TextInput', 'EntityView', 'PaperDoll', 'MapView'].includes(node.type)) {
      this.inspector.append(this.bindingField('Value', 'value', typeof node.value === 'string' ? node.value : String(node.value ?? '')));
    }
    if (node.type === 'Button') {
      const onClick = typeof node.onClick === 'string' ? node.onClick : '';
      this.inspector.append(field('On click action', textInput(onClick, (value) => this.updateSelected('onClick', value || undefined)), 'Simple action ID. A TypeScript controller is only needed for complex behavior.'));
      const actionSection = element('section', { className: 'inspector-section' });
      actionSection.append(element('h3', { text: 'Declarative action' }));
      actionSection.append(button('Send UI message…', () => {
        const message = window.prompt('Message ID', 'buy-product');
        if (!message) return;
        this.updateSelected('onClick', { type: 'SendMessage', message });
      }, 'secondary-button'));
      this.inspector.append(actionSection);
    }
    if (CONTAINER_TYPES.has(node.type)) {
      this.inspector.append(field('Gap', numberInput(node.gap ?? 0, (value) => this.updateSelected('gap', value), { min: 0, max: 128 })));
    }
    if (node.type === 'Grid') {
      this.inspector.append(field('Columns', numberInput(node.columns ?? 2, (value) => this.updateSelected('columns', Math.max(1, Math.round(value))), { min: 1, max: 24 })));
    }

    const layout = element('section', { className: 'inspector-section' });
    layout.append(element('h3', { text: 'Layout' }));
    layout.append(
      this.dimensionField('Width', 'width', node.width),
      this.dimensionField('Height', 'height', node.height),
      field('Grow', numberInput(node.grow ?? 0, (value) => this.updateSelected('grow', value), { min: 0, max: 10 })),
      this.bindingField('Visible', 'visible', typeof node.visible === 'string' ? node.visible : String(node.visible ?? true)),
      this.bindingField('Enabled', 'enabled', typeof node.enabled === 'string' ? node.enabled : String(node.enabled ?? true)),
    );
    this.inspector.append(layout);

    if (node !== this.model.root) {
      this.inspector.append(button('Delete selected node', () => this.deleteSelected(), 'danger-button'));
    }
    this.inspector.append(element('p', { className: 'inspector-note', text: `Known localization keys: ${this.metadata.localizationKeys.length}. Bindings use $state.path and are generated by the visual controls.` }));
  }

  private bindingField(label: string, property: string, value: string | undefined): HTMLElement {
    const wrapper = element('div', { className: 'binding-field' });
    const input = textInput(value ?? '', (next) => this.updateSelected(property, parseDesignerValue(next)));
    const bind = button('Bind', () => {
      const path = window.prompt('State path', String(value ?? '$state.'));
      if (!path) return;
      this.updateSelected(property, path.startsWith('$') ? path : `$state.${path}`);
    }, 'mini-button');
    wrapper.append(field(label, input), bind);
    return wrapper;
  }

  private dimensionField(label: string, property: 'width' | 'height', value: number | string | undefined): HTMLElement {
    const wrapper = element('div', { className: 'dimension-field' });
    const input = textInput(value === undefined ? '' : String(value), (next) => {
      const numeric = Number(next);
      this.updateSelected(property, next === '' ? undefined : Number.isFinite(numeric) ? numeric : next);
    });
    const fill = button('Fill', () => this.updateSelected(property, 'fill'), 'mini-button');
    wrapper.append(field(label, input), fill);
    return wrapper;
  }

  private addNode(type: string): void {
    const parent = findNode(this.model.root, this.selectedId) ?? this.model.root;
    const target = CONTAINER_TYPES.has(parent.type) ? parent : findParent(this.model.root, parent._editorId) ?? this.model.root;
    const node = createNode(type);
    this.commit((model) => {
      const modelTarget = findNode(model.root, target._editorId) ?? model.root;
      modelTarget.children ??= [];
      modelTarget.children.push(node);
    });
    this.selectedId = node._editorId;
    this.render();
  }

  private deleteSelected(): void {
    if (this.selectedId === this.model.root._editorId) return;
    const parent = findParent(this.model.root, this.selectedId);
    if (!parent?.children) return;
    const index = parent.children.findIndex((child) => child._editorId === this.selectedId);
    if (index < 0) return;
    this.commit((model) => {
      const modelParent = findNode(model.root, parent._editorId);
      modelParent?.children?.splice(index, 1);
    });
    this.selectedId = parent._editorId;
    this.render();
  }

  private moveNode(nodeId: string, parentId: string): void {
    if (nodeId === this.model.root._editorId || nodeId === parentId) return;
    const node = findNode(this.model.root, nodeId);
    const destination = findNode(this.model.root, parentId);
    if (!node || !destination || !CONTAINER_TYPES.has(destination.type) || hierarchyContainsId(node, parentId)) return;
    const oldParent = findParent(this.model.root, nodeId);
    if (!oldParent?.children) return;
    this.commit((model) => {
      const modelOldParent = findNode(model.root, oldParent._editorId);
      const modelDestination = findNode(model.root, parentId);
      if (!modelOldParent?.children || !modelDestination) return;
      const index = modelOldParent.children.findIndex((child) => child._editorId === nodeId);
      const [moved] = modelOldParent.children.splice(index, 1);
      if (!moved) return;
      modelDestination.children ??= [];
      modelDestination.children.push(moved);
    });
  }

  private updateSelected(property: string, value: unknown): void {
    this.commit((model) => {
      const target = findNode(model.root, this.selectedId);
      if (!target) return;
      if (value === undefined || value === '') delete target[property];
      else target[property] = value;
    });
  }
}

function parseUi(source: string): UiModel {
  const parsed = YAML.parse(source) as unknown;
  if (!isRecord(parsed) || typeof parsed.type !== 'string') throw new Error('Expected a HUI root object with a type.');
  return { root: addEditorIds(parsed) };
}

function addEditorIds(value: Record<string, unknown>): UiNode {
  const node: UiNode = { _editorId: crypto.randomUUID(), type: String(value.type ?? 'Panel') };
  for (const [key, raw] of Object.entries(value)) {
    if (key === 'children' && Array.isArray(raw)) {
      node.children = raw.filter(isRecord).map(addEditorIds);
    } else if (key !== 'type') {
      node[key] = raw;
    }
  }
  if (CONTAINER_TYPES.has(node.type)) node.children ??= [];
  return node;
}

function stripEditorIds(node: UiNode): Record<string, unknown> {
  const result: Record<string, unknown> = { type: node.type };
  for (const [key, value] of Object.entries(node)) {
    if (key === '_editorId' || key === 'type' || value === undefined) continue;
    if (key === 'children' && Array.isArray(value)) result.children = value.map((child) => stripEditorIds(child as UiNode));
    else result[key] = value;
  }
  return result;
}

function createNode(type: string): UiNode {
  return {
    _editorId: crypto.randomUUID(),
    type,
    ...structuredClone(DEFAULTS[type] ?? {}),
  };
}

function findNode(root: UiNode, id: string): UiNode | null {
  if (root._editorId === id) return root;
  for (const child of root.children ?? []) {
    const found = findNode(child, id);
    if (found) return found;
  }
  return null;
}

function findParent(root: UiNode, id: string): UiNode | null {
  for (const child of root.children ?? []) {
    if (child._editorId === id) return root;
    const found = findParent(child, id);
    if (found) return found;
  }
  return null;
}

function hierarchyContains(root: UiNode, query: string): boolean {
  if (!query) return true;
  if (root.type.toLowerCase().includes(query) || root.id?.toLowerCase().includes(query)) return true;
  return (root.children ?? []).some((child) => hierarchyContains(child, query));
}

function hierarchyContainsId(root: UiNode, id: string): boolean {
  if (root._editorId === id) return true;
  return (root.children ?? []).some((child) => hierarchyContainsId(child, id));
}

function walkUi(root: UiNode, visitor: (node: UiNode) => void): void {
  visitor(root);
  for (const child of root.children ?? []) walkUi(child, visitor);
}

function countNodes(root: UiNode): number {
  let count = 0;
  walkUi(root, () => { count += 1; });
  return count;
}

function allComponentTypes(): string[] {
  return ['Window', ...Object.values(PALETTE_GROUPS).flat()];
}

function applyNodeLayout(elementNode: HTMLElement, node: UiNode): void {
  if (node.width !== undefined) elementNode.style.width = dimensionToCss(node.width);
  if (node.height !== undefined) elementNode.style.height = dimensionToCss(node.height);
  if (node.grow) elementNode.style.flexGrow = String(node.grow);
  if (node.gap !== undefined) elementNode.style.gap = `${node.gap}px`;
  if (node.type === 'Grid') elementNode.style.gridTemplateColumns = `repeat(${Math.max(1, node.columns ?? 2)}, minmax(0, 1fr))`;
  if (node.visible === false) elementNode.style.opacity = '0.25';
}

function dimensionToCss(value: number | string): string {
  if (typeof value === 'number') return `${value}px`;
  if (value === 'fill') return '100%';
  return value;
}

function displayText(value: string): string {
  if (value.startsWith('$state.')) return `{${value.slice(7)}}`;
  if (value.startsWith('$')) return `{${value.slice(1)}}`;
  return value;
}

function parseDesignerValue(value: string): unknown {
  const trimmed = value.trim();
  if (trimmed === '') return undefined;
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const numeric = Number(trimmed);
  return Number.isFinite(numeric) && trimmed !== '' ? numeric : trimmed;
}

function sanitizeId(value: string): string {
  return value.trim().replace(/\.[^.]+$/, '').replace(/[^A-Za-z0-9_.-]+/g, '-').replace(/^-+|-+$/g, '') || 'new-window';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

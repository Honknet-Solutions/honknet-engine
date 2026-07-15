import {
  createHuiNode,
  getHuiControlSchema,
  isHuiContainer,
  isHuiFreeformContainer,
  listBindingPaths,
  listHuiControlSchemas,
  renderHui,
  validateHuiDocument,
  type HuiAction,
  type HuiControlSchema,
  type HuiNode,
  type HuiPropertySchema,
} from '@honknet/hui-runtime';
import YAML from 'yaml';

import {
  button,
  checkboxInput,
  clear,
  element,
  field,
  modal,
  numberInput,
  selectInput,
  textInput,
  toast,
} from '../core/dom';
import { StudioProject } from '../core/project';
import type { ProjectMetadata, ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type UiNode = {
  _editorId: string;
  type: string;
  id?: string;
  children?: UiNode[];
  x?: number;
  y?: number;
  width?: number | string;
  height?: number | string;
  anchorLeft?: boolean;
  anchorRight?: boolean;
  anchorTop?: boolean;
  anchorBottom?: boolean;
  [key: string]: unknown;
};

type UiModel = {
  root: UiNode;
};

type DragPreview = {
  nodeId: string;
  startClientX: number;
  startClientY: number;
  startX: number;
  startY: number;
  nextX: number;
  nextY: number;
  scale: number;
  element: HTMLElement;
};

type ResizePreview = {
  nodeId: string;
  startClientX: number;
  startClientY: number;
  startWidth: number;
  startHeight: number;
  nextWidth: number;
  nextHeight: number;
  scale: number;
  element: HTMLElement;
};

const DEFAULT_PREVIEW_STATE: Record<string, unknown> = {
  title: 'Preview state',
  visible: true,
  enabled: true,
  selected: {
    id: 'militech-pistol',
    name: 'Militech Pistol',
    available: true,
  },
  items: [
    { id: 'first', label: 'First item' },
    { id: 'second', label: 'Second item' },
    { id: 'third', label: 'Third item' },
  ],
  options: [
    { value: 'low', label: 'Low' },
    { value: 'medium', label: 'Medium' },
    { value: 'high', label: 'High' },
  ],
  inventory: [
    { id: 'pistol', label: 'Pistol' },
    { id: 'magazine', label: 'Magazine' },
    { id: 'medkit', label: 'Medkit' },
  ],
  messages: ['System online.', 'Welcome to Night City.'],
  entity: { id: 42, prototype: 'ExamplePlayer' },
  character: { name: 'V', slots: 12 },
  map: { id: 'debug-map', width: 32, height: 32 },
  progress: 65,
};

const PREVIEW_RESOLUTIONS = ['1920×1080', '1600×900', '1366×768', '1280×720', '1024×768', '800×600'] as const;
const EDITOR_MIME_NEW = 'application/x-honknet-ui-node';
const EDITOR_MIME_EXISTING = 'application/x-honknet-ui-existing';
const GRID_MIN = 1;
const GRID_MAX = 128;

export class UiEditor extends ModelEditor<UiModel> {
  public readonly kind = 'hui' as const;
  public readonly title: string;

  private readonly metadata: ProjectMetadata;
  private readonly project: StudioProject | null;
  private selectedId: string;
  private previewWidth = 1280;
  private previewHeight = 720;
  private hierarchyFilter = '';
  private previewState: Record<string, unknown> = structuredClone(DEFAULT_PREVIEW_STATE);
  private snapEnabled = true;
  private gridSize = 8;
  private previewScale = 1;
  private dragPreview: DragPreview | null = null;
  private resizePreview: ResizePreview | null = null;
  private copiedNode: UiNode | null = null;

  private readonly handleWindowPointerMove = (event: PointerEvent): void => {
    this.updatePointerOperation(event);
  };

  private readonly handleWindowPointerUp = (): void => {
    this.finishPointerOperation();
  };

  private readonly handleWindowKeyDown = (event: KeyboardEvent): void => {
    this.handleEditorShortcut(event);
  };

  public constructor(source: string, path: string, context: ProjectMetadata | StudioProject) {
    const model = parseUi(source);
    super(model);
    this.title = path.split('/').at(-1) ?? 'UI';
    this.project = context instanceof StudioProject ? context : null;
    this.metadata = context instanceof StudioProject ? context.projectMetadata : context;
    this.selectedId = model.root._editorId;
    window.addEventListener('pointermove', this.handleWindowPointerMove);
    window.addEventListener('pointerup', this.handleWindowPointerUp);
    window.addEventListener('keydown', this.handleWindowKeyDown);
  }

  public override unmount(): void {
    window.removeEventListener('pointermove', this.handleWindowPointerMove);
    window.removeEventListener('pointerup', this.handleWindowPointerUp);
    window.removeEventListener('keydown', this.handleWindowKeyDown);
    super.unmount();
  }

  public static create(name: string, context: ProjectMetadata | StudioProject): UiEditor {
    const root = addEditorIds({
      type: 'Window',
      id: sanitizeId(name),
      title: `${sanitizeId(name)}-title`,
      width: 640,
      height: 420,
      padding: 16,
      children: [
        {
          type: 'Column',
          gap: 12,
          width: 'fill',
          height: 'fill',
          children: [],
        },
      ],
    });
    return new UiEditor(YAML.stringify(stripEditorIds(root), { lineWidth: 0 }), `${name}.hui.yml`, context);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'ui-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar ui-toolbar' });
    const resolution = `${this.previewWidth}×${this.previewHeight}`;

    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Visual UI Designer' }),
      button('Select', () => undefined, 'tool-button active'),
      button('Duplicate', () => this.duplicateSelected(), 'tool-button'),
      button('Copy', () => this.copySelected(), 'tool-button'),
      button('Paste', () => this.pasteNode(), 'tool-button'),
      button('Delete', () => this.deleteSelected(), 'danger-button'),
      element('span', { className: 'toolbar-divider' }),
      element('span', { className: 'toolbar-label', text: 'Preview' }),
      selectInput(resolution, PREVIEW_RESOLUTIONS, (value) => {
        const [width, height] = value.split('×').map(Number);
        if (width && height) {
          this.previewWidth = width;
          this.previewHeight = height;
          this.render();
        }
      }),
      button('Preview state', () => void this.editPreviewState(), 'tool-button'),
      element('span', { className: 'toolbar-spacer' }),
      checkboxToolbar('Snap', this.snapEnabled, (checked) => {
        this.snapEnabled = checked;
      }),
      element('span', { className: 'toolbar-label', text: 'Grid' }),
      numberInput(this.gridSize, (value) => {
        this.gridSize = Math.max(GRID_MIN, Math.min(GRID_MAX, Math.round(value)));
        this.render();
      }, { min: GRID_MIN, max: GRID_MAX, step: 1 }),
    );

    const body = element('div', { className: 'ui-editor-body' });
    const center = element('div', { className: 'ui-center' });
    center.append(this.renderCanvas());
    body.append(this.renderPalette(), center, this.renderHierarchy());
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
    return validateHuiDocument(stripEditorIds(model.root) as HuiNode).map((issue) => ({
      severity: issue.severity,
      message: `${issue.path}: ${issue.message}`,
    }));
  }

  private renderPalette(): HTMLElement {
    const palette = element('aside', { className: 'ui-palette' });
    palette.append(
      element('h3', { text: 'Components' }),
      element('p', { className: 'palette-hint', text: 'Drag onto the canvas. Flow containers reorder automatically; Canvas and Overlay use free positioning.' }),
    );

    const grouped = new Map<string, HuiControlSchema[]>();
    for (const schema of listHuiControlSchemas()) {
      const group = grouped.get(schema.group) ?? [];
      group.push(schema);
      grouped.set(schema.group, group);
    }

    for (const [groupName, controls] of grouped) {
      const group = element('section', { className: 'palette-group' });
      group.append(element('h4', { text: groupName }));
      for (const schema of controls) {
        if (schema.type === 'Window') continue;
        const item = button(schema.label, () => this.addNode(schema.type), 'component-palette-item');
        item.draggable = true;
        item.dataset.controlType = schema.type;
        item.addEventListener('dragstart', (event) => {
          event.dataTransfer?.setData(EDITOR_MIME_NEW, schema.type);
          if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy';
        });
        group.append(item);
      }
      palette.append(group);
    }
    return palette;
  }

  private renderCanvas(): HTMLElement {
    const viewport = element('div', { className: 'ui-preview-viewport hui-design-surface' });
    const scale = Math.min(1, 960 / this.previewWidth, 660 / this.previewHeight);
    this.previewScale = scale;
    const device = element('div', { className: 'ui-preview-device' });
    device.style.width = `${this.previewWidth}px`;
    device.style.height = `${this.previewHeight}px`;
    device.style.transform = `scale(${scale})`;
    device.style.transformOrigin = 'top left';
    device.style.setProperty('--hui-grid-size', `${this.gridSize}px`);
    if (this.snapEnabled) device.classList.add('show-layout-grid');
    viewport.style.setProperty('--preview-width', `${this.previewWidth * scale}px`);
    viewport.style.setProperty('--preview-height', `${this.previewHeight * scale}px`);

    const preview = renderHui(this.model.root as unknown as HuiNode, this.previewContext(), {
      designMode: true,
      getNodeKey: (node) => (node as unknown as UiNode)._editorId,
      onNodeCreated: (node, nodeElement) => this.decorateDesignNode(node as unknown as UiNode, nodeElement),
      onNodePointerDown: (node, event, nodeElement) => this.handleNodePointerDown(node as unknown as UiNode, event, nodeElement),
      onNodeClick: (node, event) => {
        event.stopPropagation();
        this.selectNode((node as unknown as UiNode)._editorId);
      },
      onNodeDoubleClick: (node, event) => {
        event.stopPropagation();
        this.selectNode((node as unknown as UiNode)._editorId);
        this.inspector?.querySelector<HTMLInputElement>('input:not([type="checkbox"])')?.focus();
      },
    });
    preview.classList.add('studio-hui-preview-root');
    device.append(preview);
    viewport.append(device);

    viewport.addEventListener('click', (event) => {
      if (event.target === viewport || event.target === device) this.selectNode(this.model.root._editorId);
    });
    viewport.addEventListener('dragover', (event) => {
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    });
    viewport.addEventListener('drop', (event) => {
      event.preventDefault();
      const type = event.dataTransfer?.getData(EDITOR_MIME_NEW);
      if (type) this.addNode(type);
    });
    return viewport;
  }

  private decorateDesignNode(node: UiNode, nodeElement: HTMLElement): void {
    nodeElement.dataset.editorId = node._editorId;
    if (node._editorId === this.selectedId) nodeElement.classList.add('hui-design-selected');

    const parent = findParent(this.model.root, node._editorId);
    if (node !== this.model.root && parent && !isHuiFreeformContainer(parent.type)) {
      nodeElement.draggable = true;
      nodeElement.addEventListener('dragstart', (event) => {
        event.stopPropagation();
        event.dataTransfer?.setData(EDITOR_MIME_EXISTING, node._editorId);
        if (event.dataTransfer) event.dataTransfer.effectAllowed = 'move';
      });
      nodeElement.addEventListener('dragover', (event) => {
        event.preventDefault();
        event.stopPropagation();
        nodeElement.classList.toggle('drop-after', event.clientY > nodeElement.getBoundingClientRect().top + nodeElement.getBoundingClientRect().height / 2);
        nodeElement.classList.toggle('drop-before', !nodeElement.classList.contains('drop-after'));
      });
      nodeElement.addEventListener('dragleave', () => nodeElement.classList.remove('drop-before', 'drop-after'));
      nodeElement.addEventListener('drop', (event) => {
        event.preventDefault();
        event.stopPropagation();
        const parentNode = findParent(this.model.root, node._editorId);
        if (!parentNode) return;
        const index = parentNode.children?.findIndex((child) => child._editorId === node._editorId) ?? -1;
        const after = event.clientY > nodeElement.getBoundingClientRect().top + nodeElement.getBoundingClientRect().height / 2;
        this.handleDrop(event, parentNode, Math.max(0, index + (after ? 1 : 0)));
        nodeElement.classList.remove('drop-before', 'drop-after');
      });
    }

    if (isHuiContainer(node.type)) {
      const host = getChildrenHost(nodeElement, node.type);
      host.addEventListener('dragover', (event) => {
        event.preventDefault();
        event.stopPropagation();
        host.classList.add('drop-container');
      });
      host.addEventListener('dragleave', () => host.classList.remove('drop-container'));
      host.addEventListener('drop', (event) => {
        event.preventDefault();
        event.stopPropagation();
        host.classList.remove('drop-container');
        if (isHuiFreeformContainer(node.type)) {
          this.handleFreeformDrop(event, node, host);
        } else {
          this.handleDrop(event, node, node.children?.length ?? 0);
        }
      });
    }

    if (node._editorId === this.selectedId && parent && isHuiFreeformContainer(parent.type)) {
      this.appendResizeHandles(nodeElement, node);
    }
  }

  private handleNodePointerDown(node: UiNode, event: PointerEvent, nodeElement: HTMLElement): void {
    event.stopPropagation();
    const selectionChanged = this.selectedId !== node._editorId;
    this.selectedId = node._editorId;
    const parent = findParent(this.model.root, node._editorId);
    if (event.button !== 0 || !parent || !isHuiFreeformContainer(parent.type)) {
      if (selectionChanged) this.render();
      return;
    }
    if ((event.target as HTMLElement).closest('.hui-resize-handle')) return;
    event.preventDefault();
    const startX = typeof node.x === 'number' ? node.x : 0;
    const startY = typeof node.y === 'number' ? node.y : 0;
    this.dragPreview = {
      nodeId: node._editorId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startX,
      startY,
      nextX: startX,
      nextY: startY,
      scale: this.previewScale,
      element: nodeElement,
    };
    nodeElement.classList.add('dragging-freeform');
  }

  private appendResizeHandles(nodeElement: HTMLElement, node: UiNode): void {
    for (const direction of ['se'] as const) {
      const handle = element('span', { className: `hui-resize-handle hui-resize-${direction}` });
      handle.addEventListener('pointerdown', (event) => {
        event.preventDefault();
        event.stopPropagation();
        const rect = nodeElement.getBoundingClientRect();
        const startWidth = typeof node.width === 'number' ? node.width : rect.width / this.previewScale;
        const startHeight = typeof node.height === 'number' ? node.height : rect.height / this.previewScale;
        this.resizePreview = {
          nodeId: node._editorId,
          startClientX: event.clientX,
          startClientY: event.clientY,
          startWidth,
          startHeight,
          nextWidth: startWidth,
          nextHeight: startHeight,
          scale: this.previewScale,
          element: nodeElement,
        };
        nodeElement.classList.add('resizing-freeform');
      });
      nodeElement.append(handle);
    }
  }

  private updatePointerOperation(event: PointerEvent): void {
    if (this.dragPreview) {
      const deltaX = (event.clientX - this.dragPreview.startClientX) / this.dragPreview.scale;
      const deltaY = (event.clientY - this.dragPreview.startClientY) / this.dragPreview.scale;
      this.dragPreview.nextX = this.snap(this.dragPreview.startX + deltaX);
      this.dragPreview.nextY = this.snap(this.dragPreview.startY + deltaY);
      this.dragPreview.element.style.left = `${this.dragPreview.nextX}px`;
      this.dragPreview.element.style.top = `${this.dragPreview.nextY}px`;
      return;
    }
    if (this.resizePreview) {
      const deltaX = (event.clientX - this.resizePreview.startClientX) / this.resizePreview.scale;
      const deltaY = (event.clientY - this.resizePreview.startClientY) / this.resizePreview.scale;
      this.resizePreview.nextWidth = Math.max(8, this.snap(this.resizePreview.startWidth + deltaX));
      this.resizePreview.nextHeight = Math.max(8, this.snap(this.resizePreview.startHeight + deltaY));
      this.resizePreview.element.style.width = `${this.resizePreview.nextWidth}px`;
      this.resizePreview.element.style.height = `${this.resizePreview.nextHeight}px`;
    }
  }

  private finishPointerOperation(): void {
    if (this.dragPreview) {
      const operation = this.dragPreview;
      operation.element.classList.remove('dragging-freeform');
      this.dragPreview = null;
      if (operation.nextX === operation.startX && operation.nextY === operation.startY) {
        this.render();
        return;
      }
      this.commit((model) => {
        const node = findNode(model.root, operation.nodeId);
        if (!node) return;
        node.x = operation.nextX;
        node.y = operation.nextY;
      });
      return;
    }
    if (this.resizePreview) {
      const operation = this.resizePreview;
      operation.element.classList.remove('resizing-freeform');
      this.resizePreview = null;
      if (operation.nextWidth === operation.startWidth && operation.nextHeight === operation.startHeight) {
        this.render();
        return;
      }
      this.commit((model) => {
        const node = findNode(model.root, operation.nodeId);
        if (!node) return;
        node.width = operation.nextWidth;
        node.height = operation.nextHeight;
      });
    }
  }

  private handleDrop(event: DragEvent, parent: UiNode, index: number): void {
    const existingId = event.dataTransfer?.getData(EDITOR_MIME_EXISTING);
    const newType = event.dataTransfer?.getData(EDITOR_MIME_NEW);
    if (existingId) this.moveNode(existingId, parent._editorId, index);
    else if (newType) this.addNodeAt(newType, parent._editorId, index);
  }

  private handleFreeformDrop(event: DragEvent, parent: UiNode, host: HTMLElement): void {
    const newType = event.dataTransfer?.getData(EDITOR_MIME_NEW);
    const existingId = event.dataTransfer?.getData(EDITOR_MIME_EXISTING);
    const rect = host.getBoundingClientRect();
    const x = this.snap((event.clientX - rect.left) / this.previewScale);
    const y = this.snap((event.clientY - rect.top) / this.previewScale);
    if (newType) this.addNodeAt(newType, parent._editorId, parent.children?.length ?? 0, { x, y });
    else if (existingId) {
      this.moveNode(existingId, parent._editorId, parent.children?.length ?? 0, { x, y });
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
    row.addEventListener('click', () => this.selectNode(node._editorId));
    row.addEventListener('dragstart', (event) => event.dataTransfer?.setData(EDITOR_MIME_EXISTING, node._editorId));
    row.addEventListener('dragover', (event) => {
      event.preventDefault();
      row.classList.toggle('drop-after', event.clientY > row.getBoundingClientRect().top + row.getBoundingClientRect().height / 2);
      row.classList.toggle('drop-before', !row.classList.contains('drop-after'));
    });
    row.addEventListener('dragleave', () => row.classList.remove('drop-before', 'drop-after'));
    row.addEventListener('drop', (event) => {
      event.preventDefault();
      event.stopPropagation();
      const parent = findParent(this.model.root, node._editorId);
      if (parent) {
        const index = parent.children?.findIndex((child) => child._editorId === node._editorId) ?? 0;
        const after = event.clientY > row.getBoundingClientRect().top + row.getBoundingClientRect().height / 2;
        this.handleDrop(event, parent, index + (after ? 1 : 0));
      } else if (isHuiContainer(node.type)) {
        this.handleDrop(event, node, node.children?.length ?? 0);
      }
      row.classList.remove('drop-before', 'drop-after');
    });
    host.append(row);
    for (const child of node.children ?? []) this.renderHierarchyNode(child, host, depth + 1);
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    const node = findNode(this.model.root, this.selectedId) ?? this.model.root;
    const schema = getHuiControlSchema(node.type);
    this.inspector.append(element('h2', { text: `${schema?.label ?? node.type} Inspector` }));

    this.inspector.append(field('Type', selectInput(node.type, listHuiControlSchemas().map((entry) => entry.type), (value) => {
      this.commit((model) => {
        const target = findNode(model.root, this.selectedId);
        if (!target) return;
        target.type = value;
        if (isHuiContainer(value)) target.children ??= [];
        else delete target.children;
      });
    })));

    const parent = findParent(this.model.root, node._editorId);
    const properties = [...(schema?.properties ?? [])];
    if (parent && isHuiFreeformContainer(parent.type)) {
      properties.push(
        { name: 'x', label: 'Position X', category: 'Layout', editor: 'number', step: 1 },
        { name: 'y', label: 'Position Y', category: 'Layout', editor: 'number', step: 1 },
        { name: 'anchorLeft', label: 'Anchor left', category: 'Layout', editor: 'boolean' },
        { name: 'anchorRight', label: 'Anchor right', category: 'Layout', editor: 'boolean' },
        { name: 'anchorTop', label: 'Anchor top', category: 'Layout', editor: 'boolean' },
        { name: 'anchorBottom', label: 'Anchor bottom', category: 'Layout', editor: 'boolean' },
      );
    }

    const categories = ['Content', 'Layout', 'Appearance', 'Behavior', 'Events'] as const;
    for (const category of categories) {
      const categoryProperties = properties.filter((property) => property.category === category);
      if (categoryProperties.length === 0) continue;
      const section = element('section', { className: 'inspector-section' });
      section.append(element('h3', { text: category }));
      for (const property of categoryProperties) section.append(this.renderPropertyField(node, property));
      this.inspector.append(section);
    }

    if (node !== this.model.root) {
      this.inspector.append(
        button('Duplicate selected node', () => this.duplicateSelected(), 'secondary-button'),
        button('Delete selected node', () => this.deleteSelected(), 'danger-button'),
      );
    }
    this.inspector.append(element('p', {
      className: 'inspector-note',
      text: `Shared runtime schema: ${listHuiControlSchemas().length} controls. Localization keys found: ${this.metadata.localizationKeys.length}. Designer preview uses the same renderer as the game client.`,
    }));
  }

  private renderPropertyField(node: UiNode, property: HuiPropertySchema): HTMLElement {
    const current = node[property.name];
    switch (property.editor) {
      case 'number': {
        const value = typeof current === 'number' ? current : typeof property.defaultValue === 'number' ? property.defaultValue : 0;
        return field(property.label, numberInput(value, (next) => this.updateSelected(property.name, next), {
          ...(property.minimum !== undefined ? { min: property.minimum } : {}),
          ...(property.maximum !== undefined ? { max: property.maximum } : {}),
          ...(property.step !== undefined ? { step: property.step } : {}),
        }), property.hint);
      }
      case 'boolean': {
        const value = typeof current === 'boolean' ? current : Boolean(property.defaultValue ?? false);
        return field(property.label, checkboxInput(value, (next) => this.updateSelected(property.name, next)), property.hint);
      }
      case 'select': {
        const options = property.options ?? [];
        const value = typeof current === 'string' ? current : String(property.defaultValue ?? options[0] ?? '');
        return field(property.label, selectInput(value, options, (next) => this.updateSelected(property.name, next)), property.hint);
      }
      case 'dimension':
        return this.dimensionField(property.label, property.name, current);
      case 'binding':
      case 'items':
        return this.bindingField(property.label, property.name, current, property.hint);
      case 'action':
        return this.actionField(property.label, property.name, current as HuiAction | undefined);
      case 'resource':
        return this.resourceField(property.label, property.name, current, property.hint);
      case 'text':
      default:
        return field(property.label, textInput(current === undefined ? '' : String(current), (next) => this.updateSelected(property.name, parseDesignerValue(next))), property.hint);
    }
  }

  private bindingField(label: string, property: string, value: unknown, hint?: string): HTMLElement {
    const wrapper = element('div', { className: 'binding-field' });
    const input = textInput(value === undefined ? '' : formatEditorValue(value), (next) => this.updateSelected(property, parseDesignerValue(next)));
    const actions = element('div', { className: 'binding-actions' });
    actions.append(button('Bind', () => void this.openBindingEditor(property, value), 'mini-button'));
    if (['text', 'title', 'placeholder', 'tooltip', 'alt'].includes(property)) {
      actions.append(button('FTL', () => void this.openLocalizationPicker(property, value), 'mini-button'));
    }
    wrapper.append(field(label, input, hint), actions);
    return wrapper;
  }

  private resourceField(label: string, property: string, value: unknown, hint?: string): HTMLElement {
    const wrapper = element('div', { className: 'resource-field' });
    const input = textInput(value === undefined ? '' : String(value), (next) => this.updateSelected(property, next || undefined));
    const browse = button('Browse', () => void this.openResourcePicker(property, value), 'mini-button');
    wrapper.append(field(label, input, hint), browse);
    return wrapper;
  }

  private actionField(label: string, property: string, value: HuiAction | undefined): HTMLElement {
    const wrapper = element('div', { className: 'action-field' });
    const summary = element('code', { className: 'action-summary', text: summarizeAction(value) });
    const edit = button('Edit…', () => void this.openActionEditor(property, value), 'mini-button');
    wrapper.append(field(label, summary), edit);
    return wrapper;
  }

  private dimensionField(label: string, property: string, value: unknown): HTMLElement {
    const wrapper = element('div', { className: 'dimension-field' });
    const input = textInput(value === undefined ? '' : String(value), (next) => {
      const numeric = Number(next);
      this.updateSelected(property, next === '' ? undefined : Number.isFinite(numeric) ? numeric : next);
    });
    const auto = button('Auto', () => this.updateSelected(property, 'auto'), 'mini-button');
    const fill = button('Fill', () => this.updateSelected(property, 'fill'), 'mini-button');
    const controls = element('div', { className: 'dimension-actions' });
    controls.append(auto, fill);
    wrapper.append(field(label, input), controls);
    return wrapper;
  }

  private async openBindingEditor(property: string, current: unknown): Promise<void> {
    try {
      const selected = await modal<string>('Bind property', (body, resolve) => {
        const search = element('input', { attrs: { placeholder: 'Search state path…' } });
        const custom = element('input', { attrs: { placeholder: '$state.path' } });
        custom.value = typeof current === 'string' && current.startsWith('$') ? current : '$state.';
        const paths = listBindingPaths(this.previewState).filter((path) => path !== '$state');
        const list = element('div', { className: 'binding-path-list' });
        const renderPaths = (): void => {
          list.replaceChildren();
          const query = search.value.trim().toLowerCase();
          for (const path of paths.filter((entry) => entry.toLowerCase().includes(query)).slice(0, 250)) {
            list.append(button(path, () => resolve(path), 'binding-path-button'));
          }
        };
        search.addEventListener('input', renderPaths);
        renderPaths();
        const footer = element('div', { className: 'modal-actions' });
        footer.append(
          button('Clear binding', () => resolve(''), 'secondary-button'),
          button('Use custom path', () => resolve(custom.value.trim()), 'primary-button'),
        );
        body.append(
          element('p', { text: `Choose a typed path from Preview State for '${property}', or enter a custom binding.` }),
          search,
          list,
          field('Custom binding', custom),
          footer,
        );
      });
      this.updateSelected(property, selected ? selected : undefined);
    } catch (error) {
      if (!isAbort(error)) throw error;
    }
  }

  private async openLocalizationPicker(property: string, current: unknown): Promise<void> {
    try {
      const key = await modal<string>('Localization key', (body, resolve) => {
        const search = element('input', { attrs: { placeholder: 'Search FTL keys…' } });
        const custom = element('input', { attrs: { placeholder: 'new-localization-key' } });
        custom.value = typeof current === 'string' && !current.startsWith('$') ? current : '';
        const list = element('div', { className: 'binding-path-list' });
        const renderKeys = (): void => {
          list.replaceChildren();
          const query = search.value.trim().toLowerCase();
          for (const entry of this.metadata.localizationKeys.filter((item) => item.toLowerCase().includes(query)).slice(0, 300)) {
            list.append(button(entry, () => resolve(entry), 'binding-path-button'));
          }
        };
        search.addEventListener('input', renderKeys);
        renderKeys();
        const footer = element('div', { className: 'modal-actions' });
        footer.append(
          button('Clear', () => resolve(''), 'secondary-button'),
          button('Use key', () => resolve(custom.value.trim()), 'primary-button'),
        );
        body.append(
          element('p', { text: 'Choose an existing FTL key or enter a new key. The HUI file stores only the key.' }),
          search,
          list,
          field('Custom key', custom),
          footer,
        );
      });
      this.updateSelected(property, key || undefined);
    } catch (error) {
      if (!isAbort(error)) throw error;
    }
  }

  private async openResourcePicker(property: string, current: unknown): Promise<void> {
    try {
      const resource = await modal<string>('Select project resource', (body, resolve) => {
        const search = element('input', { attrs: { placeholder: 'Search images, RSI and audio…' } });
        const custom = element('input', { attrs: { placeholder: '/Resources/Textures/example.png' } });
        custom.value = typeof current === 'string' ? current : '';
        const list = element('div', { className: 'resource-picker-list' });
        const renderResources = (): void => {
          list.replaceChildren();
          const query = search.value.trim().toLowerCase();
          const filtered = [...new Set([...this.metadata.rsiDirectories, ...this.metadata.assets])]
            .filter((path) => path.toLowerCase().includes(query))
            .slice(0, 400);
          for (const path of filtered) {
            const item = button('', () => resolve(normalizeResourcePath(path)), 'resource-picker-item');
            const preview = element('span', { className: 'resource-picker-preview', text: resourceGlyph(path) });
            const label = element('span', { className: 'resource-picker-path', text: path });
            item.append(preview, label);
            list.append(item);
          }
          if (filtered.length === 0) list.append(element('p', { className: 'empty-state', text: 'No matching assets found in the project tree.' }));
        };
        search.addEventListener('input', renderResources);
        renderResources();
        const footer = element('div', { className: 'modal-actions' });
        footer.append(
          button('Clear', () => resolve(''), 'secondary-button'),
          button('Use path', () => resolve(custom.value.trim()), 'primary-button'),
        );
        body.append(search, list, field('Custom resource path', custom), footer);
      });
      this.updateSelected(property, resource || undefined);
    } catch (error) {
      if (!isAbort(error)) throw error;
    }
  }

  private async openActionEditor(property: string, current: HuiAction | undefined): Promise<void> {
    try {
      const action = await modal<HuiAction | undefined>('Declarative action', (body, resolve) => {
        const initialType = typeof current === 'string' ? 'ControllerAction' : current?.type ?? 'SendMessage';
        const typeSelect = selectInput(initialType, ['ControllerAction', 'SendMessage', 'CallController', 'SetState', 'ToggleState', 'OpenWindow', 'CloseWindow', 'PlaySound'], () => renderFields());
        const fields = element('div', { className: 'action-editor-fields' });
        const renderFields = (): void => {
          fields.replaceChildren();
          const type = typeSelect.value;
          if (type === 'ControllerAction') {
            const actionInput = textInput(typeof current === 'string' ? current : '', () => undefined);
            fields.append(field('Action ID', actionInput));
            actionInput.dataset.actionField = 'action';
          } else if (type === 'SendMessage') {
            const existing = actionOfType(current, 'SendMessage');
            const message = textInput(existing?.message ?? 'ui-message', () => undefined);
            const argumentsInput = createJsonTextarea(existing?.arguments ?? {});
            message.dataset.actionField = 'message';
            argumentsInput.dataset.actionField = 'arguments';
            fields.append(field('Message ID', message), field('Arguments (JSON)', argumentsInput));
          } else if (type === 'CallController') {
            const existing = actionOfType(current, 'CallController');
            const actionInput = textInput(existing?.action ?? 'controller-action', () => undefined);
            const argumentsInput = createJsonTextarea(existing?.arguments ?? {});
            actionInput.dataset.actionField = 'action';
            argumentsInput.dataset.actionField = 'arguments';
            fields.append(field('Controller action', actionInput), field('Arguments (JSON)', argumentsInput));
          } else if (type === 'SetState') {
            const existing = actionOfType(current, 'SetState');
            const pathInput = textInput(existing?.path ?? '$state.value', () => undefined);
            const valueInput = createJsonTextarea(existing?.value ?? true);
            pathInput.dataset.actionField = 'path';
            valueInput.dataset.actionField = 'value';
            fields.append(field('State path', pathInput), field('Value (JSON or binding)', valueInput));
          } else if (type === 'ToggleState') {
            const existing = actionOfType(current, 'ToggleState');
            const pathInput = textInput(existing?.path ?? '$state.visible', () => undefined);
            pathInput.dataset.actionField = 'path';
            fields.append(field('State path', pathInput));
          } else if (type === 'OpenWindow') {
            const existing = actionOfType(current, 'OpenWindow');
            const windowInput = textInput(existing?.window ?? 'window-id', () => undefined);
            windowInput.dataset.actionField = 'window';
            fields.append(field('Window ID', windowInput));
          } else if (type === 'CloseWindow') {
            const existing = actionOfType(current, 'CloseWindow');
            const windowInput = textInput(existing?.window ?? '', () => undefined);
            windowInput.dataset.actionField = 'window';
            fields.append(field('Window ID (optional)', windowInput));
          } else if (type === 'PlaySound') {
            const existing = actionOfType(current, 'PlaySound');
            const sourceInput = textInput(existing?.source ?? '/Audio/click.ogg', () => undefined);
            sourceInput.dataset.actionField = 'source';
            fields.append(field('Sound resource', sourceInput));
          }
        };
        renderFields();
        const footer = element('div', { className: 'modal-actions' });
        footer.append(
          button('Remove action', () => resolve(undefined), 'danger-button'),
          button('Apply', () => {
            try {
              resolve(readActionForm(typeSelect.value, fields));
            } catch (error) {
              toast(error instanceof Error ? error.message : String(error), 'error');
            }
          }, 'primary-button'),
        );
        body.append(field('Action type', typeSelect), fields, footer);
      });
      this.updateSelected(property, action);
    } catch (error) {
      if (!isAbort(error)) throw error;
    }
  }

  private async editPreviewState(): Promise<void> {
    try {
      const state = await modal<Record<string, unknown>>('Preview state', (body, resolve) => {
        const editor = createJsonTextarea(this.previewState);
        editor.className = 'json-editor';
        const footer = element('div', { className: 'modal-actions' });
        footer.append(
          button('Reset example', () => {
            editor.value = JSON.stringify(DEFAULT_PREVIEW_STATE, null, 2);
          }, 'secondary-button'),
          button('Apply preview state', () => {
            try {
              const parsed = JSON.parse(editor.value) as unknown;
              if (!isRecord(parsed)) throw new Error('Preview state must be a JSON object.');
              resolve(parsed);
            } catch (error) {
              toast(error instanceof Error ? error.message : String(error), 'error');
            }
          }, 'primary-button'),
        );
        body.append(
          element('p', { text: 'This state is used by bindings in the live WYSIWYG preview and by the Binding picker.' }),
          editor,
          footer,
        );
      });
      this.previewState = state;
      this.render();
    } catch (error) {
      if (!isAbort(error)) throw error;
    }
  }

  private addNode(type: string): void {
    const selected = findNode(this.model.root, this.selectedId) ?? this.model.root;
    const target = isHuiContainer(selected.type) ? selected : findParent(this.model.root, selected._editorId) ?? this.model.root;
    this.addNodeAt(type, target._editorId, target.children?.length ?? 0);
  }

  private addNodeAt(type: string, parentId: string, index: number, position?: { x: number; y: number }): void {
    const parent = findNode(this.model.root, parentId);
    if (!parent || !isHuiContainer(parent.type)) return;
    const schema = getHuiControlSchema(parent.type);
    if (schema?.maximumChildren !== undefined && (parent.children?.length ?? 0) >= schema.maximumChildren) {
      toast(`${parent.type} supports only ${schema.maximumChildren} child controls.`, 'error');
      return;
    }
    const node = createEditorNode(type);
    if (isHuiFreeformContainer(parent.type)) {
      node.x = position?.x ?? this.snap(24 + (parent.children?.length ?? 0) * 12);
      node.y = position?.y ?? this.snap(24 + (parent.children?.length ?? 0) * 12);
      if (node.width === undefined) node.width = defaultFreeformWidth(type);
      if (node.height === undefined) node.height = defaultFreeformHeight(type);
    }
    this.commit((model) => {
      const target = findNode(model.root, parentId);
      if (!target) return;
      target.children ??= [];
      target.children.splice(Math.max(0, Math.min(index, target.children.length)), 0, node);
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
    this.commit((model) => findNode(model.root, parent._editorId)?.children?.splice(index, 1));
    this.selectedId = parent._editorId;
    this.render();
  }

  private moveNode(nodeId: string, parentId: string, index: number, position?: { x: number; y: number }): void {
    if (nodeId === this.model.root._editorId || nodeId === parentId) return;
    const node = findNode(this.model.root, nodeId);
    const destination = findNode(this.model.root, parentId);
    if (!node || !destination || !isHuiContainer(destination.type) || hierarchyContainsId(node, parentId)) return;
    const destinationSchema = getHuiControlSchema(destination.type);
    if (destinationSchema?.maximumChildren !== undefined && destination !== findParent(this.model.root, nodeId) && (destination.children?.length ?? 0) >= destinationSchema.maximumChildren) {
      toast(`${destination.type} supports only ${destinationSchema.maximumChildren} child controls.`, 'error');
      return;
    }
    const oldParent = findParent(this.model.root, nodeId);
    if (!oldParent?.children) return;
    this.commit((model) => {
      const modelOldParent = findNode(model.root, oldParent._editorId);
      const modelDestination = findNode(model.root, parentId);
      if (!modelOldParent?.children || !modelDestination) return;
      const oldIndex = modelOldParent.children.findIndex((child) => child._editorId === nodeId);
      const [moved] = modelOldParent.children.splice(oldIndex, 1);
      if (!moved) return;
      modelDestination.children ??= [];
      let destinationIndex = Math.max(0, Math.min(index, modelDestination.children.length));
      if (modelOldParent === modelDestination && oldIndex < destinationIndex) destinationIndex -= 1;
      modelDestination.children.splice(destinationIndex, 0, moved);
      if (isHuiFreeformContainer(modelDestination.type)) {
        moved.x = position?.x ?? moved.x ?? 24;
        moved.y = position?.y ?? moved.y ?? 24;
        if (moved.width === undefined) moved.width = defaultFreeformWidth(moved.type);
        if (moved.height === undefined) moved.height = defaultFreeformHeight(moved.type);
      } else {
        delete moved.x;
        delete moved.y;
        delete moved.anchorLeft;
        delete moved.anchorRight;
        delete moved.anchorTop;
        delete moved.anchorBottom;
      }
    });
    this.selectedId = nodeId;
  }

  private updateSelected(property: string, value: unknown): void {
    this.commit((model) => {
      const target = findNode(model.root, this.selectedId);
      if (!target) return;
      if (value === undefined || value === '') delete target[property];
      else target[property] = value;
    });
  }

  private duplicateSelected(): void {
    const selected = findNode(this.model.root, this.selectedId);
    const parent = selected ? findParent(this.model.root, selected._editorId) : null;
    if (!selected || !parent?.children) return;
    const duplicate = cloneWithNewIds(selected);
    if (isHuiFreeformContainer(parent.type)) {
      duplicate.x = this.snap((duplicate.x ?? 0) + this.gridSize * 2);
      duplicate.y = this.snap((duplicate.y ?? 0) + this.gridSize * 2);
    }
    const index = parent.children.findIndex((child) => child._editorId === selected._editorId);
    this.commit((model) => findNode(model.root, parent._editorId)?.children?.splice(index + 1, 0, duplicate));
    this.selectedId = duplicate._editorId;
    this.render();
  }

  private copySelected(): void {
    const selected = findNode(this.model.root, this.selectedId);
    if (!selected) return;
    this.copiedNode = cloneWithNewIds(selected);
    toast('UI node copied.', 'success');
  }

  private pasteNode(): void {
    if (!this.copiedNode) return;
    const selected = findNode(this.model.root, this.selectedId) ?? this.model.root;
    const parent = isHuiContainer(selected.type) ? selected : findParent(this.model.root, selected._editorId) ?? this.model.root;
    const pasted = cloneWithNewIds(this.copiedNode);
    if (isHuiFreeformContainer(parent.type)) {
      pasted.x = this.snap((pasted.x ?? 0) + this.gridSize * 2);
      pasted.y = this.snap((pasted.y ?? 0) + this.gridSize * 2);
    }
    this.commit((model) => {
      const target = findNode(model.root, parent._editorId);
      if (!target) return;
      target.children ??= [];
      target.children.push(pasted);
    });
    this.selectedId = pasted._editorId;
    this.render();
  }

  private selectNode(id: string): void {
    if (this.selectedId === id) return;
    this.selectedId = id;
    const parent = findParent(this.model.root, id);
    if (parent?.type === 'TabContainer') {
      const index = parent.children?.findIndex((child) => child._editorId === id) ?? -1;
      if (index >= 0) parent.activeTab = index;
    }
    this.render();
  }

  private handleEditorShortcut(event: KeyboardEvent): void {
    if (this.isSourceEnabled || isEditingText(event.target)) return;
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key.toLowerCase() === 'd') {
      event.preventDefault();
      this.duplicateSelected();
      return;
    }
    if (modifier && event.key.toLowerCase() === 'c') {
      event.preventDefault();
      this.copySelected();
      return;
    }
    if (modifier && event.key.toLowerCase() === 'v') {
      event.preventDefault();
      this.pasteNode();
      return;
    }
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      this.deleteSelected();
      return;
    }
    const delta = event.shiftKey ? this.gridSize : 1;
    const directions: Record<string, [number, number]> = {
      ArrowLeft: [-delta, 0],
      ArrowRight: [delta, 0],
      ArrowUp: [0, -delta],
      ArrowDown: [0, delta],
    };
    const direction = directions[event.key];
    if (!direction) return;
    const selected = findNode(this.model.root, this.selectedId);
    const parent = selected ? findParent(this.model.root, selected._editorId) : null;
    if (!selected || !parent || !isHuiFreeformContainer(parent.type)) return;
    event.preventDefault();
    this.commit((model) => {
      const target = findNode(model.root, this.selectedId);
      if (!target) return;
      target.x = this.snap((target.x ?? 0) + direction[0]);
      target.y = this.snap((target.y ?? 0) + direction[1]);
    });
  }

  private previewContext() {
    return {
      state: this.previewState,
      localize: (key: string) => key,
      action: () => undefined,
      sendMessage: () => undefined,
      setState: () => undefined,
      openWindow: () => undefined,
      closeWindow: () => undefined,
      playSound: () => undefined,
      resolveResource: (source: string) => this.project?.getObjectUrl(source) ?? source,
    };
  }

  private snap(value: number): number {
    if (!this.snapEnabled) return Math.round(value * 100) / 100;
    return Math.round(value / this.gridSize) * this.gridSize;
  }
}

function parseUi(source: string): UiModel {
  const parsed = YAML.parse(source) as unknown;
  if (!isRecord(parsed) || typeof parsed.type !== 'string') throw new Error('Expected a HUI root object with a type.');
  return { root: addEditorIds(parsed) };
}

function addEditorIds(value: Record<string, unknown>): UiNode {
  const sourceType = String(value.type ?? 'Panel');
  const node: UiNode = { _editorId: crypto.randomUUID(), type: sourceType === 'Input' ? 'TextInput' : sourceType };
  for (const [key, raw] of Object.entries(value)) {
    if (key === 'children' && Array.isArray(raw)) node.children = raw.filter(isRecord).map(addEditorIds);
    else if (key === 'size' && Array.isArray(raw)) {
      const width = raw[0];
      const height = raw[1];
      if (typeof width === 'number' || typeof width === 'string') node.width = width;
      if (typeof height === 'number' || typeof height === 'string') node.height = height;
    } else if (key !== 'type') node[key] = raw;
  }
  if (isHuiContainer(node.type)) node.children ??= [];
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

function createEditorNode(type: string): UiNode {
  const node = createHuiNode(type);
  return addEditorIds(node);
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

function cloneWithNewIds(node: UiNode): UiNode {
  const clone = structuredClone(node);
  const assign = (target: UiNode): void => {
    target._editorId = crypto.randomUUID();
    if (target.id) target.id = `${target.id}-copy`;
    for (const child of target.children ?? []) assign(child);
  };
  assign(clone);
  return clone;
}

function getChildrenHost(nodeElement: HTMLElement, type: string): HTMLElement {
  if (type === 'Window') return nodeElement.querySelector<HTMLElement>(':scope > .hui-window-content') ?? nodeElement;
  if (type === 'TabContainer') return nodeElement.querySelector<HTMLElement>(':scope > .hui-tabcontent') ?? nodeElement;
  return nodeElement;
}

function defaultFreeformWidth(type: string): number {
  if (type === 'Label' || type === 'Checkbox') return 180;
  if (type === 'Button' || type === 'TextInput' || type === 'Dropdown') return 220;
  if (type === 'Slider' || type === 'ProgressBar') return 240;
  if (type === 'Image') return 160;
  return 260;
}

function defaultFreeformHeight(type: string): number {
  if (type === 'Label') return 32;
  if (type === 'Button' || type === 'TextInput' || type === 'Dropdown' || type === 'Checkbox') return 42;
  if (type === 'Slider' || type === 'ProgressBar') return 32;
  if (type === 'Image') return 160;
  return 180;
}

function parseDesignerValue(value: string): unknown {
  const trimmed = value.trim();
  if (trimmed === '') return undefined;
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const numeric = Number(trimmed);
  return Number.isFinite(numeric) ? numeric : trimmed;
}

function formatEditorValue(value: unknown): string {
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

function summarizeAction(value: HuiAction | undefined): string {
  if (!value) return 'No action';
  if (typeof value === 'string') return `Controller: ${value}`;
  if (value.type === 'SendMessage') return `Send ${value.message}`;
  if (value.type === 'CallController') return `Call ${value.action}`;
  if (value.type === 'SetState') return `Set ${value.path}`;
  if (value.type === 'ToggleState') return `Toggle ${value.path}`;
  if (value.type === 'OpenWindow') return `Open ${value.window}`;
  if (value.type === 'CloseWindow') return `Close ${value.window ?? 'current'}`;
  if (value.type === 'PlaySound') return `Play ${value.source}`;
  return 'Sequence';
}

function readActionForm(type: string, host: HTMLElement): HuiAction {
  const get = (name: string): HTMLInputElement | HTMLTextAreaElement | null => host.querySelector(`[data-action-field="${name}"]`);
  if (type === 'ControllerAction') return get('action')?.value.trim() || 'action';
  if (type === 'SendMessage') return { type: 'SendMessage', message: get('message')?.value.trim() || 'ui-message', arguments: parseJsonRecord(get('arguments')?.value ?? '{}') };
  if (type === 'CallController') return { type: 'CallController', action: get('action')?.value.trim() || 'controller-action', arguments: parseJsonRecord(get('arguments')?.value ?? '{}') };
  if (type === 'SetState') return { type: 'SetState', path: get('path')?.value.trim() || '$state.value', value: parseJsonValue(get('value')?.value ?? 'true') };
  if (type === 'ToggleState') return { type: 'ToggleState', path: get('path')?.value.trim() || '$state.visible' };
  if (type === 'OpenWindow') return { type: 'OpenWindow', window: get('window')?.value.trim() || 'window-id' };
  if (type === 'CloseWindow') {
    const windowValue = get('window')?.value.trim();
    return windowValue ? { type: 'CloseWindow', window: windowValue } : { type: 'CloseWindow' };
  }
  return { type: 'PlaySound', source: get('source')?.value.trim() || '/Audio/click.ogg' };
}

function parseJsonRecord(value: string): Record<string, unknown> {
  const parsed = JSON.parse(value) as unknown;
  if (!isRecord(parsed)) throw new Error('Arguments must be a JSON object.');
  return parsed;
}

function parseJsonValue(value: string): unknown {
  const trimmed = value.trim();
  if (trimmed.startsWith('$')) return trimmed;
  return JSON.parse(trimmed);
}

function createJsonTextarea(value: unknown): HTMLTextAreaElement {
  const textarea = element('textarea');
  textarea.spellcheck = false;
  textarea.value = JSON.stringify(value, null, 2);
  return textarea;
}

type HuiActionObject = Exclude<HuiAction, string>;

function actionOfType<TType extends HuiActionObject['type']>(
  value: HuiAction | undefined,
  type: TType,
): Extract<HuiActionObject, { type: TType }> | undefined {
  if (typeof value !== 'object' || value === null || value.type !== type) return undefined;
  return value as Extract<HuiActionObject, { type: TType }>;
}

function checkboxToolbar(label: string, checked: boolean, onChange: (checked: boolean) => void): HTMLLabelElement {
  const wrapper = element('label', { className: 'toolbar-checkbox' });
  const input = checkboxInput(checked, onChange);
  wrapper.append(input, element('span', { text: label }));
  return wrapper;
}

function normalizeResourcePath(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const lower = normalized.toLowerCase();
  const marker = '/resources/';
  const markerIndex = lower.indexOf(marker);
  if (markerIndex >= 0) return `/Resources/${normalized.slice(markerIndex + marker.length)}`;
  const rootMarker = 'resources/';
  const rootIndex = lower.indexOf(rootMarker);
  if (rootIndex >= 0) return `/Resources/${normalized.slice(rootIndex + rootMarker.length)}`;
  return normalized.startsWith('/') ? normalized : `/${normalized}`;
}

function resourceGlyph(path: string): string {
  const lower = path.toLowerCase();
  if (lower.endsWith('.rsi') || lower.includes('.rsi/')) return 'RSI';
  if (/\.(png|webp|jpg|jpeg|gif|svg)$/.test(lower)) return 'IMG';
  if (/\.(ogg|wav|mp3|flac)$/.test(lower)) return 'SND';
  return 'FILE';
}

function sanitizeId(value: string): string {
  return value.trim().replace(/\.[^.]+$/, '').replace(/[^A-Za-z0-9_.-]+/g, '-').replace(/^-+|-+$/g, '') || 'new-window';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError';
}

function isEditingText(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement || (target instanceof HTMLElement && target.isContentEditable);
}

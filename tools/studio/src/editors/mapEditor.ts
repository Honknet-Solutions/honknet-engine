import YAML from 'yaml';

import { button, checkboxInput, clear, element, field, numberInput, selectInput, textInput } from '../core/dom';
import { StudioProject } from '../core/project';
import type { ProjectMetadata, PrototypeSummary, TileDefinitionSummary, ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type MapEntity = {
  prototype: string;
  position: [number, number];
  rotation: number;
  grid: string;
  components: Record<string, unknown>[];
};

type MapModel = {
  id: string;
  gridId: string;
  width: number;
  height: number;
  tiles: string[][];
  entities: MapEntity[];
};

type MapTool = 'brush' | 'erase' | 'fill' | 'rectangle' | 'line' | 'eyedropper' | 'entity' | 'select' | 'pan';

export class MapEditor extends ModelEditor<MapModel> {
  public readonly kind = 'map' as const;
  public readonly title: string;

  private readonly metadata: ProjectMetadata;
  private readonly project: StudioProject | null;
  private readonly visualCache = new Map<string, CanvasImageSource>();
  private readonly visualLoading = new Map<string, Promise<CanvasImageSource | null>>();
  private tool: MapTool = 'brush';
  private selectedTile = 'floor';
  private selectedPrototype = '';
  private selectedEntityIndex: number | null = null;
  private tilesVisible = true;
  private entitiesVisible = true;
  private collisionVisible = false;
  private gridVisible = true;
  private zoom = 1;
  private panX = 24;
  private panY = 24;
  private isPointerDown = false;
  private pointerButton = 0;
  private lastPointer = { x: 0, y: 0 };
  private rectangleStart: { x: number; y: number } | null = null;
  private lineStart: { x: number; y: number } | null = null;
  private brushSize = 1;
  private copiedEntity: MapEntity | null = null;
  private draggingEntity = false;
  private entityDragOffset: [number, number] = [0, 0];
  private entityDragStartPosition: [number, number] | null = null;
  private cursorTile: { x: number; y: number } | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private resizeObserver: ResizeObserver | null = null;

  public constructor(source: string, path: string, context: ProjectMetadata | StudioProject) {
    super(parseMap(source));
    this.title = path.split('/').at(-1) ?? 'Map';
    this.project = context instanceof StudioProject ? context : null;
    this.metadata = context instanceof StudioProject ? context.projectMetadata : context;
    this.selectedPrototype = this.metadata.prototypes[0] ?? 'DebugPlayer';
  }

  public static create(name: string, width: number, height: number, context: ProjectMetadata | StudioProject): MapEditor {
    const model: MapModel = {
      id: sanitizeId(name),
      gridId: 'main',
      width,
      height,
      tiles: Array.from({ length: height }, (_, y) =>
        Array.from({ length: width }, (_, x) =>
          x === 0 || y === 0 || x === width - 1 || y === height - 1 ? 'wall' : 'floor',
        ),
      ),
      entities: [],
    };
    return new MapEditor(YAML.stringify(toMapDocument(model), { lineWidth: 0 }), `${name}.yml`, context);
  }

  public override unmount(): void {
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    super.unmount();
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;

    // A tool switch rebuilds the canvas. Never carry pointer capture state from
    // the old canvas into the new one or the map can remain stuck in paint/pan.
    this.isPointerDown = false;
    this.pointerButton = 0;
    this.rectangleStart = null;
    this.lineStart = null;
    this.draggingEntity = false;
    this.cursorTile = null;

    const shell = element('div', { className: 'map-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar map-toolbar' });

    const tools: Array<{ tool: MapTool; label: string; key: string }> = [
      { tool: 'select', label: 'Select', key: 'V' },
      { tool: 'brush', label: 'Brush', key: 'B' },
      { tool: 'erase', label: 'Erase', key: 'E' },
      { tool: 'fill', label: 'Fill', key: 'F' },
      { tool: 'rectangle', label: 'Rectangle', key: 'R' },
      { tool: 'line', label: 'Line', key: 'L' },
      { tool: 'eyedropper', label: 'Pick', key: 'I' },
      { tool: 'entity', label: 'Entity', key: 'N' },
      { tool: 'pan', label: 'Pan', key: 'Space' },
    ];
    for (const entry of tools) {
      const toolButton = button(entry.label, () => {
        this.tool = entry.tool;
        this.render();
      }, `tool-button ${this.tool === entry.tool ? 'active' : ''}`);
      toolButton.title = `${entry.label} (${entry.key})`;
      toolbar.append(toolButton);
    }

    toolbar.append(
      element('span', { className: 'toolbar-separator' }),
      element('span', { className: 'toolbar-label', text: 'Brush' }),
      numberInput(this.brushSize, (value) => { this.brushSize = Math.max(1, Math.min(16, Math.round(value))); }, { min: 1, max: 16, step: 1 }),
      button('Copy entity', () => this.copySelectedEntity(), 'tool-button'),
      button('Paste entity', () => this.pasteEntity(), 'tool-button'),
      element('span', { className: 'toolbar-separator' }),
    );
    toolbar.append(button('−', () => this.setZoom(this.zoom / 1.2), 'icon-button'));
    toolbar.append(element('span', { className: 'zoom-label', text: `${Math.round(this.zoom * 100)}%` }));
    toolbar.append(button('+', () => this.setZoom(this.zoom * 1.2), 'icon-button'));
    toolbar.append(button('Fit', () => this.fitMap(), 'tool-button'));

    const body = element('div', { className: 'map-editor-body' });
    const palette = this.renderPalette();
    const canvasHost = element('div', { className: 'map-canvas-host' });
    const canvas = element('canvas', { className: 'map-canvas' });
    canvas.tabIndex = 0;
    canvasHost.append(canvas);
    body.append(palette, canvasHost);
    shell.append(toolbar, body);
    this.container.append(shell);
    this.canvas = canvas;

    this.installCanvasEvents(canvas);
    this.resizeObserver?.disconnect();
    this.resizeObserver = new ResizeObserver(() => this.resizeCanvas());
    this.resizeObserver.observe(canvasHost);
    this.resizeCanvas();
    this.renderInspector();
  }

  protected serializeModel(model: MapModel): string {
    return YAML.stringify(toMapDocument(model), { lineWidth: 0, indent: 2 });
  }

  protected parseSource(source: string): MapModel {
    return parseMap(source);
  }

  protected validateModel(model: MapModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    if (!model.id.trim()) messages.push({ severity: 'error', message: 'Map ID is required.' });
    if (model.width < 1 || model.height < 1) messages.push({ severity: 'error', message: 'Map size must be positive.' });
    if (model.tiles.length !== model.height) messages.push({ severity: 'error', message: 'Tile row count does not match map height.' });
    for (const [index, row] of model.tiles.entries()) {
      if (row.length !== model.width) {
        messages.push({ severity: 'error', message: `Tile row ${index} does not match map width.` });
      }
    }
    for (const [index, entity] of model.entities.entries()) {
      if (!entity.prototype) messages.push({ severity: 'error', message: `Entity ${index} has no prototype.` });
      if (entity.position[0] < 0 || entity.position[1] < 0 || entity.position[0] >= model.width || entity.position[1] >= model.height) {
        messages.push({ severity: 'warning', message: `Entity ${entity.prototype} is outside the map.` });
      }
    }
    if (messages.length === 0) messages.push({ severity: 'info', message: `Map is valid: ${model.width}×${model.height}, ${model.entities.length} entities.` });
    return messages;
  }

  private renderPalette(): HTMLElement {
    const palette = element('aside', { className: 'map-palette' });
    const tabs = element('div', { className: 'palette-tabs' });
    const tileTab = button('Tiles', () => {
      this.tool = 'brush';
      this.render();
    }, this.tool !== 'entity' ? 'active' : '');
    const entityTab = button('Entities', () => {
      this.tool = 'entity';
      this.render();
    }, this.tool === 'entity' ? 'active' : '');
    tabs.append(tileTab, entityTab);
    palette.append(tabs);

    const search = element('input', { className: 'palette-search', attrs: { placeholder: 'Search palette…' } });
    palette.append(search);
    const list = element('div', { className: 'palette-list' });
    palette.append(list);

    const renderItems = (): void => {
      clear(list);
      const query = search.value.trim().toLowerCase();
      if (this.tool === 'entity') {
        const prototypes = this.metadata.prototypes.filter((id) => id.toLowerCase().includes(query));
        for (const prototype of prototypes) {
          const item = button(prototype, () => {
            this.selectedPrototype = prototype;
            this.tool = 'entity';
            renderItems();
          }, `palette-item entity-item ${prototype === this.selectedPrototype ? 'selected' : ''}`);
          const summary = this.metadata.prototypeSummaries.find((entry) => entry.id === prototype);
          item.prepend(this.renderPaletteVisual(summary?.sprite, summary?.state, colorFromString(prototype), prototype.slice(0, 1).toUpperCase()));
          list.append(item);
        }
        if (prototypes.length === 0) list.append(element('p', { className: 'empty-state', text: 'No prototypes found.' }));
      } else {
        for (const tile of this.tileDefinitions().filter((entry) => entry.label.toLowerCase().includes(query) || entry.id.toLowerCase().includes(query) || entry.category.toLowerCase().includes(query))) {
          const item = button(tile.label, () => {
            this.selectedTile = tile.id;
            if (this.tool === 'select' || this.tool === 'entity' || this.tool === 'pan') this.tool = 'brush';
            renderItems();
          }, `palette-item ${tile.id === this.selectedTile ? 'selected' : ''}`);
          item.prepend(this.renderPaletteVisual(tile.sprite, tile.state, tile.color));
          item.append(element('small', { className: 'palette-category', text: tile.category }));
          list.append(item);
        }
      }
    };
    search.addEventListener('input', renderItems);
    renderItems();
    return palette;
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    this.inspector.append(element('h2', { text: this.selectedEntityIndex === null ? 'Map Inspector' : 'Entity Inspector' }));

    if (this.selectedEntityIndex !== null) {
      const entity = this.model.entities[this.selectedEntityIndex];
      if (!entity) {
        this.selectedEntityIndex = null;
        this.renderInspector();
        return;
      }
      this.inspector.append(
        field('Prototype', selectInput(entity.prototype, this.metadata.prototypes.length ? this.metadata.prototypes : [entity.prototype], (value) => {
          this.commit((model) => { const target = model.entities[this.selectedEntityIndex ?? -1]; if (target) target.prototype = value; });
        })),
        field('X', numberInput(entity.position[0], (value) => {
          this.commit((model) => { const target = model.entities[this.selectedEntityIndex ?? -1]; if (target) target.position[0] = value; });
        }, { step: 0.5 })),
        field('Y', numberInput(entity.position[1], (value) => {
          this.commit((model) => { const target = model.entities[this.selectedEntityIndex ?? -1]; if (target) target.position[1] = value; });
        }, { step: 0.5 })),
        field('Rotation', numberInput(entity.rotation, (value) => {
          this.commit((model) => { const target = model.entities[this.selectedEntityIndex ?? -1]; if (target) target.rotation = value; });
        }, { step: 15 })),
        field('Grid', textInput(entity.grid, (value) => {
          this.commit((model) => { const target = model.entities[this.selectedEntityIndex ?? -1]; if (target) target.grid = value; });
        })),
      );
      this.inspector.append(button('Duplicate entity', () => this.duplicateSelectedEntity(), 'secondary-button'));
      this.inspector.append(button('Delete entity', () => {
        const index = this.selectedEntityIndex;
        if (index === null) return;
        this.commit((model) => model.entities.splice(index, 1));
        this.selectedEntityIndex = null;
        this.render();
      }, 'danger-button'));
      return;
    }

    this.inspector.append(
      field('Map ID', textInput(this.model.id, (value) => this.commit((model) => { model.id = value; }))),
      field('Grid ID', textInput(this.model.gridId, (value) => this.commit((model) => {
        const oldId = model.gridId;
        model.gridId = value;
        for (const entity of model.entities) if (entity.grid === oldId) entity.grid = value;
      }))),
    );

    const sizeRow = element('div', { className: 'inspector-row' });
    sizeRow.append(
      field('Width', numberInput(this.model.width, (value) => this.resizeMap(Math.round(value), this.model.height), { min: 1, max: 4096 })),
      field('Height', numberInput(this.model.height, (value) => this.resizeMap(this.model.width, Math.round(value)), { min: 1, max: 4096 })),
    );
    this.inspector.append(sizeRow);

    const layers = element('section', { className: 'inspector-section' });
    layers.append(element('h3', { text: 'Layers' }));
    layers.append(
      field('Tiles', checkboxInput(this.tilesVisible, (value) => { this.tilesVisible = value; this.draw(); })),
      field('Entities', checkboxInput(this.entitiesVisible, (value) => { this.entitiesVisible = value; this.draw(); })),
      field('Collision overlay', checkboxInput(this.collisionVisible, (value) => { this.collisionVisible = value; this.draw(); })),
      field('Grid', checkboxInput(this.gridVisible, (value) => { this.gridVisible = value; this.draw(); })),
    );
    this.inspector.append(layers);
    this.inspector.append(element('p', { className: 'inspector-note', text: 'Mouse wheel: zoom. Middle/right drag: pan. Delete: remove selected entity.' }));
  }

  private installCanvasEvents(canvas: HTMLCanvasElement): void {
    canvas.addEventListener('contextmenu', (event) => event.preventDefault());
    canvas.addEventListener('wheel', (event) => {
      event.preventDefault();
      const before = this.screenToWorld(event.offsetX, event.offsetY);
      this.setZoom(this.zoom * (event.deltaY < 0 ? 1.12 : 1 / 1.12), false);
      const after = this.screenToWorld(event.offsetX, event.offsetY);
      this.panX += (after.x - before.x) * 32 * this.zoom;
      this.panY += (after.y - before.y) * 32 * this.zoom;
      this.draw();
    }, { passive: false });

    canvas.addEventListener('pointerdown', (event) => {
      canvas.focus();
      canvas.setPointerCapture(event.pointerId);
      this.isPointerDown = true;
      this.pointerButton = event.button;
      this.lastPointer = { x: event.offsetX, y: event.offsetY };
      const tile = this.screenToTile(event.offsetX, event.offsetY);
      if (!tile) return;
      this.cursorTile = tile;
      if (event.button === 1 || event.button === 2 || this.tool === 'pan') return;
      if (this.tool === 'rectangle') {
        this.rectangleStart = tile;
        this.draw();
        return;
      }
      if (this.tool === 'line') {
        this.lineStart = tile;
        this.draw();
        return;
      }
      if (this.tool === 'select') {
        const world = this.screenToWorld(event.offsetX, event.offsetY);
        const index = this.findEntityAt(world.x, world.y);
        this.selectedEntityIndex = index;
        if (index !== null) {
          const entity = this.model.entities[index];
          if (entity) {
            this.draggingEntity = true;
            this.entityDragStartPosition = [...entity.position] as [number, number];
            this.entityDragOffset = [entity.position[0] - world.x, entity.position[1] - world.y];
          }
        }
        this.renderInspector();
        this.draw();
        return;
      }
      this.applyTool(tile.x, tile.y);
    });

    canvas.addEventListener('pointermove', (event) => {
      const tile = this.screenToTile(event.offsetX, event.offsetY);
      this.cursorTile = tile;
      if (this.isPointerDown && (this.pointerButton === 1 || this.pointerButton === 2 || this.tool === 'pan')) {
        this.panX += event.offsetX - this.lastPointer.x;
        this.panY += event.offsetY - this.lastPointer.y;
        this.lastPointer = { x: event.offsetX, y: event.offsetY };
        this.draw();
        return;
      }
      if (this.isPointerDown && this.draggingEntity && this.selectedEntityIndex !== null) {
        const world = this.screenToWorld(event.offsetX, event.offsetY);
        const entity = this.model.entities[this.selectedEntityIndex];
        if (entity) {
          entity.position = [
            clamp(world.x + this.entityDragOffset[0], 0, this.model.width),
            clamp(world.y + this.entityDragOffset[1], 0, this.model.height),
          ];
          this.draw();
        }
        return;
      }
      if (this.isPointerDown && tile && (this.tool === 'brush' || this.tool === 'erase')) this.applyTool(tile.x, tile.y, false);
      else this.draw();
    });

    const finishPointer = (event: PointerEvent): void => {
      if (this.tool === 'rectangle' && this.rectangleStart) {
        const end = this.screenToTile(event.offsetX, event.offsetY);
        if (end) this.paintRectangle(this.rectangleStart, end);
      }
      if (this.tool === 'line' && this.lineStart) {
        const end = this.screenToTile(event.offsetX, event.offsetY);
        if (end) this.paintLine(this.lineStart, end);
      }
      if (this.draggingEntity && this.selectedEntityIndex !== null) {
        const index = this.selectedEntityIndex;
        const nextPosition = this.model.entities[index]?.position;
        const previousPosition = this.entityDragStartPosition;
        if (nextPosition && previousPosition) {
          const finalPosition = [...nextPosition] as [number, number];
          const current = this.model.entities[index];
          if (current) current.position = [...previousPosition] as [number, number];
          this.commit((model) => {
            const entity = model.entities[index];
            if (entity) entity.position = finalPosition;
          }, false);
        }
      }
      this.rectangleStart = null;
      this.lineStart = null;
      this.draggingEntity = false;
      this.entityDragStartPosition = null;
      this.isPointerDown = false;
      this.draw();
    };
    canvas.addEventListener('pointerup', finishPointer);
    canvas.addEventListener('pointercancel', finishPointer);
    canvas.addEventListener('pointerleave', () => {
      if (!this.isPointerDown) {
        this.cursorTile = null;
        this.draw();
      }
    });

    canvas.addEventListener('keydown', (event) => {
      if (event.key === 'Delete' && this.selectedEntityIndex !== null) {
        const index = this.selectedEntityIndex;
        this.commit((model) => model.entities.splice(index, 1), false);
        this.selectedEntityIndex = null;
        this.renderInspector();
        this.draw();
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'c') { event.preventDefault(); this.copySelectedEntity(); return; }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'v') { event.preventDefault(); this.pasteEntity(); return; }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'd') { event.preventDefault(); this.duplicateSelectedEntity(); return; }
      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight' || event.key === 'ArrowUp' || event.key === 'ArrowDown') {
        this.nudgeSelectedEntity(event.key, event.shiftKey ? 1 : 0.25);
        event.preventDefault();
        return;
      }
      const shortcuts: Record<string, MapTool> = { v: 'select', b: 'brush', e: 'erase', f: 'fill', r: 'rectangle', l: 'line', i: 'eyedropper', n: 'entity' };
      const next = shortcuts[event.key.toLowerCase()];
      if (next) {
        this.tool = next;
        this.render();
      }
    });
  }

  private applyTool(x: number, y: number, withHistory = true): void {
    if (x < 0 || y < 0 || x >= this.model.width || y >= this.model.height) return;
    if (this.tool === 'brush' || this.tool === 'erase') {
      const nextTile = this.tool === 'erase' ? 'void' : this.selectedTile;
      const paint = (model: MapModel): void => {
        const radius = Math.floor((this.brushSize - 1) / 2);
        for (let py = y - radius; py < y - radius + this.brushSize; py += 1) {
          for (let px = x - radius; px < x - radius + this.brushSize; px += 1) {
            const row = model.tiles[py];
            if (row && px >= 0 && px < model.width) row[px] = nextTile;
          }
        }
      };
      if (withHistory) this.commit(paint, false);
      else paint(this.model);
      this.draw();
      return;
    }
    if (this.tool === 'eyedropper') {
      this.selectedTile = this.model.tiles[y]?.[x] ?? this.selectedTile;
      this.tool = 'brush';
      this.render();
      return;
    }
    if (this.tool === 'fill') {
      this.floodFill(x, y, this.selectedTile);
      return;
    }
    if (this.tool === 'entity') {
      this.commit((model) => model.entities.push({
        prototype: this.selectedPrototype || this.metadata.prototypes[0] || 'NewEntity',
        position: [x + 0.5, y + 0.5],
        rotation: 0,
        grid: model.gridId,
        components: [],
      }), false);
      this.selectedEntityIndex = this.model.entities.length - 1;
      this.renderInspector();
      this.draw();
      return;
    }
    if (this.tool === 'select') {
      this.selectedEntityIndex = this.findEntityAt(x + 0.5, y + 0.5);
      this.renderInspector();
      this.draw();
    }
  }

  private floodFill(startX: number, startY: number, replacement: string): void {
    const target = this.model.tiles[startY]?.[startX];
    if (target === undefined || target === replacement) return;
    this.commit((model) => {
      const queue: Array<[number, number]> = [[startX, startY]];
      const visited = new Set<string>();
      while (queue.length > 0) {
        const current = queue.shift();
        if (!current) break;
        const [x, y] = current;
        const key = `${x},${y}`;
        if (visited.has(key) || x < 0 || y < 0 || x >= model.width || y >= model.height) continue;
        visited.add(key);
        const row = model.tiles[y];
        if (!row || row[x] !== target) continue;
        row[x] = replacement;
        queue.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]);
      }
    }, false);
    this.draw();
  }

  private paintRectangle(start: { x: number; y: number }, end: { x: number; y: number }): void {
    const minX = Math.max(0, Math.min(start.x, end.x));
    const maxX = Math.min(this.model.width - 1, Math.max(start.x, end.x));
    const minY = Math.max(0, Math.min(start.y, end.y));
    const maxY = Math.min(this.model.height - 1, Math.max(start.y, end.y));
    this.commit((model) => {
      for (let y = minY; y <= maxY; y += 1) {
        const row = model.tiles[y];
        if (!row) continue;
        for (let x = minX; x <= maxX; x += 1) row[x] = this.selectedTile;
      }
    }, false);
  }

  private paintLine(start: { x: number; y: number }, end: { x: number; y: number }): void {
    const points = bresenham(start.x, start.y, end.x, end.y);
    this.commit((model) => {
      for (const [x, y] of points) {
        const row = model.tiles[y];
        if (row && x >= 0 && x < model.width) row[x] = this.selectedTile;
      }
    }, false);
  }

  private renderPaletteVisual(source: string | undefined, state: string | undefined, fallback: string, label = ''): HTMLElement {
    const preview = element('canvas', { className: 'palette-visual' });
    preview.width = 28;
    preview.height = 28;
    const context = preview.getContext('2d');
    if (context) {
      context.fillStyle = fallback;
      context.fillRect(0, 0, preview.width, preview.height);
      if (label) {
        context.fillStyle = '#f1fff9';
        context.font = 'bold 13px system-ui';
        context.textAlign = 'center';
        context.textBaseline = 'middle';
        context.fillText(label, preview.width / 2, preview.height / 2);
      }
    }
    if (source) {
      void this.ensureVisual(source, state).then((visual) => {
        const nextContext = preview.getContext('2d');
        if (!visual || !nextContext || !preview.isConnected) return;
        nextContext.clearRect(0, 0, preview.width, preview.height);
        nextContext.imageSmoothingEnabled = false;
        nextContext.drawImage(visual, 2, 2, preview.width - 4, preview.height - 4);
      });
    }
    return preview;
  }

  private requestVisual(source: string | undefined, state: string | undefined): CanvasImageSource | null {
    if (!source || !this.project) return null;
    const key = `${source}#${state ?? ''}`;
    const cached = this.visualCache.get(key);
    if (cached) return cached;
    void this.ensureVisual(source, state).then((visual) => {
      if (visual) this.draw();
    });
    return null;
  }

  private ensureVisual(source: string, state: string | undefined): Promise<CanvasImageSource | null> {
    if (!this.project) return Promise.resolve(null);
    const key = `${source}#${state ?? ''}`;
    const cached = this.visualCache.get(key);
    if (cached) return Promise.resolve(cached);
    const existing = this.visualLoading.get(key);
    if (existing) return existing;
    const loading = this.loadVisual(source, state)
      .then((visual) => {
        if (visual) this.visualCache.set(key, visual);
        return visual;
      })
      .catch(() => null)
      .finally(() => this.visualLoading.delete(key));
    this.visualLoading.set(key, loading);
    return loading;
  }

  private async loadVisual(source: string, state: string | undefined): Promise<CanvasImageSource | null> {
    if (!this.project) return null;
    const normalized = this.project.resolveProjectPath(source).replace(/\/$/, '');
    if (normalized.toLowerCase().endsWith('.rsi')) {
      const rsi = await this.project.readRsi(normalized);
      const selected = rsi.states.find((entry) => entry.name === state) ?? rsi.states[0];
      if (!selected) return null;
      const image = await loadImage(await this.project.getObjectUrl(selected.imagePath));
      const frameWidth = Math.max(1, rsi.meta.size?.x ?? image.naturalWidth ?? image.width);
      const frameHeight = Math.max(1, rsi.meta.size?.y ?? image.naturalHeight ?? image.height);
      const frame = document.createElement('canvas');
      frame.width = frameWidth;
      frame.height = frameHeight;
      const context = frame.getContext('2d');
      if (!context) return image;
      context.imageSmoothingEnabled = false;
      context.drawImage(image, 0, 0, frameWidth, frameHeight, 0, 0, frameWidth, frameHeight);
      return frame;
    }
    return loadImage(await this.project.getObjectUrl(normalized));
  }

  private tileDefinitions(): TileDefinitionSummary[] {
    return this.metadata.tiles.length > 0 ? this.metadata.tiles : [
      { id: 'floor', label: 'Floor', color: '#283843', collision: false, category: 'Fallback' },
      { id: 'wall', label: 'Wall', color: '#7a8992', collision: true, category: 'Fallback' },
      { id: 'void', label: 'Void', color: '#07090c', collision: true, category: 'Fallback' },
    ];
  }

  private tileColor(id: string): string {
    return this.tileDefinitions().find((tile) => tile.id === id)?.color ?? colorFromString(id);
  }

  private tileCollision(id: string): boolean {
    const definition = this.tileDefinitions().find((tile) => tile.id === id);
    return definition?.collision ?? id.toLowerCase().includes('wall');
  }

  private copySelectedEntity(): void {
    const entity = this.selectedEntityIndex === null ? null : this.model.entities[this.selectedEntityIndex];
    this.copiedEntity = entity ? structuredClone(entity) : null;
  }

  private pasteEntity(): void {
    if (!this.copiedEntity) return;
    const entity = structuredClone(this.copiedEntity);
    entity.position = [clamp(entity.position[0] + 0.5, 0, this.model.width), clamp(entity.position[1] + 0.5, 0, this.model.height)];
    this.commit((model) => { model.entities.push(entity); }, false);
    this.selectedEntityIndex = this.model.entities.length - 1;
    this.tool = 'select';
    this.renderInspector();
    this.draw();
  }

  private duplicateSelectedEntity(): void {
    this.copySelectedEntity();
    this.pasteEntity();
  }

  private nudgeSelectedEntity(key: string, amount: number): void {
    const index = this.selectedEntityIndex;
    if (index === null) return;
    this.commit((model) => {
      const entity = model.entities[index];
      if (!entity) return;
      if (key === 'ArrowLeft') entity.position[0] -= amount;
      if (key === 'ArrowRight') entity.position[0] += amount;
      if (key === 'ArrowUp') entity.position[1] -= amount;
      if (key === 'ArrowDown') entity.position[1] += amount;
      entity.position[0] = clamp(entity.position[0], 0, model.width);
      entity.position[1] = clamp(entity.position[1], 0, model.height);
    }, false);
    this.renderInspector();
    this.draw();
  }

  private resizeMap(width: number, height: number): void {
    const safeWidth = Math.max(1, Math.min(4096, width));
    const safeHeight = Math.max(1, Math.min(4096, height));
    if (safeWidth === this.model.width && safeHeight === this.model.height) return;
    this.commit((model) => {
      const next = Array.from({ length: safeHeight }, (_, y) =>
        Array.from({ length: safeWidth }, (_, x) => model.tiles[y]?.[x] ?? 'void'),
      );
      model.width = safeWidth;
      model.height = safeHeight;
      model.tiles = next;
    });
  }

  private findEntityAt(x: number, y: number): number | null {
    let best: number | null = null;
    let bestDistance = 0.55;
    for (const [index, entity] of this.model.entities.entries()) {
      const distance = Math.hypot(entity.position[0] - x, entity.position[1] - y);
      if (distance < bestDistance) {
        best = index;
        bestDistance = distance;
      }
    }
    return best;
  }

  private resizeCanvas(): void {
    const canvas = this.canvas;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.floor(rect.width * dpr));
    canvas.height = Math.max(1, Math.floor(rect.height * dpr));
    this.draw();
  }

  private draw(): void {
    const canvas = this.canvas;
    if (!canvas) return;
    const context = canvas.getContext('2d');
    if (!context) return;
    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, rect.width, rect.height);
    context.fillStyle = '#05080b';
    context.fillRect(0, 0, rect.width, rect.height);

    const cell = 32 * this.zoom;
    context.save();
    context.translate(this.panX, this.panY);

    if (this.tilesVisible) {
      for (let y = 0; y < this.model.height; y += 1) {
        const row = this.model.tiles[y];
        if (!row) continue;
        for (let x = 0; x < this.model.width; x += 1) {
          const tile = row[x] ?? 'void';
          const definition = this.tileDefinitions().find((entry) => entry.id === tile);
          const visual = this.requestVisual(definition?.sprite, definition?.state);
          if (visual) {
            context.imageSmoothingEnabled = false;
            context.drawImage(visual, x * cell, y * cell, cell + 0.5, cell + 0.5);
          } else {
            context.fillStyle = this.tileColor(tile);
            context.fillRect(x * cell, y * cell, cell + 0.5, cell + 0.5);
          }
          if (this.collisionVisible && this.tileCollision(tile)) {
            context.fillStyle = 'rgba(255, 80, 80, 0.38)';
            context.fillRect(x * cell, y * cell, cell, cell);
          }
        }
      }
    }

    if (this.gridVisible && cell >= 8) {
      context.strokeStyle = 'rgba(170, 210, 225, 0.13)';
      context.lineWidth = 1;
      context.beginPath();
      for (let x = 0; x <= this.model.width; x += 1) {
        context.moveTo(x * cell + 0.5, 0);
        context.lineTo(x * cell + 0.5, this.model.height * cell);
      }
      for (let y = 0; y <= this.model.height; y += 1) {
        context.moveTo(0, y * cell + 0.5);
        context.lineTo(this.model.width * cell, y * cell + 0.5);
      }
      context.stroke();
    }

    if (this.entitiesVisible) {
      for (const [index, entity] of this.model.entities.entries()) {
        const x = entity.position[0] * cell;
        const y = entity.position[1] * cell;
        const selected = index === this.selectedEntityIndex;
        context.save();
        context.translate(x, y);
        context.rotate((entity.rotation * Math.PI) / 180);
        const summary = this.metadata.prototypeSummaries.find((entry) => entry.id === entity.prototype);
        const visual = this.requestVisual(summary?.sprite, summary?.state);
        if (visual) {
          context.imageSmoothingEnabled = false;
          context.drawImage(visual, -cell * 0.4, -cell * 0.4, cell * 0.8, cell * 0.8);
        } else {
          context.fillStyle = selected ? '#63ffd0' : colorFromString(entity.prototype);
          context.beginPath();
          context.roundRect(-cell * 0.3, -cell * 0.3, cell * 0.6, cell * 0.6, Math.max(2, cell * 0.08));
          context.fill();
        }
        context.strokeStyle = selected ? '#ffffff' : 'rgba(7, 16, 20, 0.75)';
        context.lineWidth = selected ? 3 : 1.5;
        context.strokeRect(-cell * 0.42, -cell * 0.42, cell * 0.84, cell * 0.84);
        context.restore();
        if (cell >= 20) {
          context.fillStyle = '#f1fff9';
          context.font = `${Math.max(9, Math.min(13, cell * 0.28))}px system-ui`;
          context.textAlign = 'center';
          context.fillText(entity.prototype, x, y + cell * 0.48);
        }
      }
    }

    if (this.cursorTile) {
      context.strokeStyle = '#63ffd0';
      context.lineWidth = 2;
      const start = this.rectangleStart;
      if ((this.tool === 'rectangle' || this.tool === 'line') && (start ?? this.lineStart)) {
        const activeStart = start ?? this.lineStart!;
        if (this.tool === 'line') {
          context.beginPath();
          context.moveTo((activeStart.x + 0.5) * cell, (activeStart.y + 0.5) * cell);
          context.lineTo((this.cursorTile.x + 0.5) * cell, (this.cursorTile.y + 0.5) * cell);
          context.stroke();
        } else {
        const minX = Math.min(activeStart.x, this.cursorTile.x);
        const minY = Math.min(activeStart.y, this.cursorTile.y);
        const width = Math.abs(activeStart.x - this.cursorTile.x) + 1;
        const height = Math.abs(activeStart.y - this.cursorTile.y) + 1;
        context.strokeRect(minX * cell + 1, minY * cell + 1, width * cell - 2, height * cell - 2);
        }
      } else {
        context.strokeRect(this.cursorTile.x * cell + 1, this.cursorTile.y * cell + 1, cell - 2, cell - 2);
      }
    }

    context.restore();

    context.fillStyle = 'rgba(4, 10, 14, 0.82)';
    context.fillRect(10, rect.height - 30, 300, 22);
    context.fillStyle = '#c8f9e8';
    context.font = '12px ui-monospace, monospace';
    const cursorText = this.cursorTile ? `x:${this.cursorTile.x} y:${this.cursorTile.y}` : 'x:- y:-';
    context.fillText(`${cursorText}  ${this.model.width}×${this.model.height}  tool:${this.tool}`, 16, rect.height - 15);
  }

  private setZoom(value: number, redraw = true): void {
    this.zoom = Math.max(0.1, Math.min(6, value));
    if (redraw) this.render();
  }

  private fitMap(): void {
    const canvas = this.canvas;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    this.zoom = Math.max(0.1, Math.min(4, Math.min((rect.width - 48) / (this.model.width * 32), (rect.height - 48) / (this.model.height * 32))));
    this.panX = (rect.width - this.model.width * 32 * this.zoom) / 2;
    this.panY = (rect.height - this.model.height * 32 * this.zoom) / 2;
    this.render();
  }

  private screenToWorld(screenX: number, screenY: number): { x: number; y: number } {
    const cell = 32 * this.zoom;
    return { x: (screenX - this.panX) / cell, y: (screenY - this.panY) / cell };
  }

  private screenToTile(screenX: number, screenY: number): { x: number; y: number } | null {
    const world = this.screenToWorld(screenX, screenY);
    const x = Math.floor(world.x);
    const y = Math.floor(world.y);
    if (x < 0 || y < 0 || x >= this.model.width || y >= this.model.height) return null;
    return { x, y };
  }
}

function parseMap(source: string): MapModel {
  const parsed = YAML.parse(source) as unknown;
  if (!isRecord(parsed) || !isRecord(parsed.map)) throw new Error('Expected a map document with a map root.');
  const map = parsed.map;
  const id = typeof map.id === 'string' ? map.id : 'new-map';
  const grids = Array.isArray(map.grids) ? map.grids : [];
  const grid = grids.find(isRecord) ?? { id: 'main', chunks: [] };
  const gridId = typeof grid.id === 'string' ? grid.id : 'main';
  const chunks = Array.isArray(grid.chunks) ? grid.chunks : [];
  const chunk = chunks.find(isRecord);
  const rawTiles = chunk && Array.isArray(chunk.tiles) ? chunk.tiles : [];
  const tiles = rawTiles.map((row) => Array.isArray(row) ? row.map((tile) => String(tile)) : []);
  const width = Math.max(1, ...tiles.map((row) => row.length));
  const height = Math.max(1, tiles.length);
  while (tiles.length < height) tiles.push(Array.from({ length: width }, () => 'void'));
  for (const row of tiles) while (row.length < width) row.push('void');

  const entities: MapEntity[] = [];
  if (Array.isArray(map.entities)) {
    for (const raw of map.entities) {
      if (!isRecord(raw) || typeof raw.prototype !== 'string') continue;
      const position = Array.isArray(raw.position) ? raw.position : [0.5, 0.5];
      entities.push({
        prototype: raw.prototype,
        position: [Number(position[0] ?? 0.5), Number(position[1] ?? 0.5)],
        rotation: typeof raw.rotation === 'number' ? raw.rotation : 0,
        grid: typeof raw.grid === 'string' ? raw.grid : gridId,
        components: Array.isArray(raw.components) ? raw.components.filter(isRecord) : [],
      });
    }
  }

  return { id, gridId, width, height, tiles, entities };
}

function toMapDocument(model: MapModel): Record<string, unknown> {
  return {
    map: {
      id: model.id,
      grids: [{
        id: model.gridId,
        position: [0, 0],
        rotation: 0,
        chunks: [{ position: [0, 0], tiles: model.tiles }],
      }],
      entities: model.entities.map((entity) => {
        const result: Record<string, unknown> = {
          prototype: entity.prototype,
          position: entity.position,
          grid: entity.grid,
        };
        if (entity.rotation !== 0) result.rotation = entity.rotation;
        if (entity.components.length > 0) result.components = entity.components;
        return result;
      }),
    },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}


function bresenham(x0: number, y0: number, x1: number, y1: number): Array<[number, number]> {
  const points: Array<[number, number]> = [];
  let x = x0;
  let y = y0;
  const dx = Math.abs(x1 - x0);
  const sx = x0 < x1 ? 1 : -1;
  const dy = -Math.abs(y1 - y0);
  const sy = y0 < y1 ? 1 : -1;
  let error = dx + dy;
  while (true) {
    points.push([x, y]);
    if (x === x1 && y === y1) break;
    const twice = 2 * error;
    if (twice >= dy) { error += dy; x += sx; }
    if (twice <= dx) { error += dx; y += sy; }
  }
  return points;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function sanitizeId(value: string): string {
  return value.trim().replace(/\.[^.]+$/, '').replace(/[^A-Za-z0-9_.-]+/g, '-').replace(/^-+|-+$/g, '') || 'new-map';
}

function loadImage(source: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.decoding = 'async';
    image.addEventListener('load', () => resolve(image), { once: true });
    image.addEventListener('error', () => reject(new Error(`Failed to load image: ${source}`)), { once: true });
    image.src = source;
  });
}

function colorFromString(value: string): string {
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) hash = ((hash << 5) - hash + value.charCodeAt(index)) | 0;
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue} 48% 42%)`;
}

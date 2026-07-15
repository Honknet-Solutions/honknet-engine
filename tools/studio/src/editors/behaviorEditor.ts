import YAML from 'yaml';

import {
  button,
  clear,
  element,
  field,
  selectInput,
  textInput,
} from '../core/dom';
import type { ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type BehaviorNode = {
  _editorId: string;
  node: string;
  [key: string]: unknown;
};

type BehaviorModel = {
  id: string;
  events: Record<string, BehaviorNode[]>;
  positions: Record<string, { x: number; y: number }>;
};

type BranchSlot = 'root' | 'success' | 'failure' | 'children';

type NodeSelection = {
  event: string;
  nodeId: string | null;
  branch: BranchSlot;
};

const EVENTS = [
  'OnSpawn',
  'OnDelete',
  'OnInteract',
  'OnUse',
  'OnDamage',
  'OnTimer',
  'OnEnteredContainer',
  'OnExitedContainer',
  'OnCollision',
  'OnUiMessage',
  'OnSignal',
];

const NODE_GROUPS: Record<string, string[]> = {
  Conditions: ['HasComponent', 'HasTag', 'Compare', 'IsAlive', 'IsInRange', 'HasAccess', 'Chance', 'ContainerContains', 'CooldownReady'],
  Actions: ['AddComponent', 'RemoveComponent', 'SetField', 'SpawnEntity', 'DeleteEntity', 'EmitEvent', 'PlaySound', 'OpenUi', 'ApplyDamage', 'MoveEntity', 'StartTimer', 'SetSpriteState', 'InsertIntoContainer', 'ToggleDoor', 'Log', 'CallScript'],
  Control: ['Branch', 'Sequence', 'Parallel', 'Delay', 'Repeat', 'StateMachine', 'Switch', 'ForEachEntity'],
};

const NODE_DEFAULTS: Record<string, Record<string, unknown>> = {
  HasComponent: { entity: '$self', component: 'Door' },
  HasTag: { entity: '$self', tag: 'example' },
  Compare: { left: '$event.value', operator: '==', right: 0 },
  IsAlive: { entity: '$self' },
  IsInRange: { source: '$event.user', target: '$self', range: 1.5 },
  HasAccess: { target: '$self', user: '$event.user' },
  Chance: { probability: 0.5 },
  ContainerContains: { container: '$self', entity: '$event.item' },
  CooldownReady: { entity: '$self', cooldown: 'default' },
  AddComponent: { entity: '$self', component: 'NewComponent' },
  RemoveComponent: { entity: '$self', component: 'NewComponent' },
  SetField: { entity: '$self', component: 'Component', field: 'value', value: 0 },
  SpawnEntity: { prototype: 'NewEntity', position: '$self.position' },
  DeleteEntity: { entity: '$self' },
  EmitEvent: { event: 'custom-event', payload: {} },
  PlaySound: { target: '$self', sound: '/Audio/example.ogg' },
  OpenUi: { user: '$event.user', ui: 'example.window', target: '$self' },
  ApplyDamage: { target: '$self', amount: 10, damageType: 'blunt' },
  MoveEntity: { entity: '$self', position: '$event.position' },
  StartTimer: { entity: '$self', id: 'timer', duration: 1 },
  SetSpriteState: { entity: '$self', layer: 'base', state: 'default' },
  InsertIntoContainer: { entity: '$event.item', container: '$self' },
  ToggleDoor: { entity: '$self' },
  Log: { level: 'info', message: 'Behavior executed' },
  CallScript: { function: 'gameFunction', arguments: {} },
  Branch: { condition: { node: 'Compare', left: 1, operator: '==', right: 1 }, success: [], failure: [] },
  Sequence: { children: [] },
  Parallel: { children: [] },
  Delay: { duration: 1, children: [] },
  Repeat: { count: 1, maxIterations: 32, children: [] },
  StateMachine: { state: 'off', children: [] },
  Switch: { value: '$event.value', children: [] },
  ForEachEntity: { query: ['Transform'], maxIterations: 128, children: [] },
};

export class BehaviorEditor extends ModelEditor<BehaviorModel> {
  public readonly kind = 'behavior' as const;
  public readonly title: string;

  private selection: NodeSelection;
  private panX = 30;
  private panY = 24;
  private zoom = 1;
  private nodePositions = new Map<string, { x: number; y: number }>();

  public constructor(source: string, path: string) {
    const model = parseBehavior(source);
    super(model);
    this.title = path.split('/').at(-1) ?? 'Behavior graph';
    const event = Object.keys(model.events)[0] ?? 'OnInteract';
    this.selection = { event, nodeId: null, branch: 'root' };
    for (const [id, position] of Object.entries(model.positions)) this.nodePositions.set(id, { ...position });
  }

  public static create(name: string): BehaviorEditor {
    const id = name.replace(/\.[^.]+$/, '').replace(/[^A-Za-z0-9_.-]+/g, '-') || 'new-behavior';
    return new BehaviorEditor(YAML.stringify({
      id,
      events: {
        OnInteract: [
          { node: 'Log', level: 'info', message: 'Interacted' },
        ],
      },
    }), `${name}.hgraph.yml`);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'behavior-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Behavior Graph' }),
      field('Event', selectInput(this.selection.event, Object.keys(this.model.events), (value) => {
        this.selection = { event: value, nodeId: null, branch: 'root' };
        this.render();
      })),
      button('+ Event', () => this.addEvent(), 'tool-button'),
      element('span', { className: 'toolbar-spacer' }),
      button('−', () => { this.zoom = Math.max(0.5, this.zoom / 1.15); this.render(); }, 'icon-button'),
      element('span', { className: 'zoom-label', text: `${Math.round(this.zoom * 100)}%` }),
      button('+', () => { this.zoom = Math.min(2, this.zoom * 1.15); this.render(); }, 'icon-button'),
      button('Auto layout', () => this.autoLayout(), 'tool-button'),
    );

    const body = element('div', { className: 'behavior-editor-body' });
    const palette = this.renderPalette();
    const canvas = this.renderGraphCanvas();
    body.append(palette, canvas);
    shell.append(toolbar, body);
    this.container.append(shell);
    this.renderInspector();
  }

  protected serializeModel(model: BehaviorModel): string {
    const events: Record<string, unknown> = {};
    for (const [eventName, nodes] of Object.entries(model.events)) {
      events[eventName] = nodes.map(stripEditorIds);
    }
    return YAML.stringify({ id: model.id, events, editor: { positions: model.positions } }, { lineWidth: 0, indent: 2 });
  }

  protected parseSource(source: string): BehaviorModel {
    return parseBehavior(source);
  }

  protected validateModel(model: BehaviorModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    if (!model.id.trim()) messages.push({ severity: 'error', message: 'Behavior ID is required.' });
    for (const [eventName, nodes] of Object.entries(model.events)) {
      if (!EVENTS.includes(eventName) && !eventName.startsWith('On')) {
        messages.push({ severity: 'warning', message: `Unknown event: ${eventName}` });
      }
      walkBehaviorNodes(nodes, (node) => {
        if (!node.node) messages.push({ severity: 'error', message: `${eventName} contains a node without a type.` });
        if ((node.node === 'Repeat' || node.node === 'ForEachEntity') && typeof node.maxIterations !== 'number') {
          messages.push({ severity: 'error', message: `${node.node} must define maxIterations to protect the server.` });
        }
      });
    }
    if (messages.length === 0) messages.push({ severity: 'info', message: `Behavior graph is valid: ${Object.keys(model.events).length} events.` });
    return messages;
  }

  private renderPalette(): HTMLElement {
    const palette = element('aside', { className: 'behavior-palette' });
    palette.append(element('h3', { text: 'Nodes' }));
    palette.append(element('p', { className: 'palette-hint', text: 'Click a node to add it to the selected branch. No code is required.' }));
    for (const [groupName, nodes] of Object.entries(NODE_GROUPS)) {
      const group = element('section', { className: 'palette-group' });
      group.append(element('h4', { text: groupName }));
      for (const nodeType of nodes) {
        group.append(button(nodeType, () => this.addNode(nodeType), 'behavior-palette-item'));
      }
      palette.append(group);
    }
    return palette;
  }

  private renderGraphCanvas(): HTMLElement {
    const viewport = element('div', { className: 'behavior-canvas-viewport' });
    const graph = element('div', { className: 'behavior-graph' });
    graph.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;
    graph.style.transformOrigin = 'top left';
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.classList.add('behavior-links');
    graph.append(svg);

    const eventName = this.selection.event;
    const roots = this.model.events[eventName] ?? [];
    const cards: Array<{ node: BehaviorNode; parentId: string | null; branch: BranchSlot; depth: number; order: number }> = [];
    let order = 0;
    const collect = (nodes: BehaviorNode[], parentId: string | null, branch: BranchSlot, depth: number): void => {
      for (const node of nodes) {
        cards.push({ node, parentId, branch, depth, order: order++ });
        for (const slot of nestedSlots(node)) {
          const children = getNestedNodes(node, slot);
          collect(children, node._editorId, slot, depth + 1);
        }
      }
    };
    collect(roots, null, 'root', 0);

    const eventCard = element('button', { className: `behavior-node event-node ${this.selection.nodeId === null ? 'selected' : ''}` });
    eventCard.type = 'button';
    eventCard.style.left = '0px';
    eventCard.style.top = '160px';
    eventCard.append(element('strong', { text: eventName }), element('small', { text: 'Event' }));
    eventCard.addEventListener('click', () => {
      this.selection = { event: eventName, nodeId: null, branch: 'root' };
      this.render();
    });
    graph.append(eventCard);

    const rowByDepth = new Map<number, number>();
    for (const cardData of cards) {
      const row = rowByDepth.get(cardData.depth) ?? 0;
      rowByDepth.set(cardData.depth, row + 1);
      const autoPosition = { x: 250 + cardData.depth * 260, y: 40 + row * 150 };
      const position = this.nodePositions.get(cardData.node._editorId) ?? autoPosition;
      this.nodePositions.set(cardData.node._editorId, position);
      const card = this.renderNodeCard(cardData.node, position, cardData.branch);
      graph.append(card);
    }

    requestAnimationFrame(() => this.drawLinks(svg, graph, cards, eventCard));

    let panning = false;
    let last = { x: 0, y: 0 };
    viewport.addEventListener('pointerdown', (event) => {
      if (event.button === 1 || event.button === 2 || event.target === viewport || event.target === graph || event.target === svg) {
        panning = true;
        last = { x: event.clientX, y: event.clientY };
        viewport.setPointerCapture(event.pointerId);
      }
    });
    viewport.addEventListener('pointermove', (event) => {
      if (!panning) return;
      this.panX += event.clientX - last.x;
      this.panY += event.clientY - last.y;
      last = { x: event.clientX, y: event.clientY };
      graph.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;
    });
    viewport.addEventListener('pointerup', () => { panning = false; });
    viewport.addEventListener('contextmenu', (event) => event.preventDefault());
    viewport.addEventListener('wheel', (event) => {
      event.preventDefault();
      this.zoom = Math.max(0.5, Math.min(2, this.zoom * (event.deltaY < 0 ? 1.1 : 1 / 1.1)));
      graph.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;
    }, { passive: false });

    viewport.append(graph);
    return viewport;
  }

  private renderNodeCard(node: BehaviorNode, position: { x: number; y: number }, branch: BranchSlot): HTMLElement {
    const card = element('button', { className: `behavior-node ${nodeCategory(node.node)} ${node._editorId === this.selection.nodeId ? 'selected' : ''}` });
    card.type = 'button';
    card.style.left = `${position.x}px`;
    card.style.top = `${position.y}px`;
    card.dataset.nodeId = node._editorId;
    card.append(
      element('span', { className: 'behavior-node-branch', text: branch === 'root' ? '' : branch }),
      element('strong', { text: node.node }),
      element('small', { text: nodeSummary(node) }),
    );
    card.addEventListener('click', (event) => {
      event.stopPropagation();
      this.selection = { event: this.selection.event, nodeId: node._editorId, branch: defaultBranch(node) };
      this.render();
    });

    let dragging = false;
    let start = { x: 0, y: 0 };
    card.addEventListener('pointerdown', (event) => {
      if (event.button !== 0) return;
      dragging = true;
      start = { x: event.clientX, y: event.clientY };
      card.setPointerCapture(event.pointerId);
    });
    card.addEventListener('pointermove', (event) => {
      if (!dragging) return;
      const current = this.nodePositions.get(node._editorId) ?? position;
      current.x += (event.clientX - start.x) / this.zoom;
      current.y += (event.clientY - start.y) / this.zoom;
      start = { x: event.clientX, y: event.clientY };
      this.nodePositions.set(node._editorId, current);
      card.style.left = `${current.x}px`;
      card.style.top = `${current.y}px`;
    });
    card.addEventListener('pointerup', () => {
      if (!dragging) return;
      dragging = false;
      const finalPosition = this.nodePositions.get(node._editorId) ?? position;
      this.commit((model) => { model.positions[node._editorId] = { x: Math.round(finalPosition.x), y: Math.round(finalPosition.y) }; }, false);
    });
    card.addEventListener('pointercancel', () => { dragging = false; });
    return card;
  }

  private drawLinks(
    svg: SVGSVGElement,
    graph: HTMLElement,
    cards: Array<{ node: BehaviorNode; parentId: string | null; branch: BranchSlot }>,
    eventCard: HTMLElement,
  ): void {
    svg.replaceChildren();
    const graphRect = graph.getBoundingClientRect();
    const localRect = (elementNode: Element): DOMRect => {
      const rect = elementNode.getBoundingClientRect();
      const scale = this.zoom;
      return new DOMRect((rect.left - graphRect.left) / scale, (rect.top - graphRect.top) / scale, rect.width / scale, rect.height / scale);
    };
    const eventRect = localRect(eventCard);
    for (const cardData of cards) {
      const target = graph.querySelector<HTMLElement>(`[data-node-id="${CSS.escape(cardData.node._editorId)}"]`);
      if (!target) continue;
      const targetRect = localRect(target);
      let sourceRect = eventRect;
      if (cardData.parentId) {
        const source = graph.querySelector<HTMLElement>(`[data-node-id="${CSS.escape(cardData.parentId)}"]`);
        if (!source) continue;
        sourceRect = localRect(source);
      }
      const x1 = sourceRect.right;
      const y1 = sourceRect.top + sourceRect.height / 2;
      const x2 = targetRect.left;
      const y2 = targetRect.top + targetRect.height / 2;
      const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      const control = Math.max(50, (x2 - x1) * 0.5);
      path.setAttribute('d', `M ${x1} ${y1} C ${x1 + control} ${y1}, ${x2 - control} ${y2}, ${x2} ${y2}`);
      path.setAttribute('class', `behavior-link branch-${cardData.branch}`);
      svg.append(path);
    }
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    const selected = this.selection.nodeId ? findBehaviorNode(this.model.events[this.selection.event] ?? [], this.selection.nodeId) : null;
    if (!selected) {
      this.inspector.append(element('h2', { text: 'Behavior Inspector' }));
      this.inspector.append(
        field('Behavior ID', textInput(this.model.id, (value) => this.commit((model) => { model.id = value; }))),
        field('Selected event', selectInput(this.selection.event, Object.keys(this.model.events), (value) => {
          this.selection = { event: value, nodeId: null, branch: 'root' };
          this.render();
        })),
        button('Delete event', () => this.deleteEvent(), 'danger-button'),
      );
      this.inspector.append(element('p', { className: 'inspector-note', text: 'Select a node to edit it. Use the node palette to build server logic visually.' }));
      return;
    }

    this.inspector.append(element('h2', { text: `${selected.node} Inspector` }));
    this.inspector.append(field('Node type', selectInput(selected.node, Object.values(NODE_GROUPS).flat(), (value) => {
      this.commit((model) => {
        const target = findBehaviorNode(model.events[this.selection.event] ?? [], selected._editorId);
        if (!target) return;
        const id = target._editorId;
        for (const key of Object.keys(target)) delete target[key];
        Object.assign(target, { _editorId: id, node: value, ...structuredClone(NODE_DEFAULTS[value] ?? {}) });
      });
    })));

    const properties = element('section', { className: 'inspector-section' });
    properties.append(element('h3', { text: 'Parameters' }));
    for (const [key, value] of Object.entries(selected)) {
      if (key === '_editorId' || key === 'node' || key === 'success' || key === 'failure' || key === 'children' || key === 'condition') continue;
      properties.append(this.propertyEditor(selected, key, value));
    }
    properties.append(button('+ Parameter', () => {
      const key = window.prompt('Parameter name');
      if (key) this.updateSelectedNode((node) => { node[key] = ''; });
    }, 'secondary-button'));
    this.inspector.append(properties);

    const slots = nestedSlots(selected);
    if (slots.length > 0) {
      const branches = element('section', { className: 'inspector-section' });
      branches.append(element('h3', { text: 'Add to branch' }));
      for (const slot of slots) {
        const count = getNestedNodes(selected, slot).length;
        branches.append(button(`${slot} (${count})`, () => {
          this.selection = { event: this.selection.event, nodeId: selected._editorId, branch: slot };
          this.renderInspector();
        }, this.selection.branch === slot ? 'active secondary-button' : 'secondary-button'));
      }
      this.inspector.append(branches);
    }

    this.inspector.append(button('Delete node', () => this.deleteSelectedNode(), 'danger-button'));
  }

  private propertyEditor(node: BehaviorNode, key: string, value: unknown): HTMLElement {
    if (typeof value === 'boolean') {
      return field(key, selectInput(String(value), ['true', 'false'], (next) => this.updateSelectedNode((target) => { target[key] = next === 'true'; })));
    }
    if (typeof value === 'number') {
      return field(key, textInput(String(value), (next) => this.updateSelectedNode((target) => {
        const numeric = Number(next);
        target[key] = Number.isFinite(numeric) ? numeric : next;
      })));
    }
    return field(key, textInput(formatValue(value), (next) => this.updateSelectedNode((target) => { target[key] = parseValue(next); })));
  }

  private addEvent(): void {
    const available = EVENTS.filter((eventName) => !(eventName in this.model.events));
    const eventName = window.prompt(`Event name:\n${available.join(', ')}`, available[0] ?? 'OnSignal');
    if (!eventName) return;
    this.commit((model) => { model.events[eventName] = []; });
    this.selection = { event: eventName, nodeId: null, branch: 'root' };
    this.render();
  }

  private deleteEvent(): void {
    if (Object.keys(this.model.events).length <= 1) return;
    if (!window.confirm(`Delete event ${this.selection.event}?`)) return;
    this.commit((model) => { delete model.events[this.selection.event]; });
    this.selection = { event: Object.keys(this.model.events)[0] ?? 'OnInteract', nodeId: null, branch: 'root' };
    this.render();
  }

  private addNode(type: string): void {
    const newNode: BehaviorNode = {
      _editorId: crypto.randomUUID(),
      node: type,
      ...structuredClone(NODE_DEFAULTS[type] ?? {}),
    };
    ensureNestedIds(newNode);
    this.commit((model) => {
      const roots = model.events[this.selection.event] ?? (model.events[this.selection.event] = []);
      if (!this.selection.nodeId || this.selection.branch === 'root') {
        roots.push(newNode);
        return;
      }
      const parent = findBehaviorNode(roots, this.selection.nodeId);
      if (!parent) {
        roots.push(newNode);
        return;
      }
      const branch = getNestedNodes(parent, this.selection.branch);
      branch.push(newNode);
      parent[this.selection.branch] = branch;
      model.positions[newNode._editorId] = { x: 260, y: 100 + Object.keys(model.positions).length * 24 };
    });
    if (!this.model.positions[newNode._editorId]) this.model.positions[newNode._editorId] = { x: 260, y: 100 };
    this.selection = { event: this.selection.event, nodeId: newNode._editorId, branch: defaultBranch(newNode) };
    this.render();
  }

  private deleteSelectedNode(): void {
    const id = this.selection.nodeId;
    if (!id) return;
    this.commit((model) => {
      const roots = model.events[this.selection.event] ?? [];
      removeBehaviorNode(roots, id);
      delete model.positions[id];
    });
    this.nodePositions.delete(id);
    this.selection = { event: this.selection.event, nodeId: null, branch: 'root' };
    this.render();
  }

  private autoLayout(): void {
    const event = this.selection.event;
    const roots = this.model.events[event] ?? [];
    const positions: Record<string, { x: number; y: number }> = {};
    const rows = new Map<number, number>();
    const visit = (nodes: BehaviorNode[], depth: number): void => {
      for (const node of nodes) {
        const row = rows.get(depth) ?? 0;
        rows.set(depth, row + 1);
        positions[node._editorId] = { x: 250 + depth * 260, y: 40 + row * 150 };
        for (const slot of nestedSlots(node)) visit(getNestedNodes(node, slot), depth + 1);
      }
    };
    visit(roots, 0);
    this.commit((model) => { Object.assign(model.positions, positions); }, false);
    this.nodePositions.clear();
    for (const [id, position] of Object.entries(this.model.positions)) this.nodePositions.set(id, { ...position });
    this.render();
  }

  private updateSelectedNode(mutator: (node: BehaviorNode) => void): void {
    const id = this.selection.nodeId;
    if (!id) return;
    this.commit((model) => {
      const target = findBehaviorNode(model.events[this.selection.event] ?? [], id);
      if (target) mutator(target);
    });
  }
}

function parseBehavior(source: string): BehaviorModel {
  const parsed = YAML.parse(source) as unknown;
  if (!isRecord(parsed) || typeof parsed.id !== 'string' || !isRecord(parsed.events)) {
    throw new Error('Expected a behavior graph with id and events.');
  }
  const events: Record<string, BehaviorNode[]> = {};
  for (const [eventName, rawNodes] of Object.entries(parsed.events)) {
    events[eventName] = Array.isArray(rawNodes) ? rawNodes.filter(isRecord).map(addEditorIds) : [];
  }
  if (Object.keys(events).length === 0) events.OnInteract = [];
  const positions: Record<string, { x: number; y: number }> = {};
  const editor = isRecord(parsed.editor) ? parsed.editor : null;
  if (editor && isRecord(editor.positions)) {
    for (const [id, raw] of Object.entries(editor.positions)) {
      if (isRecord(raw) && typeof raw.x === 'number' && typeof raw.y === 'number') positions[id] = { x: raw.x, y: raw.y };
    }
  }
  return { id: parsed.id, events, positions };
}

function addEditorIds(raw: Record<string, unknown>): BehaviorNode {
  const node: BehaviorNode = { _editorId: typeof raw.editorId === 'string' ? raw.editorId : crypto.randomUUID(), node: String(raw.node ?? 'Log') };
  for (const [key, value] of Object.entries(raw)) {
    if (key === 'node' || key === 'editorId') continue;
    if (['success', 'failure', 'children'].includes(key) && Array.isArray(value)) {
      node[key] = value.filter(isRecord).map(addEditorIds);
    } else if (key === 'condition' && isRecord(value)) {
      node.condition = addEditorIds(value);
    } else {
      node[key] = value;
    }
  }
  return node;
}

function stripEditorIds(node: BehaviorNode): Record<string, unknown> {
  const result: Record<string, unknown> = { node: node.node, editorId: node._editorId };
  for (const [key, value] of Object.entries(node)) {
    if (key === '_editorId' || key === 'node') continue;
    if (['success', 'failure', 'children'].includes(key) && Array.isArray(value)) {
      result[key] = value.map((child) => stripEditorIds(child as BehaviorNode));
    } else if (key === 'condition' && isRecord(value)) {
      const condition = value as unknown as BehaviorNode;
      result.condition = stripEditorIds(condition);
    } else {
      result[key] = value;
    }
  }
  return result;
}

function ensureNestedIds(node: BehaviorNode): void {
  if (isRecord(node.condition) && !('_editorId' in node.condition)) node.condition = addEditorIds(node.condition);
  for (const slot of ['success', 'failure', 'children'] as const) {
    const value = node[slot];
    if (!Array.isArray(value)) continue;
    node[slot] = value.map((child) => {
      if (isRecord(child) && '_editorId' in child) return child;
      return isRecord(child) ? addEditorIds(child) : child;
    });
  }
}

function nestedSlots(node: BehaviorNode): BranchSlot[] {
  if (node.node === 'Branch') return ['success', 'failure'];
  if (['Sequence', 'Parallel', 'Delay', 'Repeat', 'StateMachine', 'Switch', 'ForEachEntity'].includes(node.node)) return ['children'];
  return [];
}

function defaultBranch(node: BehaviorNode): BranchSlot {
  return node.node === 'Branch' ? 'success' : nestedSlots(node).length > 0 ? 'children' : 'root';
}

function getNestedNodes(node: BehaviorNode, slot: BranchSlot): BehaviorNode[] {
  if (slot === 'root') return [];
  const value = node[slot];
  return Array.isArray(value) ? value.filter(isBehaviorNode) : [];
}

function isBehaviorNode(value: unknown): value is BehaviorNode {
  return isRecord(value) && typeof value.node === 'string' && typeof value._editorId === 'string';
}

function findBehaviorNode(nodes: BehaviorNode[], id: string): BehaviorNode | null {
  for (const node of nodes) {
    if (node._editorId === id) return node;
    if (isBehaviorNode(node.condition)) {
      if (node.condition._editorId === id) return node.condition;
    }
    for (const slot of nestedSlots(node)) {
      const found = findBehaviorNode(getNestedNodes(node, slot), id);
      if (found) return found;
    }
  }
  return null;
}

function removeBehaviorNode(nodes: BehaviorNode[], id: string): boolean {
  const index = nodes.findIndex((node) => node._editorId === id);
  if (index >= 0) {
    nodes.splice(index, 1);
    return true;
  }
  for (const node of nodes) {
    for (const slot of nestedSlots(node)) {
      if (removeBehaviorNode(getNestedNodes(node, slot), id)) return true;
    }
  }
  return false;
}

function walkBehaviorNodes(nodes: BehaviorNode[], visitor: (node: BehaviorNode) => void): void {
  for (const node of nodes) {
    visitor(node);
    if (isBehaviorNode(node.condition)) visitor(node.condition);
    for (const slot of nestedSlots(node)) walkBehaviorNodes(getNestedNodes(node, slot), visitor);
  }
}

function nodeCategory(type: string): string {
  if (NODE_GROUPS.Conditions?.includes(type)) return 'condition-node';
  if (NODE_GROUPS.Control?.includes(type)) return 'control-node';
  return 'action-node';
}

function nodeSummary(node: BehaviorNode): string {
  const parts = Object.entries(node)
    .filter(([key, value]) => !['_editorId', 'node', 'success', 'failure', 'children', 'condition'].includes(key) && (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean'))
    .slice(0, 2)
    .map(([key, value]) => `${key}: ${String(value)}`);
  return parts.join(' · ') || 'No parameters';
}

function formatValue(value: unknown): string {
  if (typeof value === 'string') return value;
  return JSON.stringify(value);
}

function parseValue(value: string): unknown {
  const trimmed = value.trim();
  if (trimmed === 'true') return true;
  if (trimmed === 'false') return false;
  const numeric = Number(trimmed);
  if (Number.isFinite(numeric)) return numeric;
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
    try { return JSON.parse(trimmed); } catch { return trimmed; }
  }
  return trimmed;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

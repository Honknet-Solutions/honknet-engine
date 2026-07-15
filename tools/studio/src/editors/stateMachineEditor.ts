import YAML from 'yaml';

import { button, clear, element, field, modal, selectInput, textInput } from '../core/dom';
import type { ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type Transition = {
  event: string;
  target: string;
  guard?: string;
  action?: string;
};

type StateModel = {
  id: string;
  entry: string[];
  exit: string[];
  transitions: Transition[];
  x: number;
  y: number;
};

type StateMachineModel = {
  id: string;
  initial: string;
  states: Record<string, StateModel>;
};

export class StateMachineEditor extends ModelEditor<StateMachineModel> {
  public readonly kind = 'state-machine' as const;
  public readonly title: string;

  private selectedState: string;
  private panX = 40;
  private panY = 40;
  private zoom = 1;

  public constructor(source: string, path: string) {
    const model = parseMachine(source);
    super(model);
    this.title = path.split('/').at(-1) ?? 'State machine';
    this.selectedState = model.initial || Object.keys(model.states)[0] || '';
  }

  public static create(name: string): StateMachineEditor {
    const id = sanitizeId(name);
    const source = YAML.stringify({
      id,
      initial: 'idle',
      states: {
        idle: { entry: [], exit: [], transitions: [{ event: 'activate', target: 'active' }], editor: { x: 80, y: 120 } },
        active: { entry: [], exit: [], transitions: [{ event: 'deactivate', target: 'idle' }], editor: { x: 380, y: 120 } },
      },
    }, { lineWidth: 0 });
    return new StateMachineEditor(source, `${name}.hsm.yml`);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'state-machine-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'State Machine Editor' }),
      button('+ State', () => void this.addState(), 'tool-button'),
      button('+ Transition', () => void this.addTransition(), 'tool-button'),
      button('Delete state', () => this.deleteState(), 'danger-button'),
      element('span', { className: 'toolbar-spacer' }),
      button('−', () => { this.zoom = Math.max(0.4, this.zoom / 1.15); this.render(); }, 'icon-button'),
      element('span', { className: 'zoom-label', text: `${Math.round(this.zoom * 100)}%` }),
      button('+', () => { this.zoom = Math.min(2.5, this.zoom * 1.15); this.render(); }, 'icon-button'),
      button('Auto layout', () => this.autoLayout(), 'tool-button'),
    );

    const viewport = element('div', { className: 'state-machine-viewport' });
    const graph = element('div', { className: 'state-machine-graph' });
    graph.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;
    graph.style.transformOrigin = 'top left';
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.classList.add('state-machine-links');
    graph.append(svg);

    const cards = new Map<string, HTMLElement>();
    for (const [id, state] of Object.entries(this.model.states)) {
      const card = this.renderStateCard(id, state);
      cards.set(id, card);
      graph.append(card);
    }
    requestAnimationFrame(() => this.drawTransitions(svg, graph, cards));

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
    viewport.addEventListener('pointercancel', () => { panning = false; });
    viewport.addEventListener('contextmenu', (event) => event.preventDefault());
    viewport.addEventListener('wheel', (event) => {
      event.preventDefault();
      this.zoom = Math.max(0.4, Math.min(2.5, this.zoom * (event.deltaY < 0 ? 1.1 : 1 / 1.1)));
      graph.style.transform = `translate(${this.panX}px, ${this.panY}px) scale(${this.zoom})`;
    }, { passive: false });

    viewport.append(graph);
    shell.append(toolbar, viewport);
    this.container.append(shell);
    this.renderInspector();
  }

  protected serializeModel(model: StateMachineModel): string {
    const states: Record<string, unknown> = {};
    for (const [id, state] of Object.entries(model.states)) {
      states[id] = {
        entry: state.entry,
        exit: state.exit,
        transitions: state.transitions.map((transition) => compactTransition(transition)),
        editor: { x: Math.round(state.x), y: Math.round(state.y) },
      };
    }
    return YAML.stringify({ id: model.id, initial: model.initial, states }, { lineWidth: 0, indent: 2 });
  }

  protected parseSource(source: string): StateMachineModel {
    return parseMachine(source);
  }

  protected validateModel(model: StateMachineModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    const ids = Object.keys(model.states);
    if (!model.id.trim()) messages.push({ severity: 'error', message: 'State machine ID is required.' });
    if (!model.initial || !model.states[model.initial]) messages.push({ severity: 'error', message: `Initial state ${model.initial || '(empty)'} does not exist.` });
    for (const [stateId, state] of Object.entries(model.states)) {
      for (const transition of state.transitions) {
        if (!transition.event.trim()) messages.push({ severity: 'error', message: `${stateId} contains a transition without an event.` });
        if (!model.states[transition.target]) messages.push({ severity: 'error', message: `${stateId}.${transition.event} targets missing state ${transition.target}.` });
      }
    }
    if (ids.length === 0) messages.push({ severity: 'error', message: 'State machine must contain at least one state.' });
    if (messages.length === 0) messages.push({ severity: 'info', message: `State machine is valid: ${ids.length} states.` });
    return messages;
  }

  private renderStateCard(id: string, state: StateModel): HTMLElement {
    const card = element('button', { className: `state-machine-node ${id === this.selectedState ? 'selected' : ''} ${id === this.model.initial ? 'initial' : ''}` });
    card.type = 'button';
    card.style.left = `${state.x}px`;
    card.style.top = `${state.y}px`;
    card.dataset.stateId = id;
    card.append(
      element('strong', { text: id }),
      element('small', { text: id === this.model.initial ? 'Initial state' : `${state.transitions.length} transitions` }),
    );
    card.addEventListener('click', (event) => {
      event.stopPropagation();
      this.selectedState = id;
      this.render();
    });

    let dragging = false;
    let start = { x: 0, y: 0, left: 0, top: 0 };
    card.addEventListener('pointerdown', (event) => {
      if (event.button !== 0) return;
      event.stopPropagation();
      dragging = true;
      start = { x: event.clientX, y: event.clientY, left: state.x, top: state.y };
      card.setPointerCapture(event.pointerId);
    });
    card.addEventListener('pointermove', (event) => {
      if (!dragging) return;
      const x = start.left + (event.clientX - start.x) / this.zoom;
      const y = start.top + (event.clientY - start.y) / this.zoom;
      card.style.left = `${x}px`;
      card.style.top = `${y}px`;
    });
    const finish = (event: PointerEvent): void => {
      if (!dragging) return;
      dragging = false;
      const x = start.left + (event.clientX - start.x) / this.zoom;
      const y = start.top + (event.clientY - start.y) / this.zoom;
      this.commit((model) => {
        const target = model.states[id];
        if (target) {
          target.x = Math.round(x);
          target.y = Math.round(y);
        }
      });
    };
    card.addEventListener('pointerup', finish);
    card.addEventListener('pointercancel', finish);
    return card;
  }

  private drawTransitions(svg: SVGSVGElement, graph: HTMLElement, cards: Map<string, HTMLElement>): void {
    svg.replaceChildren();
    const graphRect = graph.getBoundingClientRect();
    for (const [sourceId, state] of Object.entries(this.model.states)) {
      const source = cards.get(sourceId);
      if (!source) continue;
      const sourceRect = source.getBoundingClientRect();
      for (const transition of state.transitions) {
        const target = cards.get(transition.target);
        if (!target) continue;
        const targetRect = target.getBoundingClientRect();
        const x1 = (sourceRect.right - graphRect.left) / this.zoom;
        const y1 = (sourceRect.top + sourceRect.height / 2 - graphRect.top) / this.zoom;
        const x2 = (targetRect.left - graphRect.left) / this.zoom;
        const y2 = (targetRect.top + targetRect.height / 2 - graphRect.top) / this.zoom;
        const curve = Math.max(60, Math.abs(x2 - x1) * 0.4);
        const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
        path.setAttribute('d', `M ${x1} ${y1} C ${x1 + curve} ${y1}, ${x2 - curve} ${y2}, ${x2} ${y2}`);
        path.classList.add('state-transition-line');
        svg.append(path);
        const label = document.createElementNS('http://www.w3.org/2000/svg', 'text');
        label.setAttribute('x', String((x1 + x2) / 2));
        label.setAttribute('y', String((y1 + y2) / 2 - 8));
        label.textContent = transition.event;
        label.classList.add('state-transition-label');
        svg.append(label);
      }
    }
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    this.inspector.append(element('h2', { text: 'State Machine Inspector' }));
    this.inspector.append(
      field('Machine ID', textInput(this.model.id, (value) => this.commit((model) => { model.id = value; }))),
      field('Initial state', selectInput(this.model.initial, Object.keys(this.model.states), (value) => this.commit((model) => { model.initial = value; }))),
    );
    const state = this.model.states[this.selectedState];
    if (!state) return;
    const section = element('section', { className: 'inspector-section' });
    section.append(element('h3', { text: `State: ${this.selectedState}` }));
    section.append(
      field('Entry actions', textInput(state.entry.join(', '), (value) => this.updateState((target) => { target.entry = splitCsv(value); }))),
      field('Exit actions', textInput(state.exit.join(', '), (value) => this.updateState((target) => { target.exit = splitCsv(value); }))),
    );
    this.inspector.append(section);

    const transitions = element('section', { className: 'inspector-section' });
    transitions.append(element('h3', { text: 'Transitions' }));
    state.transitions.forEach((transition, index) => {
      const row = element('div', { className: 'transition-inspector-row' });
      row.append(
        element('strong', { text: transition.event }),
        element('span', { text: `→ ${transition.target}` }),
        button('Edit', () => void this.editTransition(index), 'mini-button'),
        button('×', () => this.updateState((target) => { target.transitions.splice(index, 1); }), 'icon-button'),
      );
      transitions.append(row);
    });
    this.inspector.append(transitions);
  }

  private updateState(mutator: (state: StateModel) => void): void {
    const id = this.selectedState;
    this.commit((model) => {
      const state = model.states[id];
      if (state) mutator(state);
    });
  }

  private async addState(): Promise<void> {
    const id = window.prompt('State ID', 'new-state')?.trim();
    if (!id || this.model.states[id]) return;
    this.commit((model) => {
      model.states[id] = { id, entry: [], exit: [], transitions: [], x: 180, y: 180 };
    });
    this.selectedState = id;
    this.render();
  }

  private deleteState(): void {
    const id = this.selectedState;
    if (!id || Object.keys(this.model.states).length <= 1) return;
    if (!window.confirm(`Delete state ${id} and transitions targeting it?`)) return;
    this.commit((model) => {
      delete model.states[id];
      for (const state of Object.values(model.states)) state.transitions = state.transitions.filter((transition) => transition.target !== id);
      if (model.initial === id) model.initial = Object.keys(model.states)[0] ?? '';
    });
    this.selectedState = this.model.initial;
    this.render();
  }

  private async addTransition(): Promise<void> {
    if (!this.selectedState) return;
    const transition = await this.transitionDialog({ event: 'event', target: this.model.initial });
    if (!transition) return;
    this.updateState((state) => { state.transitions.push(transition); });
  }

  private async editTransition(index: number): Promise<void> {
    const state = this.model.states[this.selectedState];
    const current = state?.transitions[index];
    if (!current) return;
    const transition = await this.transitionDialog(current);
    if (!transition) return;
    this.updateState((target) => { target.transitions[index] = transition; });
  }

  private transitionDialog(initial: Transition): Promise<Transition | null> {
    return modal<Transition | null>('Transition', (body, resolve) => {
      let event = initial.event;
      let target = initial.target;
      let guard = initial.guard ?? '';
      let action = initial.action ?? '';
      body.append(
        field('Event', textInput(event, (value) => { event = value; })),
        field('Target state', selectInput(target, Object.keys(this.model.states), (value) => { target = value; })),
        field('Guard', textInput(guard, (value) => { guard = value; })),
        field('Action', textInput(action, (value) => { action = value; })),
      );
      const actions = element('div', { className: 'modal-actions' });
      actions.append(
        button('Cancel', () => resolve(null), 'secondary-button'),
        button('Apply', () => {
          if (!event.trim() || !target) return;
          const result: Transition = { event: event.trim(), target };
          if (guard.trim()) result.guard = guard.trim();
          if (action.trim()) result.action = action.trim();
          resolve(result);
        }, 'primary-button'),
      );
      body.append(actions);
    });
  }

  private autoLayout(): void {
    this.commit((model) => {
      Object.values(model.states).forEach((state, index) => {
        state.x = 80 + (index % 4) * 280;
        state.y = 80 + Math.floor(index / 4) * 180;
      });
    });
  }
}

function parseMachine(source: string): StateMachineModel {
  const parsed = YAML.parse(source) as unknown;
  if (!isRecord(parsed)) throw new Error('State machine document must be an object.');
  const states: Record<string, StateModel> = {};
  if (isRecord(parsed.states)) {
    let index = 0;
    for (const [id, rawState] of Object.entries(parsed.states)) {
      if (!isRecord(rawState)) continue;
      const editor = isRecord(rawState.editor) ? rawState.editor : {};
      const transitions = Array.isArray(rawState.transitions)
        ? rawState.transitions.filter(isRecord).map((transition) => ({
            event: String(transition.event ?? ''),
            target: String(transition.target ?? ''),
            ...(typeof transition.guard === 'string' ? { guard: transition.guard } : {}),
            ...(typeof transition.action === 'string' ? { action: transition.action } : {}),
          }))
        : [];
      states[id] = {
        id,
        entry: Array.isArray(rawState.entry) ? rawState.entry.map(String) : [],
        exit: Array.isArray(rawState.exit) ? rawState.exit.map(String) : [],
        transitions,
        x: typeof editor.x === 'number' ? editor.x : 80 + (index % 4) * 280,
        y: typeof editor.y === 'number' ? editor.y : 80 + Math.floor(index / 4) * 180,
      };
      index += 1;
    }
  }
  return {
    id: typeof parsed.id === 'string' ? parsed.id : 'state-machine',
    initial: typeof parsed.initial === 'string' ? parsed.initial : Object.keys(states)[0] ?? '',
    states,
  };
}

function compactTransition(transition: Transition): Record<string, unknown> {
  const result: Record<string, unknown> = { event: transition.event, target: transition.target };
  if (transition.guard) result.guard = transition.guard;
  if (transition.action) result.action = transition.action;
  return result;
}

function splitCsv(value: string): string[] {
  return value.split(',').map((entry) => entry.trim()).filter(Boolean);
}

function sanitizeId(value: string): string {
  return value.replace(/\.[^.]+$/, '').trim().replace(/[^A-Za-z0-9_.-]+/g, '-') || 'state-machine';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

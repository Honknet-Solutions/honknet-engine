import { clear, element, field, numberInput, selectInput, textInput } from '../core/dom';
import type { RsiMeta, RsiStateSummary, ValidationMessage } from '../core/types';
import { StudioProject } from '../core/project';
import { ModelEditor } from './baseEditor';

type RsiModel = {
  meta: RsiMeta;
};

export class RsiEditor extends ModelEditor<RsiModel> {
  public readonly kind = 'rsi' as const;
  public readonly title: string;

  private selectedState = 0;
  private selectedDirection = 0;
  private frame = 0;
  private imageUrl: string | null = null;
  private image: HTMLImageElement | null = null;
  private animationTimer: number | null = null;
  private readonly states: RsiStateSummary[];

  private constructor(
    private readonly project: StudioProject,
    private readonly rsiPath: string,
    source: string,
    states: RsiStateSummary[],
  ) {
    super({ meta: JSON.parse(source) as RsiMeta });
    this.title = rsiPath.split('/').at(-1) ?? 'RSI';
    this.states = states;
  }

  public static async load(project: StudioProject, path: string): Promise<RsiEditor> {
    const { meta, states } = await project.readRsi(path);
    return new RsiEditor(project, path, JSON.stringify(meta, null, 2), states);
  }

  public override unmount(): void {
    this.stopAnimation();
    this.imageUrl = null;
    this.image = null;
    super.unmount();
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    this.stopAnimation();
    const shell = element('div', { className: 'rsi-editor editor-fill' });
    const statePanel = element('aside', { className: 'rsi-state-list' });
    statePanel.append(element('h3', { text: 'States' }));
    const search = element('input', { attrs: { placeholder: 'Search states…' } });
    const list = element('div', { className: 'rsi-state-items' });
    const drawList = (): void => {
      clear(list);
      const query = search.value.trim().toLowerCase();
      this.states.forEach((state, index) => {
        if (query && !state.name.toLowerCase().includes(query)) return;
        const item = element('button', { className: `rsi-state-item ${index === this.selectedState ? 'selected' : ''}` });
        item.textContent = state.name;
        item.addEventListener('click', () => {
          this.selectedState = index;
          this.selectedDirection = 0;
          this.frame = 0;
          this.render();
        });
        list.append(item);
      });
    };
    search.addEventListener('input', drawList);
    drawList();
    statePanel.append(search, list);

    const preview = element('section', { className: 'rsi-preview-panel' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    const state = this.states[this.selectedState];
    toolbar.append(
      element('span', { className: 'toolbar-title', text: state?.name ?? 'No state' }),
      element('span', { className: 'toolbar-spacer' }),
      element('span', { className: 'toolbar-label', text: 'Direction' }),
      selectInput(String(this.selectedDirection), Array.from({ length: state?.directions ?? 1 }, (_, index) => String(index)), (value) => {
        this.selectedDirection = Number(value);
        this.frame = 0;
        this.render();
      }),
      element('span', { className: 'toolbar-label', text: 'Scale' }),
      selectInput('4×', ['1×', '2×', '4×', '8×'], (value) => {
        const scale = Number(value.replace('×', '')) || 4;
        preview.style.setProperty('--rsi-scale', String(scale));
      }),
    );
    const stage = element('div', { className: 'rsi-preview-stage' });
    const sprite = element('div', { className: 'rsi-preview-sprite' });
    stage.append(sprite);
    preview.append(toolbar, stage);
    shell.append(statePanel, preview);
    this.container.append(shell);
    this.renderInspector();

    if (state) void this.loadAndAnimateState(state, sprite);
  }

  protected serializeModel(model: RsiModel): string {
    return `${JSON.stringify(model.meta, null, 2)}\n`;
  }

  protected parseSource(source: string): RsiModel {
    const meta = JSON.parse(source) as unknown;
    if (!isRecord(meta)) throw new Error('RSI meta.json must contain an object.');
    return { meta: meta as RsiMeta };
  }

  protected validateModel(model: RsiModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    const size = model.meta.size;
    if (!size || !Number.isInteger(size.x) || !Number.isInteger(size.y) || size.x <= 0 || size.y <= 0) {
      messages.push({ severity: 'error', message: 'meta.json must define positive integer size.x and size.y.' });
    }
    const names = new Set<string>();
    for (const state of model.meta.states ?? []) {
      if (!state.name?.trim()) messages.push({ severity: 'error', message: 'RSI contains a state without a name.' });
      if (names.has(state.name)) messages.push({ severity: 'error', message: `Duplicate RSI state: ${state.name}` });
      names.add(state.name);
      if (![1, 4, 8].includes(state.directions ?? 1)) {
        messages.push({ severity: 'warning', message: `${state.name} uses ${state.directions} directions; Honknet normally expects 1, 4 or 8.` });
      }
    }
    if (messages.length === 0) messages.push({ severity: 'info', message: `RSI is valid: ${this.states.length} states.` });
    return messages;
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    const meta = this.model.meta;
    const state = meta.states?.[this.selectedState];
    this.inspector.append(element('h2', { text: 'RSI Inspector' }));
    this.inspector.append(
      field('Frame width', numberInput(meta.size?.x ?? 32, (value) => this.commit((model) => {
        model.meta.size ??= { x: 32, y: 32 };
        model.meta.size.x = Math.max(1, Math.round(value));
      }), { min: 1, max: 4096, step: 1 })),
      field('Frame height', numberInput(meta.size?.y ?? 32, (value) => this.commit((model) => {
        model.meta.size ??= { x: 32, y: 32 };
        model.meta.size.y = Math.max(1, Math.round(value));
      }), { min: 1, max: 4096, step: 1 })),
      field('License', textInput(meta.license ?? '', (value) => this.commit((model) => { model.meta.license = value; }))),
      field('Copyright', textInput(meta.copyright ?? '', (value) => this.commit((model) => { model.meta.copyright = value; }))),
    );
    if (state) {
      const section = element('section', { className: 'inspector-section' });
      section.append(element('h3', { text: `State: ${state.name}` }));
      section.append(
        field('Name', textInput(state.name, (value) => this.commit((model) => {
          const target = model.meta.states?.[this.selectedState];
          if (target) target.name = value;
        })), 'Renaming the metadata state does not rename the PNG file.'),
        field('Directions', selectInput(String(state.directions ?? 1), ['1', '4', '8'], (value) => this.commit((model) => {
          const target = model.meta.states?.[this.selectedState];
          if (target) target.directions = Number(value);
        }))),
      );
      this.inspector.append(section);
    }
    this.inspector.append(element('p', { className: 'inspector-note', text: `${this.rsiPath}/meta.json. Animation preview uses the actual state PNG and delay table.` }));
  }

  private async loadAndAnimateState(state: RsiStateSummary, sprite: HTMLElement): Promise<void> {
    try {
      this.imageUrl = await this.project.getObjectUrl(state.imagePath);
      const image = new Image();
      image.src = this.imageUrl;
      await image.decode();
      this.image = image;
      const width = this.model.meta.size?.x ?? 32;
      const height = this.model.meta.size?.y ?? 32;
      const framesPerRow = Math.max(1, Math.floor(image.naturalWidth / width));
      const delays = state.delays[this.selectedDirection] ?? state.delays[0] ?? [1];
      sprite.style.width = `${width}px`;
      sprite.style.height = `${height}px`;
      sprite.style.backgroundImage = `url(${this.imageUrl})`;
      sprite.style.setProperty('--frame-width', `${width}px`);
      sprite.style.setProperty('--frame-height', `${height}px`);
      const drawFrame = (): void => {
        const frameCount = Math.max(1, delays.length);
        const linear = this.selectedDirection * frameCount + (this.frame % frameCount);
        const column = linear % framesPerRow;
        const row = Math.floor(linear / framesPerRow);
        sprite.style.backgroundPosition = `${-column * width}px ${-row * height}px`;
        const delay = Math.max(0.01, delays[this.frame % frameCount] ?? 0.1) * 1000;
        this.frame = (this.frame + 1) % frameCount;
        this.animationTimer = window.setTimeout(drawFrame, delay);
      };
      drawFrame();
    } catch (error) {
      sprite.textContent = error instanceof Error ? error.message : String(error);
      sprite.classList.add('rsi-preview-error');
    }
  }

  private stopAnimation(): void {
    if (this.animationTimer !== null) window.clearTimeout(this.animationTimer);
    this.animationTimer = null;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

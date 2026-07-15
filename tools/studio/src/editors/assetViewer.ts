import { button, checkboxInput, clear, element, field, numberInput } from '../core/dom';
import type { EditorCommandState, StudioEditor, ValidationMessage } from '../core/types';

export class AssetViewer implements StudioEditor {
  public readonly kind = 'asset' as const;
  public readonly title: string;
  private objectUrl: string | null = null;
  private container: HTMLElement | null = null;
  private inspector: HTMLElement | null = null;
  private zoom = 1;
  private pixelated = true;

  public constructor(private readonly file: File, private readonly path: string) {
    this.title = path.split('/').at(-1) ?? file.name;
  }

  public mount(container: HTMLElement, inspector: HTMLElement): void {
    this.container = container;
    this.inspector = inspector;
    this.render();
  }

  public unmount(): void {
    if (this.objectUrl) URL.revokeObjectURL(this.objectUrl);
    this.objectUrl = null;
    this.container?.replaceChildren();
    this.inspector?.replaceChildren();
    this.container = null;
    this.inspector = null;
  }

  public serialize(): string { return ''; }
  public validate(): ValidationMessage[] { return [{ severity: 'info', message: 'Asset loaded.' }]; }
  public isDirty(): boolean { return false; }
  public markSaved(): void {}
  public undo(): void {}
  public redo(): void {}
  public getCommandState(): EditorCommandState { return { canUndo: false, canRedo: false, dirty: false }; }

  private render(): void {
    if (!this.container || !this.inspector) return;
    clear(this.container);
    clear(this.inspector);
    this.objectUrl ??= URL.createObjectURL(this.file);
    const shell = element('div', { className: 'asset-viewer editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: this.title }),
      element('span', { className: 'toolbar-spacer' }),
      button('−', () => { this.zoom = Math.max(0.1, this.zoom / 1.25); this.render(); }, 'icon-button'),
      element('span', { className: 'zoom-label', text: `${Math.round(this.zoom * 100)}%` }),
      button('+', () => { this.zoom = Math.min(32, this.zoom * 1.25); this.render(); }, 'icon-button'),
      button('100%', () => { this.zoom = 1; this.render(); }, 'tool-button'),
    );
    const stage = element('div', { className: 'asset-stage' });
    shell.append(toolbar, stage);
    this.container.append(shell);

    if (this.file.type.startsWith('image/')) {
      this.renderImage(stage);
    } else if (this.file.type.startsWith('audio/')) {
      void this.renderAudio(stage);
    } else if (/\.(woff2?|ttf|otf)$/i.test(this.file.name)) {
      this.renderFont(stage);
    } else if (this.file.type.includes('json') || this.file.name.toLowerCase().endsWith('.json')) {
      void this.renderJson(stage);
    } else {
      stage.append(element('div', { className: 'asset-placeholder', text: this.file.name }));
    }

    this.inspector.append(
      element('h2', { text: 'Asset Inspector' }),
      field('Path', element('code', { text: this.path })),
      field('MIME type', element('span', { text: this.file.type || 'Unknown' })),
      field('Size', element('span', { text: formatBytes(this.file.size) })),
      field('Last modified', element('span', { text: new Date(this.file.lastModified).toLocaleString() })),
    );
    if (this.file.type.startsWith('image/')) {
      this.inspector.append(
        field('Pixelated', checkboxInput(this.pixelated, (value) => { this.pixelated = value; this.render(); })),
        field('Zoom', numberInput(this.zoom, (value) => { this.zoom = Math.max(0.1, Math.min(32, value)); this.render(); }, { min: 0.1, max: 32, step: 0.1 })),
      );
    }
  }

  private renderImage(stage: HTMLElement): void {
    const image = element('img', { className: 'asset-image' });
    image.src = this.objectUrl ?? '';
    image.alt = this.file.name;
    image.style.transform = `scale(${this.zoom})`;
    image.style.imageRendering = this.pixelated ? 'pixelated' : 'auto';
    image.addEventListener('load', () => {
      if (!this.inspector) return;
      this.inspector.append(
        field('Dimensions', element('span', { text: `${image.naturalWidth}×${image.naturalHeight}` })),
        field('Aspect ratio', element('span', { text: image.naturalHeight ? (image.naturalWidth / image.naturalHeight).toFixed(3) : '—' })),
      );
    });
    stage.append(image);
  }

  private async renderAudio(stage: HTMLElement): Promise<void> {
    const audio = document.createElement('audio');
    audio.controls = true;
    audio.src = this.objectUrl ?? '';
    const canvas = element('canvas', { className: 'audio-waveform' });
    canvas.width = 1200;
    canvas.height = 220;
    stage.append(audio, canvas);
    try {
      const context = new AudioContext();
      const buffer = await context.decodeAudioData(await this.file.arrayBuffer());
      drawWaveform(canvas, buffer);
      await context.close();
      if (this.inspector) {
        this.inspector.append(
          field('Duration', element('span', { text: `${buffer.duration.toFixed(2)} s` })),
          field('Channels', element('span', { text: String(buffer.numberOfChannels) })),
          field('Sample rate', element('span', { text: `${buffer.sampleRate} Hz` })),
        );
      }
    } catch (error) {
      canvas.replaceWith(element('pre', { className: 'source-error', text: error instanceof Error ? error.message : String(error) }));
    }
  }

  private renderFont(stage: HTMLElement): void {
    const family = `honknet-preview-${crypto.randomUUID()}`;
    const style = document.createElement('style');
    style.textContent = `@font-face{font-family:"${family}";src:url("${this.objectUrl}");}`;
    const preview = element('div', { className: 'font-preview' });
    preview.style.fontFamily = family;
    preview.append(
      element('p', { text: 'Honknet Studio — Night City 2045' }),
      element('p', { text: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ' }),
      element('p', { text: 'abcdefghijklmnopqrstuvwxyz 0123456789' }),
      element('p', { text: 'АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ' }),
    );
    stage.append(style, preview);
  }

  private async renderJson(stage: HTMLElement): Promise<void> {
    try {
      const source = await this.file.text();
      const formatted = JSON.stringify(JSON.parse(source), null, 2);
      stage.append(element('pre', { className: 'asset-json-preview', text: formatted }));
    } catch (error) {
      stage.append(element('pre', { className: 'source-error', text: error instanceof Error ? error.message : String(error) }));
    }
  }
}

function drawWaveform(canvas: HTMLCanvasElement, buffer: AudioBuffer): void {
  const context = canvas.getContext('2d');
  if (!context) return;
  const data = buffer.getChannelData(0);
  const step = Math.max(1, Math.floor(data.length / canvas.width));
  const middle = canvas.height / 2;
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.strokeStyle = '#65d7ff';
  context.lineWidth = 1;
  context.beginPath();
  for (let x = 0; x < canvas.width; x += 1) {
    let min = 1;
    let max = -1;
    const start = x * step;
    for (let index = start; index < Math.min(data.length, start + step); index += 1) {
      const sample = data[index] ?? 0;
      min = Math.min(min, sample);
      max = Math.max(max, sample);
    }
    context.moveTo(x, middle + min * middle * 0.9);
    context.lineTo(x, middle + max * middle * 0.9);
  }
  context.stroke();
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

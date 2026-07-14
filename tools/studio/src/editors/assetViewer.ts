import { element } from '../core/dom';
import type { EditorCommandState, StudioEditor, ValidationMessage } from '../core/types';

export class AssetViewer implements StudioEditor {
  public readonly kind = 'asset' as const;
  public readonly title: string;
  private objectUrl: string | null = null;

  public constructor(private readonly file: File, path: string) {
    this.title = path.split('/').at(-1) ?? file.name;
  }

  public mount(container: HTMLElement, inspector: HTMLElement): void {
    container.replaceChildren();
    inspector.replaceChildren();
    this.objectUrl = URL.createObjectURL(this.file);
    const shell = element('div', { className: 'asset-viewer' });
    if (this.file.type.startsWith('image/')) {
      const image = element('img');
      image.src = this.objectUrl;
      image.alt = this.file.name;
      shell.append(image);
    } else if (this.file.type.startsWith('audio/')) {
      const audio = document.createElement('audio');
      audio.controls = true;
      audio.src = this.objectUrl;
      shell.append(audio);
    } else {
      shell.append(element('div', { className: 'asset-placeholder', text: this.file.name }));
    }
    container.append(shell);
    inspector.append(
      element('h2', { text: 'Asset Inspector' }),
      element('p', { text: this.file.name }),
      element('p', { text: this.file.type || 'Unknown MIME type' }),
      element('p', { text: `${Math.round(this.file.size / 1024)} KB` }),
    );
  }

  public unmount(): void {
    if (this.objectUrl) URL.revokeObjectURL(this.objectUrl);
    this.objectUrl = null;
  }

  public serialize(): string { return ''; }
  public validate(): ValidationMessage[] { return [{ severity: 'info', message: 'Asset loaded.' }]; }
  public isDirty(): boolean { return false; }
  public markSaved(): void {}
  public undo(): void {}
  public redo(): void {}
  public getCommandState(): EditorCommandState { return { canUndo: false, canRedo: false, dirty: false }; }
}

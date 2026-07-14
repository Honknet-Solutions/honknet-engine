import { clear, element } from '../core/dom';
import type { StudioFileKind, ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

export class TextEditor extends ModelEditor<string> {
  public readonly kind: StudioFileKind;
  public readonly title: string;

  public constructor(source: string, path: string, kind: StudioFileKind = 'unknown') {
    super(source);
    this.kind = kind;
    this.title = path.split('/').at(-1) ?? 'Text';
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const textarea = element('textarea', { className: 'plain-text-editor' });
    textarea.spellcheck = false;
    textarea.value = this.model;
    textarea.addEventListener('input', () => {
      const value = textarea.value;
      this.commit(() => { this.model = value; }, false);
    });
    this.container.append(textarea);
    clear(this.inspector);
    this.inspector.append(
      element('h2', { text: 'Text file' }),
      element('p', { className: 'inspector-note', text: 'This file has no visual editor. It remains available for engine developers in Developer Mode.' }),
    );
  }

  protected serializeModel(model: string): string {
    return model;
  }

  protected parseSource(source: string): string {
    return source;
  }

  protected validateModel(): ValidationMessage[] {
    return [{ severity: 'info', message: 'Text file loaded.' }];
  }
}

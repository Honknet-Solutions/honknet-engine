import type {
  EditorCommandState,
  StudioEditor,
  StudioFileKind,
  ValidationMessage,
} from '../core/types';
import { deepClone } from '../core/dom';

export abstract class ModelEditor<TModel> implements StudioEditor {
  public abstract readonly kind: StudioFileKind;
  public abstract readonly title: string;

  protected model: TModel;
  protected container: HTMLElement | null = null;
  protected inspector: HTMLElement | null = null;

  private savedSnapshot: string;
  private readonly undoStack: TModel[] = [];
  private readonly redoStack: TModel[] = [];
  private sourceEnabled = false;

  protected constructor(initialModel: TModel) {
    this.model = initialModel;
    this.savedSnapshot = this.serializeModel(initialModel);
  }

  public mount(container: HTMLElement, inspector: HTMLElement): void {
    this.container = container;
    this.inspector = inspector;
    this.render();
  }

  public unmount(): void {
    this.container?.replaceChildren();
    this.inspector?.replaceChildren();
    this.container = null;
    this.inspector = null;
  }

  public serialize(): string {
    return this.serializeModel(this.model);
  }

  public validate(): ValidationMessage[] {
    return this.validateModel(this.model);
  }

  public isDirty(): boolean {
    return this.serialize() !== this.savedSnapshot;
  }

  public markSaved(): void {
    this.savedSnapshot = this.serialize();
    this.emitStateChanged();
  }

  public undo(): void {
    const previous = this.undoStack.pop();
    if (!previous) return;
    this.redoStack.push(deepClone(this.model));
    this.model = previous;
    this.render();
    this.emitStateChanged();
  }

  public redo(): void {
    const next = this.redoStack.pop();
    if (!next) return;
    this.undoStack.push(deepClone(this.model));
    this.model = next;
    this.render();
    this.emitStateChanged();
  }

  public getCommandState(): EditorCommandState {
    return {
      canUndo: this.undoStack.length > 0,
      canRedo: this.redoStack.length > 0,
      dirty: this.isDirty(),
    };
  }

  public showSource(enabled: boolean): void {
    this.sourceEnabled = enabled;
    this.render();
  }

  protected get isSourceEnabled(): boolean {
    return this.sourceEnabled;
  }

  protected commit(mutator: (model: TModel) => void, render = true): void {
    this.undoStack.push(deepClone(this.model));
    if (this.undoStack.length > 100) this.undoStack.shift();
    this.redoStack.length = 0;
    mutator(this.model);
    if (render) this.render();
    this.emitStateChanged();
  }

  protected replaceModel(next: TModel): void {
    this.undoStack.push(deepClone(this.model));
    this.redoStack.length = 0;
    this.model = next;
    this.render();
    this.emitStateChanged();
  }

  protected abstract renderDesigner(): void;
  protected abstract serializeModel(model: TModel): string;
  protected abstract parseSource(source: string): TModel;
  protected abstract validateModel(model: TModel): ValidationMessage[];

  protected render(): void {
    if (!this.container || !this.inspector) return;
    this.container.replaceChildren();
    this.inspector.replaceChildren();

    if (this.sourceEnabled) {
      this.renderSourceEditor();
    } else {
      this.renderDesigner();
    }
  }

  protected emitStateChanged(): void {
    window.dispatchEvent(new CustomEvent('honknet-editor-state-changed'));
  }

  private renderSourceEditor(): void {
    if (!this.container || !this.inspector) return;
    const wrapper = document.createElement('div');
    wrapper.className = 'source-editor-shell';
    const textarea = document.createElement('textarea');
    textarea.className = 'source-editor';
    textarea.spellcheck = false;
    textarea.value = this.serialize();
    const error = document.createElement('pre');
    error.className = 'source-error';

    let timeout: number | null = null;
    textarea.addEventListener('input', () => {
      if (timeout !== null) window.clearTimeout(timeout);
      timeout = window.setTimeout(() => {
        try {
          const next = this.parseSource(textarea.value);
          error.textContent = '';
          this.replaceModel(next);
          this.sourceEnabled = true;
          if (this.container) {
            this.container.replaceChildren(wrapper);
          }
        } catch (parseError) {
          error.textContent = parseError instanceof Error ? parseError.message : String(parseError);
        }
      }, 250);
    });

    wrapper.append(textarea, error);
    this.container.append(wrapper);
    this.inspector.append(document.createTextNode('Source mode: edit YAML/FTL directly. Designer mode remains the default.'));
  }
}

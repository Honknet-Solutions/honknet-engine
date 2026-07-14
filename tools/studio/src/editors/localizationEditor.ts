import {
  button,
  clear,
  element,
  field,
  textInput,
} from '../core/dom';
import type { ValidationMessage } from '../core/types';
import { ModelEditor } from './baseEditor';

type LocalizationEntry = {
  key: string;
  value: string;
  comment: string;
};

type LocalizationModel = {
  entries: LocalizationEntry[];
};

export class LocalizationEditor extends ModelEditor<LocalizationModel> {
  public readonly kind = 'localization' as const;
  public readonly title: string;
  private selectedIndex: number | null = null;
  private search = '';

  public constructor(source: string, path: string) {
    super(parseFtl(source));
    this.title = path.split('/').at(-1) ?? 'Localization';
  }

  public static create(name: string): LocalizationEditor {
    return new LocalizationEditor('new-key = New text\n', `${name}.ftl`);
  }

  protected renderDesigner(): void {
    if (!this.container || !this.inspector) return;
    const shell = element('div', { className: 'localization-editor editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    const search = element('input', { className: 'toolbar-search', attrs: { placeholder: 'Search keys or text…' } });
    search.value = this.search;
    search.addEventListener('input', () => {
      this.search = search.value;
      this.render();
    });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Localization Editor' }),
      search,
      button('+ Entry', () => this.addEntry(), 'tool-button'),
    );
    const table = element('div', { className: 'localization-table' });
    const header = element('div', { className: 'localization-row header' });
    header.append(element('strong', { text: 'Key' }), element('strong', { text: 'Translation' }));
    table.append(header);
    const query = this.search.trim().toLowerCase();
    for (const [index, entry] of this.model.entries.entries()) {
      if (query && !entry.key.toLowerCase().includes(query) && !entry.value.toLowerCase().includes(query)) continue;
      const row = element('button', { className: `localization-row ${index === this.selectedIndex ? 'selected' : ''}` });
      row.type = 'button';
      row.append(element('code', { text: entry.key }), element('span', { text: entry.value }));
      row.addEventListener('click', () => {
        this.selectedIndex = index;
        this.render();
      });
      table.append(row);
    }
    shell.append(toolbar, table);
    this.container.append(shell);
    this.renderInspector();
  }

  protected serializeModel(model: LocalizationModel): string {
    return model.entries.map((entry) => {
      const comment = entry.comment ? `${entry.comment.split(/\r?\n/).map((line) => `# ${line}`).join('\n')}\n` : '';
      const lines = entry.value.split(/\r?\n/);
      if (lines.length === 1) return `${comment}${entry.key} = ${lines[0] ?? ''}`;
      return `${comment}${entry.key} = ${lines[0] ?? ''}\n${lines.slice(1).map((line) => `    ${line}`).join('\n')}`;
    }).join('\n\n') + '\n';
  }

  protected parseSource(source: string): LocalizationModel {
    return parseFtl(source);
  }

  protected validateModel(model: LocalizationModel): ValidationMessage[] {
    const messages: ValidationMessage[] = [];
    const keys = new Set<string>();
    for (const entry of model.entries) {
      if (!/^[A-Za-z0-9_.-]+$/.test(entry.key)) messages.push({ severity: 'error', message: `Invalid FTL key: ${entry.key}` });
      if (keys.has(entry.key)) messages.push({ severity: 'error', message: `Duplicate FTL key: ${entry.key}` });
      keys.add(entry.key);
      if (!entry.value.trim()) messages.push({ severity: 'warning', message: `${entry.key} has an empty translation.` });
    }
    if (messages.length === 0) messages.push({ severity: 'info', message: `Localization is valid: ${model.entries.length} messages.` });
    return messages;
  }

  private renderInspector(): void {
    if (!this.inspector) return;
    clear(this.inspector);
    this.inspector.append(element('h2', { text: 'Localization Inspector' }));
    const entry = this.selectedIndex === null ? undefined : this.model.entries[this.selectedIndex];
    if (!entry) {
      this.inspector.append(element('p', { className: 'empty-state', text: 'Select a message to edit it. Designers can work entirely in this table without touching FTL syntax.' }));
      return;
    }
    const valueArea = element('textarea', { className: 'inspector-textarea' });
    valueArea.value = entry.value;
    valueArea.addEventListener('input', () => this.updateEntry((target) => { target.value = valueArea.value; }, false));
    const commentArea = element('textarea', { className: 'inspector-textarea small' });
    commentArea.value = entry.comment;
    commentArea.addEventListener('input', () => this.updateEntry((target) => { target.comment = commentArea.value; }, false));
    this.inspector.append(
      field('Key', textInput(entry.key, (value) => this.updateEntry((target) => { target.key = value; }))),
      field('Translation', valueArea),
      field('Translator comment', commentArea),
      button('Delete message', () => this.deleteEntry(), 'danger-button'),
    );
  }

  private addEntry(): void {
    let index = 1;
    let key = 'new-key';
    while (this.model.entries.some((entry) => entry.key === key)) key = `new-key-${index++}`;
    this.commit((model) => model.entries.push({ key, value: 'New text', comment: '' }));
    this.selectedIndex = this.model.entries.length - 1;
    this.render();
  }

  private deleteEntry(): void {
    const index = this.selectedIndex;
    if (index === null) return;
    this.commit((model) => model.entries.splice(index, 1));
    this.selectedIndex = null;
    this.render();
  }

  private updateEntry(mutator: (entry: LocalizationEntry) => void, render = true): void {
    this.commit((model) => {
      const target = model.entries[this.selectedIndex ?? -1];
      if (target) mutator(target);
    }, render);
  }
}

function parseFtl(source: string): LocalizationModel {
  const entries: LocalizationEntry[] = [];
  let pendingComments: string[] = [];
  let current: LocalizationEntry | null = null;
  for (const line of source.split(/\r?\n/)) {
    const comment = /^\s*#\s?(.*)$/.exec(line);
    if (comment) {
      pendingComments.push(comment[1] ?? '');
      continue;
    }
    const match = /^\s*([A-Za-z0-9_.-]+)\s*=\s?(.*)$/.exec(line);
    if (match?.[1] !== undefined) {
      current = { key: match[1], value: match[2] ?? '', comment: pendingComments.join('\n') };
      entries.push(current);
      pendingComments = [];
      continue;
    }
    const continuation = /^\s{2,}(.*)$/.exec(line);
    if (continuation && current) current.value += `\n${continuation[1] ?? ''}`;
    else if (line.trim()) pendingComments = [];
  }
  return { entries };
}

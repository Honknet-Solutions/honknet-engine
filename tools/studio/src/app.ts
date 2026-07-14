import { button, clear, element, field, modal, numberInput, required, selectInput, textInput, toast } from './core/dom';
import { detectFileKind, joinPath } from './core/fileSystem';
import { StudioProject } from './core/project';
import type {
  CreateDocumentRequest,
  ProjectTreeNode,
  StudioEditor,
  StudioFileKind,
  ValidationMessage,
} from './core/types';
import { AssetViewer } from './editors/assetViewer';
import { BehaviorEditor } from './editors/behaviorEditor';
import { LocalizationEditor } from './editors/localizationEditor';
import { MapEditor } from './editors/mapEditor';
import { PrototypeEditor } from './editors/prototypeEditor';
import { SchemaEditor } from './editors/schemaEditor';
import { TextEditor } from './editors/textEditor';
import { UiEditor } from './editors/uiEditor';

const DEFAULT_DIRECTORIES: Record<CreateDocumentRequest['kind'], string> = {
  map: 'game/example-module/maps',
  hui: 'game/example-module/content/ui',
  prototype: 'game/example-module/content/prototypes',
  'component-schema': 'game/example-module/content/component-schemas',
  behavior: 'game/example-module/content/behaviors',
  localization: 'game/example-module/localization/ru-RU',
};

const EXTENSIONS: Record<CreateDocumentRequest['kind'], string> = {
  map: '.yml',
  hui: '.hui.yml',
  prototype: '.yml',
  'component-schema': '.yml',
  behavior: '.hgraph.yml',
  localization: '.ftl',
};

export class HonknetStudioApp {
  private readonly project = new StudioProject();
  private editor: StudioEditor | null = null;
  private currentPath: string | null = null;
  private sourceMode = false;
  private selectedTreePath: string | null = null;
  private treeFilter = '';
  private readonly expandedDirectories = new Set<string>(['', 'game', 'game/example-module', 'game/example-module/content']);
  private validationMessages: ValidationMessage[] = [];
  private consoleLines: string[] = ['Honknet Studio ready. Open the engine project folder.'];

  private readonly projectName: HTMLElement;
  private readonly currentPathElement: HTMLElement;
  private readonly treeHost: HTMLElement;
  private readonly editorHost: HTMLElement;
  private readonly inspectorHost: HTMLElement;
  private readonly validationHost: HTMLElement;
  private readonly consoleHost: HTMLElement;
  private readonly welcomeHost: HTMLElement;
  private readonly undoButton: HTMLButtonElement;
  private readonly redoButton: HTMLButtonElement;
  private readonly saveButton: HTMLButtonElement;
  private readonly sourceButton: HTMLButtonElement;

  public constructor(private readonly root: HTMLElement) {
    root.innerHTML = this.template();
    this.projectName = required('#project-name', root);
    this.currentPathElement = required('#current-path', root);
    this.treeHost = required('#project-tree', root);
    this.editorHost = required('#editor-host', root);
    this.inspectorHost = required('#inspector-host', root);
    this.validationHost = required('#validation-list', root);
    this.consoleHost = required('#console-list', root);
    this.welcomeHost = required('#welcome', root);
    this.undoButton = required('#undo', root);
    this.redoButton = required('#redo', root);
    this.saveButton = required('#save', root);
    this.sourceButton = required('#source-mode', root);

    this.bindEvents();
    this.renderProjectTree();
    this.renderBottomPanels();
    this.updateToolbar();
  }

  private template(): string {
    return `
      <div class="studio-shell">
        <header class="studio-header">
          <div class="brand">
            <p>HONKNET SOLUTIONS</p>
            <h1>Honknet Studio</h1>
          </div>
          <nav class="main-actions">
            <button id="open-project" class="primary-button">Open Project</button>
            <button id="new-document">New</button>
            <button id="save">Save</button>
            <span class="toolbar-divider"></span>
            <button id="undo" title="Undo (Ctrl+Z)">Undo</button>
            <button id="redo" title="Redo (Ctrl+Y)">Redo</button>
            <button id="source-mode" title="Toggle Source/Designer mode">Source</button>
            <button id="validate">Validate</button>
          </nav>
        </header>

        <div class="project-bar">
          <strong id="project-name">No project</strong>
          <span id="current-path">Open your Space Station 15 folder to begin.</span>
          <span id="dirty-indicator"></span>
        </div>

        <main class="studio-main">
          <aside class="project-panel">
            <div class="panel-heading">
              <h2>Project</h2>
              <div class="mini-actions">
                <button id="new-folder" class="icon-button" title="New folder">＋</button>
                <button id="refresh-project" class="icon-button" title="Refresh project">↻</button>
              </div>
            </div>
            <input id="tree-search" class="tree-search" placeholder="Search files…" />
            <div id="project-tree" class="project-tree"></div>
            <div class="tree-actions">
              <button id="rename-entry">Rename</button>
              <button id="delete-entry" class="danger-link">Delete</button>
            </div>
          </aside>

          <section class="editor-area">
            <div id="welcome" class="welcome-screen">
              <div class="welcome-card">
                <p class="eyebrow">DESIGNER WORKFLOW</p>
                <h2>Build content without writing YAML</h2>
                <p>Open the project folder. Map designers paint tiles and place entities. UI designers drag controls onto a live canvas. YAML, HUI, HGRAPH and FTL remain internal formats.</p>
                <button id="welcome-open" class="primary-button">Open Project Folder</button>
                <div class="welcome-features">
                  <span>Project Tree</span><span>Map Editor</span><span>UI Designer</span><span>Prototype Studio</span><span>Behavior Graph</span><span>Localization</span>
                </div>
              </div>
            </div>
            <div id="editor-host" class="editor-host"></div>
          </section>

          <aside id="inspector-host" class="inspector-panel">
            <h2>Inspector</h2>
            <p class="empty-state">Select an object in an editor.</p>
          </aside>
        </main>

        <section class="bottom-panel">
          <div class="bottom-tabs">
            <button data-bottom-tab="validation" class="active">Validation</button>
            <button data-bottom-tab="console">Console</button>
          </div>
          <div id="validation-list" class="bottom-content active"></div>
          <div id="console-list" class="bottom-content"></div>
        </section>
      </div>
    `;
  }

  private bindEvents(): void {
    required<HTMLButtonElement>('#open-project', this.root).addEventListener('click', () => void this.openProject());
    required<HTMLButtonElement>('#welcome-open', this.root).addEventListener('click', () => void this.openProject());
    required<HTMLButtonElement>('#new-document', this.root).addEventListener('click', () => void this.createDocument());
    required<HTMLButtonElement>('#new-folder', this.root).addEventListener('click', () => void this.createFolder());
    required<HTMLButtonElement>('#refresh-project', this.root).addEventListener('click', () => void this.refreshProject());
    required<HTMLButtonElement>('#rename-entry', this.root).addEventListener('click', () => void this.renameSelectedEntry());
    required<HTMLButtonElement>('#delete-entry', this.root).addEventListener('click', () => void this.deleteSelectedEntry());
    this.saveButton.addEventListener('click', () => void this.saveCurrent());
    this.undoButton.addEventListener('click', () => this.editor?.undo());
    this.redoButton.addEventListener('click', () => this.editor?.redo());
    this.sourceButton.addEventListener('click', () => {
      if (!this.editor?.showSource) return;
      this.sourceMode = !this.sourceMode;
      this.editor.showSource(this.sourceMode);
      this.updateToolbar();
    });
    required<HTMLButtonElement>('#validate', this.root).addEventListener('click', () => this.validateCurrent());

    const search = required<HTMLInputElement>('#tree-search', this.root);
    search.addEventListener('input', () => {
      this.treeFilter = search.value;
      this.renderProjectTree();
    });

    for (const tab of this.root.querySelectorAll<HTMLButtonElement>('[data-bottom-tab]')) {
      tab.addEventListener('click', () => this.switchBottomTab(tab.dataset.bottomTab ?? 'validation'));
    }

    window.addEventListener('honknet-editor-state-changed', () => this.updateToolbar());
    window.addEventListener('keydown', (event) => {
      const modifier = event.ctrlKey || event.metaKey;
      if (modifier && event.key.toLowerCase() === 's') {
        event.preventDefault();
        void this.saveCurrent();
      } else if (modifier && event.key.toLowerCase() === 'z' && !event.shiftKey) {
        event.preventDefault();
        this.editor?.undo();
      } else if (modifier && (event.key.toLowerCase() === 'y' || (event.key.toLowerCase() === 'z' && event.shiftKey))) {
        event.preventDefault();
        this.editor?.redo();
      }
    });
    window.addEventListener('beforeunload', (event) => {
      if (this.editor?.isDirty()) {
        event.preventDefault();
        event.returnValue = '';
      }
    });
  }

  private async openProject(): Promise<void> {
    try {
      if (!(await this.confirmDiscard())) return;
      await this.project.open();
      this.projectName.textContent = this.project.name;
      this.welcomeHost.classList.add('hidden');
      this.log(`Opened project: ${this.project.name}`);
      this.renderProjectTree();
      toast(`Project opened: ${this.project.name}`, 'success');
    } catch (error) {
      if (isAbort(error)) return;
      this.handleError('Failed to open project', error);
    }
  }

  private async refreshProject(): Promise<void> {
    if (!this.project.isOpen) return;
    try {
      await this.project.refresh();
      this.renderProjectTree();
      this.log('Project tree refreshed.');
    } catch (error) {
      this.handleError('Failed to refresh project', error);
    }
  }

  private renderProjectTree(): void {
    clear(this.treeHost);
    const tree = this.project.projectTree;
    if (!tree) {
      this.treeHost.append(element('p', { className: 'empty-state', text: 'No project opened.' }));
      return;
    }
    const query = this.treeFilter.trim().toLowerCase();
    for (const child of tree.children) this.renderTreeNode(child, this.treeHost, 0, query);
  }

  private renderTreeNode(node: ProjectTreeNode, host: HTMLElement, depth: number, query: string): void {
    if (query && !treeContains(node, query)) return;
    const row = element('button', { className: `tree-row ${node.path === this.selectedTreePath ? 'selected' : ''}` });
    row.type = 'button';
    row.style.paddingLeft = `${8 + depth * 14}px`;
    const expanded = this.expandedDirectories.has(node.path) || Boolean(query);
    const disclosure = node.kind === 'directory' ? (expanded ? '▾' : '▸') : '';
    row.append(
      element('span', { className: 'tree-disclosure', text: disclosure }),
      element('span', { className: `file-icon kind-${node.fileKind}`, text: fileGlyph(node) }),
      element('span', { className: 'tree-name', text: node.name }),
    );
    row.addEventListener('click', () => {
      this.selectedTreePath = node.path;
      if (node.kind === 'directory') {
        if (this.expandedDirectories.has(node.path)) this.expandedDirectories.delete(node.path);
        else this.expandedDirectories.add(node.path);
        this.renderProjectTree();
      } else {
        void this.openFile(node.path, node.fileKind);
      }
    });
    host.append(row);
    if (node.kind === 'directory' && expanded) {
      for (const child of node.children) this.renderTreeNode(child, host, depth + 1, query);
    }
  }

  private async openFile(path: string, kind = detectFileKind(path)): Promise<void> {
    if (!(await this.confirmDiscard())) return;
    try {
      this.editor?.unmount();
      this.editor = null;
      this.currentPath = path;
      this.currentPathElement.textContent = path;
      this.sourceMode = false;

      if (kind === 'asset') {
        const file = await this.project.files.readFile(path);
        this.editor = new AssetViewer(file, path);
      } else {
        const source = await this.project.readText(path);
        this.editor = this.createEditor(kind, source, path);
      }

      this.editor.mount(this.editorHost, this.inspectorHost);
      this.validationMessages = this.editor.validate();
      this.renderBottomPanels();
      this.updateToolbar();
      this.log(`Opened ${path}`);
    } catch (error) {
      this.handleError(`Failed to open ${path}`, error);
    }
  }

  private createEditor(kind: StudioFileKind, source: string, path: string): StudioEditor {
    switch (kind) {
      case 'map': return new MapEditor(source, path, this.project.projectMetadata);
      case 'hui': return new UiEditor(source, path, this.project.projectMetadata);
      case 'prototype': return new PrototypeEditor(source, path, this.project.projectMetadata);
      case 'component-schema': return new SchemaEditor(source, path);
      case 'behavior': return new BehaviorEditor(source, path);
      case 'localization': return new LocalizationEditor(source, path);
      default: return new TextEditor(source, path, kind);
    }
  }

  private async saveCurrent(): Promise<void> {
    if (!this.editor || !this.currentPath || this.editor.kind === 'asset') return;
    try {
      this.validationMessages = this.editor.validate();
      this.renderBottomPanels();
      if (this.validationMessages.some((message) => message.severity === 'error')) {
        this.switchBottomTab('validation');
        toast('Fix validation errors before saving.', 'error');
        return;
      }
      await this.project.writeText(this.currentPath, this.editor.serialize());
      this.editor.markSaved();
      this.log(`Saved ${this.currentPath}`);
      toast('Saved', 'success');
      this.updateToolbar();
      await this.project.refresh();
      this.renderProjectTree();
    } catch (error) {
      this.handleError('Failed to save file', error);
    }
  }

  private validateCurrent(): void {
    if (!this.editor) return;
    this.validationMessages = this.editor.validate();
    this.renderBottomPanels();
    this.switchBottomTab('validation');
    const errors = this.validationMessages.filter((message) => message.severity === 'error').length;
    toast(errors === 0 ? 'Validation passed.' : `${errors} validation errors.`, errors === 0 ? 'success' : 'error');
  }

  private async createDocument(): Promise<void> {
    if (!this.project.isOpen) {
      toast('Open a project first.', 'error');
      return;
    }
    try {
      const request = await this.createDocumentDialog();
      const extension = EXTENSIONS[request.kind];
      const rawName = request.name.trim();
      const fileName = rawName.toLowerCase().endsWith(extension) ? rawName : `${rawName}${extension}`;
      const path = joinPath(request.directory, fileName);
      let editor: StudioEditor;
      switch (request.kind) {
        case 'map': editor = MapEditor.create(rawName, request.width ?? 32, request.height ?? 32, this.project.projectMetadata); break;
        case 'hui': editor = UiEditor.create(rawName, this.project.projectMetadata); break;
        case 'prototype': editor = PrototypeEditor.create(rawName, this.project.projectMetadata); break;
        case 'component-schema': editor = SchemaEditor.create(rawName); break;
        case 'behavior': editor = BehaviorEditor.create(rawName); break;
        case 'localization': editor = LocalizationEditor.create(rawName); break;
      }
      await this.project.writeText(path, editor.serialize());
      await this.project.refresh();
      this.expandedDirectories.add(request.directory);
      this.renderProjectTree();
      await this.openFile(path, request.kind);
      toast(`Created ${path}`, 'success');
    } catch (error) {
      if (isAbort(error)) return;
      this.handleError('Failed to create document', error);
    }
  }

  private createDocumentDialog(): Promise<CreateDocumentRequest> {
    return modal<CreateDocumentRequest>('Create content', (body, resolve, reject) => {
      let kind: CreateDocumentRequest['kind'] = 'map';
      let name = 'new-map';
      let directory = DEFAULT_DIRECTORIES[kind];
      let width = 32;
      let height = 32;
      const kindSelect = selectInput(kind, ['map', 'hui', 'prototype', 'component-schema', 'behavior', 'localization'], (value) => {
        kind = value as CreateDocumentRequest['kind'];
        directory = DEFAULT_DIRECTORIES[kind];
        directoryInput.value = directory;
        sizeFields.hidden = kind !== 'map';
      });
      const nameInput = textInput(name, (value) => { name = value; });
      const directoryInput = textInput(directory, (value) => { directory = value; });
      const sizeFields = element('div', { className: 'modal-grid' });
      sizeFields.append(
        field('Width', numberInput(width, (value) => { width = Math.round(value); }, { min: 1, max: 4096 })),
        field('Height', numberInput(height, (value) => { height = Math.round(value); }, { min: 1, max: 4096 })),
      );
      const actions = element('div', { className: 'modal-actions' });
      actions.append(button('Cancel', reject, 'secondary-button'), button('Create', () => {
        if (!name.trim()) return;
        const request: CreateDocumentRequest = { kind, name: name.trim(), directory: directory.trim() };
        if (kind === 'map') {
          request.width = Math.max(1, width);
          request.height = Math.max(1, height);
        }
        resolve(request);
      }, 'primary-button'));
      body.append(
        field('Document type', kindSelect),
        field('File name', nameInput),
        field('Folder', directoryInput),
        sizeFields,
        actions,
      );
    });
  }

  private async createFolder(): Promise<void> {
    if (!this.project.isOpen) return;
    const parent = this.selectedTreePath && this.findTreeNode(this.selectedTreePath)?.kind === 'directory' ? this.selectedTreePath : '';
    const path = window.prompt('New folder path', joinPath(parent, 'new-folder'));
    if (!path) return;
    try {
      await this.project.files.createDirectory(path);
      this.expandedDirectories.add(parent);
      await this.refreshProject();
      toast('Folder created.', 'success');
    } catch (error) {
      this.handleError('Failed to create folder', error);
    }
  }

  private async renameSelectedEntry(): Promise<void> {
    const path = this.selectedTreePath;
    if (!path) return;
    const node = this.findTreeNode(path);
    if (!node || node.kind !== 'file') {
      toast('Browser mode can rename files. Directory rename requires desktop Studio.', 'error');
      return;
    }
    const name = window.prompt('New file name', node.name);
    if (!name || name === node.name) return;
    try {
      const newPath = await this.project.files.renameEntry(path, name);
      if (this.currentPath === path) this.currentPath = newPath;
      this.selectedTreePath = newPath;
      await this.refreshProject();
      toast('File renamed.', 'success');
    } catch (error) {
      this.handleError('Failed to rename file', error);
    }
  }

  private async deleteSelectedEntry(): Promise<void> {
    const path = this.selectedTreePath;
    if (!path) return;
    if (!window.confirm(`Delete ${path}? This cannot be undone.`)) return;
    try {
      await this.project.files.deleteEntry(path);
      if (this.currentPath === path) {
        this.editor?.unmount();
        this.editor = null;
        this.currentPath = null;
        this.currentPathElement.textContent = 'No file selected.';
      }
      this.selectedTreePath = null;
      await this.refreshProject();
      toast('Deleted.', 'success');
    } catch (error) {
      this.handleError('Failed to delete entry', error);
    }
  }

  private findTreeNode(path: string): ProjectTreeNode | null {
    const root = this.project.projectTree;
    if (!root) return null;
    const visit = (node: ProjectTreeNode): ProjectTreeNode | null => {
      if (node.path === path) return node;
      for (const child of node.children) {
        const found = visit(child);
        if (found) return found;
      }
      return null;
    };
    return visit(root);
  }

  private async confirmDiscard(): Promise<boolean> {
    if (!this.editor?.isDirty()) return true;
    const save = window.confirm('The current file has unsaved changes. Press OK to save before continuing, or Cancel to stay in the editor.');
    if (!save) return false;
    await this.saveCurrent();
    return !this.editor.isDirty();
  }

  private updateToolbar(): void {
    const state = this.editor?.getCommandState();
    this.undoButton.disabled = !state?.canUndo;
    this.redoButton.disabled = !state?.canRedo;
    this.saveButton.disabled = !this.editor || this.editor.kind === 'asset' || !state?.dirty;
    this.sourceButton.disabled = !this.editor?.showSource;
    this.sourceButton.classList.toggle('active', this.sourceMode);
    const dirty = required<HTMLElement>('#dirty-indicator', this.root);
    dirty.textContent = state?.dirty ? '● Unsaved changes' : '';
  }

  private renderBottomPanels(): void {
    clear(this.validationHost);
    if (this.validationMessages.length === 0) {
      this.validationHost.append(element('p', { className: 'empty-state', text: 'No validation results.' }));
    } else {
      for (const message of this.validationMessages) {
        const row = element('div', { className: `validation-message ${message.severity}` });
        row.append(
          element('span', { className: 'validation-icon', text: message.severity === 'error' ? '✕' : message.severity === 'warning' ? '!' : '✓' }),
          element('span', { text: message.message }),
        );
        this.validationHost.append(row);
      }
    }

    clear(this.consoleHost);
    const pre = element('pre');
    pre.textContent = this.consoleLines.slice(-100).join('\n');
    this.consoleHost.append(pre);
  }

  private switchBottomTab(tabName: string): void {
    for (const tab of this.root.querySelectorAll<HTMLElement>('[data-bottom-tab]')) {
      tab.classList.toggle('active', tab.dataset.bottomTab === tabName);
    }
    this.validationHost.classList.toggle('active', tabName === 'validation');
    this.consoleHost.classList.toggle('active', tabName === 'console');
  }

  private log(message: string): void {
    const timestamp = new Date().toLocaleTimeString();
    this.consoleLines.push(`${timestamp} ${message}`);
    this.renderBottomPanels();
  }

  private handleError(prefix: string, error: unknown): void {
    const message = error instanceof Error ? error.message : String(error);
    this.log(`${prefix}: ${message}`);
    toast(`${prefix}: ${message}`, 'error');
  }
}

function treeContains(node: ProjectTreeNode, query: string): boolean {
  if (node.name.toLowerCase().includes(query) || node.path.toLowerCase().includes(query)) return true;
  return node.children.some((child) => treeContains(child, query));
}

function fileGlyph(node: ProjectTreeNode): string {
  if (node.kind === 'directory') return '▰';
  switch (node.fileKind) {
    case 'map': return '▦';
    case 'hui': return '▤';
    case 'prototype': return '◈';
    case 'component-schema': return '◇';
    case 'behavior': return '⌘';
    case 'localization': return '文';
    case 'asset': return '▧';
    case 'script': return '</>';
    default: return '·';
  }
}

function isAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError';
}

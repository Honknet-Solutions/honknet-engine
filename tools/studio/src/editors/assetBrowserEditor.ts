import { clear, element, selectInput } from '../core/dom';
import { StudioProject } from '../core/project';
import type { EditorCommandState, StudioEditor, ValidationMessage } from '../core/types';

export class AssetBrowserEditor implements StudioEditor {
  public readonly kind = 'asset' as const;
  public readonly title = 'Asset Browser';
  private container: HTMLElement | null = null;
  private inspector: HTMLElement | null = null;
  private filter = '';
  private assetKind: 'all' | 'image' | 'audio' | 'rsi' | 'font' = 'all';

  public constructor(
    private readonly project: StudioProject,
    private readonly openPath: (path: string) => void,
  ) {}

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

  public serialize(): string { return ''; }
  public validate(): ValidationMessage[] { return [{ severity: 'info', message: `${this.project.projectMetadata.assetSummaries.length} assets indexed.` }]; }
  public isDirty(): boolean { return false; }
  public markSaved(): void {}
  public undo(): void {}
  public redo(): void {}
  public getCommandState(): EditorCommandState { return { canUndo: false, canRedo: false, dirty: false }; }

  private render(): void {
    if (!this.container || !this.inspector) return;
    clear(this.container);
    clear(this.inspector);
    const shell = element('div', { className: 'asset-browser editor-fill' });
    const toolbar = element('div', { className: 'editor-toolbar' });
    const search = element('input', { className: 'asset-browser-search', attrs: { placeholder: 'Search resources…' } });
    search.value = this.filter;
    search.addEventListener('input', () => { this.filter = search.value; this.renderGrid(grid); });
    toolbar.append(
      element('span', { className: 'toolbar-title', text: 'Asset & Sound Browser' }),
      search,
      selectInput(this.assetKind, ['all', 'image', 'audio', 'rsi', 'font'], (value) => {
        this.assetKind = value as typeof this.assetKind;
        this.renderGrid(grid);
      }),
    );
    const grid = element('div', { className: 'asset-browser-grid' });
    shell.append(toolbar, grid);
    this.container.append(shell);
    this.renderGrid(grid);

    const metadata = this.project.projectMetadata;
    this.inspector.append(
      element('h2', { text: 'Resource Index' }),
      element('p', { text: `${metadata.assetSummaries.length} files` }),
      element('p', { text: `${metadata.rsiDirectories.length} RSI directories` }),
      element('p', { text: `${metadata.assetSummaries.filter((asset) => asset.kind === 'audio').length} audio files` }),
      element('p', { className: 'inspector-note', text: 'Double-click an asset to open its dedicated viewer. RSI directories include state and animation preview.' }),
    );
  }

  private renderGrid(grid: HTMLElement): void {
    clear(grid);
    const query = this.filter.trim().toLowerCase();
    const assets = this.project.projectMetadata.assetSummaries.filter((asset) => {
      if (this.assetKind !== 'all' && asset.kind !== this.assetKind) return false;
      return !query || asset.path.toLowerCase().includes(query);
    });
    for (const asset of assets) {
      const card = element('button', { className: `asset-browser-card asset-kind-${asset.kind}` });
      card.type = 'button';
      const icon = asset.kind === 'image' ? '▧' : asset.kind === 'audio' ? '♪' : asset.kind === 'font' ? 'Aa' : asset.kind === 'rsi' ? '◫' : '·';
      card.append(
        element('span', { className: 'asset-browser-icon', text: icon }),
        element('strong', { text: asset.path.split('/').at(-1) ?? asset.path }),
        element('small', { text: asset.path }),
      );
      card.addEventListener('dblclick', () => this.openPath(asset.path));
      card.addEventListener('click', () => this.renderAssetInspector(asset.path, asset.kind));
      grid.append(card);
    }
    for (const rsiPath of this.project.projectMetadata.rsiDirectories.filter((path) => {
      if (this.assetKind !== 'all' && this.assetKind !== 'rsi') return false;
      return !query || path.toLowerCase().includes(query);
    })) {
      const card = element('button', { className: 'asset-browser-card asset-kind-rsi' });
      card.type = 'button';
      card.append(
        element('span', { className: 'asset-browser-icon', text: '◫' }),
        element('strong', { text: rsiPath.split('/').at(-1) ?? rsiPath }),
        element('small', { text: rsiPath }),
      );
      card.addEventListener('dblclick', () => this.openPath(rsiPath));
      card.addEventListener('click', () => this.renderAssetInspector(rsiPath, 'rsi'));
      grid.append(card);
    }
    if (grid.childElementCount === 0) grid.append(element('p', { className: 'empty-state', text: 'No matching assets.' }));
  }

  private renderAssetInspector(path: string, kind: string): void {
    if (!this.inspector) return;
    clear(this.inspector);
    this.inspector.append(
      element('h2', { text: path.split('/').at(-1) ?? path }),
      element('p', { text: path }),
      element('p', { text: `Type: ${kind}` }),
      element('p', { className: 'inspector-note', text: 'Double-click the card to open the resource.' }),
    );
  }
}

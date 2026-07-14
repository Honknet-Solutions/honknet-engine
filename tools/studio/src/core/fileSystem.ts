import type { ProjectTreeNode, StudioFileKind } from './types';

const IGNORED_DIRECTORIES = new Set([
  '.git',
  'node_modules',
  'target',
  'dist',
  '.vite',
  '.idea',
  '.vscode',
]);

export class BrowserProjectFileSystem {
  private rootHandle: FileSystemDirectoryHandle | null = null;

  public get isOpen(): boolean {
    return this.rootHandle !== null;
  }

  public get projectName(): string {
    return this.rootHandle?.name ?? 'No project';
  }

  public async openProject(): Promise<void> {
    const picker = window.showDirectoryPicker;
    if (!picker) {
      throw new Error('Folder access requires Chromium/Edge and HTTPS or localhost.');
    }
    const handle = await picker({ mode: 'readwrite' });
    const permission = await (handle as DirectoryHandleExtended).requestPermission({ mode: 'readwrite' });
    if (permission !== 'granted') {
      throw new Error('Write permission was not granted.');
    }
    this.rootHandle = handle;
  }

  public async buildTree(): Promise<ProjectTreeNode> {
    const root = this.requireRoot();
    return {
      name: root.name,
      path: '',
      kind: 'directory',
      fileKind: 'unknown',
      children: await this.scanDirectory(root, ''),
    };
  }

  public async readFile(path: string): Promise<File> {
    const fileHandle = await this.getFileHandle(path, false);
    return fileHandle.getFile();
  }

  public async readText(path: string): Promise<string> {
    const file = await this.readFile(path);
    return file.text();
  }

  public async writeText(path: string, content: string): Promise<void> {
    const fileHandle = await this.getFileHandle(path, true);
    const writable = await fileHandle.createWritable();
    await writable.write(content);
    await writable.close();
  }

  public async createDirectory(path: string): Promise<void> {
    await this.getDirectoryHandle(path, true);
  }

  public async deleteEntry(path: string): Promise<void> {
    const segments = splitPath(path);
    const name = segments.pop();
    if (!name) throw new Error('Cannot delete the project root.');
    const parent = await this.getDirectoryHandle(segments.join('/'), false);
    await parent.removeEntry(name, { recursive: true });
  }

  public async renameEntry(path: string, newName: string): Promise<string> {
    const nodeKind = await this.entryKind(path);
    const parentPath = splitPath(path).slice(0, -1).join('/');
    const destination = joinPath(parentPath, newName);
    if (nodeKind === 'file') {
      const content = await this.readText(path);
      await this.writeText(destination, content);
      await this.deleteEntry(path);
      return destination;
    }
    throw new Error('Directory rename is not supported by the browser API. Create a new folder and move files manually.');
  }

  public async listFiles(path = ''): Promise<string[]> {
    const directory = await this.getDirectoryHandle(path, false);
    const files: string[] = [];
    await this.collectFiles(directory, path, files);
    return files;
  }

  private async collectFiles(
    directory: FileSystemDirectoryHandle,
    path: string,
    output: string[],
  ): Promise<void> {
    for await (const [name, handle] of (directory as DirectoryHandleExtended).entries()) {
      if (handle.kind === 'directory') {
        if (IGNORED_DIRECTORIES.has(name)) continue;
        await this.collectFiles(handle as FileSystemDirectoryHandle, joinPath(path, name), output);
      } else {
        output.push(joinPath(path, name));
      }
    }
  }

  private async scanDirectory(
    directory: FileSystemDirectoryHandle,
    path: string,
  ): Promise<ProjectTreeNode[]> {
    const nodes: ProjectTreeNode[] = [];
    for await (const [name, handle] of (directory as DirectoryHandleExtended).entries()) {
      if (handle.kind === 'directory' && IGNORED_DIRECTORIES.has(name)) continue;
      const childPath = joinPath(path, name);
      if (handle.kind === 'directory') {
        nodes.push({
          name,
          path: childPath,
          kind: 'directory',
          fileKind: 'unknown',
          children: await this.scanDirectory(handle as FileSystemDirectoryHandle, childPath),
        });
      } else {
        nodes.push({
          name,
          path: childPath,
          kind: 'file',
          fileKind: detectFileKind(childPath),
          children: [],
        });
      }
    }
    nodes.sort((left, right) => {
      if (left.kind !== right.kind) return left.kind === 'directory' ? -1 : 1;
      return left.name.localeCompare(right.name, undefined, { numeric: true });
    });
    return nodes;
  }

  private async entryKind(path: string): Promise<FileSystemHandleKind> {
    const segments = splitPath(path);
    const name = segments.pop();
    if (!name) return 'directory';
    const parent = await this.getDirectoryHandle(segments.join('/'), false);
    for await (const [entryName, handle] of (parent as DirectoryHandleExtended).entries()) {
      if (entryName === name) return handle.kind;
    }
    throw new Error(`Entry not found: ${path}`);
  }

  private async getFileHandle(path: string, create: boolean): Promise<FileSystemFileHandle> {
    const segments = splitPath(path);
    const fileName = segments.pop();
    if (!fileName) throw new Error('File path is empty.');
    const parent = await this.getDirectoryHandle(segments.join('/'), create);
    return parent.getFileHandle(fileName, { create });
  }

  private async getDirectoryHandle(path: string, create: boolean): Promise<FileSystemDirectoryHandle> {
    let directory = this.requireRoot();
    for (const segment of splitPath(path)) {
      directory = await directory.getDirectoryHandle(segment, { create });
    }
    return directory;
  }

  private requireRoot(): FileSystemDirectoryHandle {
    if (!this.rootHandle) throw new Error('Open a project folder first.');
    return this.rootHandle;
  }
}

export function detectFileKind(path: string): StudioFileKind {
  const normalized = path.toLowerCase();
  if (normalized.endsWith('.ftl')) return 'localization';
  if (normalized.endsWith('.hui') || normalized.includes('.hui.')) return 'hui';
  if (normalized.endsWith('.hgraph') || normalized.includes('.hgraph.')) return 'behavior';
  if (normalized.includes('/maps/') && /\.ya?ml$/.test(normalized)) return 'map';
  if (normalized.includes('component-schema') || normalized.includes('component_schemas') || normalized.includes('component-schemas')) return 'component-schema';
  if (normalized.includes('/prototypes/') && /\.ya?ml$/.test(normalized)) return 'prototype';
  if (/\.(png|webp|jpg|jpeg|gif|svg|ogg|wav|mp3|flac|json)$/.test(normalized)) return 'asset';
  if (/\.(ts|tsx|js|jsx|rs|toml|md)$/.test(normalized)) return 'script';
  return 'unknown';
}

export function joinPath(...parts: string[]): string {
  return parts
    .flatMap((part) => part.split('/'))
    .filter((part) => part.length > 0 && part !== '.')
    .join('/');
}

function splitPath(path: string): string[] {
  return path.split('/').filter((segment) => segment.length > 0 && segment !== '.');
}

declare global {
  interface Window {
    showDirectoryPicker?: (options?: { mode?: 'read' | 'readwrite' }) => Promise<FileSystemDirectoryHandle>;
  }
}

interface DirectoryHandleExtended extends FileSystemDirectoryHandle {
  entries(): AsyncIterableIterator<[string, FileSystemHandle]>;
  requestPermission(options?: { mode?: 'read' | 'readwrite' }): Promise<PermissionState>;
}

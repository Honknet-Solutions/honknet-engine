import type { ProjectTreeNode, StudioFileKind } from './types';

const IGNORED_DIRECTORIES = new Set([
  '.git',
  'node_modules',
  'target',
  'dist',
  '.vite',
  '.idea',
  '.vscode',
  '.honknet-backups',
]);

const HANDLE_DB = 'honknet-studio';
const HANDLE_STORE = 'handles';
const HANDLE_KEY = 'last-project';

export class BrowserProjectFileSystem {
  private rootHandle: FileSystemDirectoryHandle | null = null;

  public get isOpen(): boolean {
    return this.rootHandle !== null;
  }

  public get projectName(): string {
    return this.rootHandle?.name ?? 'No project';
  }

  public async hasRecentProject(): Promise<boolean> {
    return (await loadHandle()) !== null;
  }

  public async openProject(): Promise<void> {
    const picker = window.showDirectoryPicker;
    if (!picker) {
      throw new Error('Folder access requires Chromium/Edge and HTTPS or localhost.');
    }
    const handle = await picker({ mode: 'readwrite' });
    await ensurePermission(handle, 'readwrite');
    this.rootHandle = handle;
    await saveHandle(handle);
  }

  public async reopenRecentProject(): Promise<void> {
    const handle = await loadHandle();
    if (!handle) throw new Error('No recent project is stored in this browser.');
    await ensurePermission(handle, 'readwrite');
    this.rootHandle = handle;
  }

  public async forgetRecentProject(): Promise<void> {
    await deleteHandle();
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

  public async readJson<T>(path: string): Promise<T> {
    const source = await this.readText(path);
    return JSON.parse(source) as T;
  }

  public async writeText(path: string, content: string): Promise<void> {
    const fileHandle = await this.getFileHandle(path, true);
    const writable = await fileHandle.createWritable();
    await writable.write(content);
    await writable.close();
  }

  public async writeFile(path: string, content: Blob | BufferSource | string): Promise<void> {
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
    if (destination === path) return path;

    if (nodeKind === 'file') {
      const file = await this.readFile(path);
      await this.writeFile(destination, file);
      await this.deleteEntry(path);
      return destination;
    }

    await this.copyDirectory(path, destination);
    await this.deleteEntry(path);
    return destination;
  }

  public async copyEntry(source: string, destination: string): Promise<void> {
    const nodeKind = await this.entryKind(source);
    if (nodeKind === 'file') {
      await this.writeFile(destination, await this.readFile(source));
      return;
    }
    await this.copyDirectory(source, destination);
  }

  public async listFiles(path = ''): Promise<string[]> {
    const directory = await this.getDirectoryHandle(path, false);
    const files: string[] = [];
    await this.collectFiles(directory, path, files);
    files.sort((left, right) => left.localeCompare(right, undefined, { numeric: true }));
    return files;
  }

  public async listDirectory(path: string): Promise<Array<{ name: string; kind: FileSystemHandleKind }>> {
    const directory = await this.getDirectoryHandle(path, false);
    const entries: Array<{ name: string; kind: FileSystemHandleKind }> = [];
    for await (const [name, handle] of (directory as DirectoryHandleExtended).entries()) {
      entries.push({ name, kind: handle.kind });
    }
    entries.sort((left, right) => left.name.localeCompare(right.name, undefined, { numeric: true }));
    return entries;
  }

  private async copyDirectory(source: string, destination: string): Promise<void> {
    const sourceDirectory = await this.getDirectoryHandle(source, false);
    const destinationDirectory = await this.getDirectoryHandle(destination, true);
    await this.copyDirectoryRecursive(sourceDirectory, destinationDirectory);
  }

  private async copyDirectoryRecursive(
    source: FileSystemDirectoryHandle,
    destination: FileSystemDirectoryHandle,
  ): Promise<void> {
    for await (const [name, handle] of (source as DirectoryHandleExtended).entries()) {
      if (handle.kind === 'directory') {
        const next = await destination.getDirectoryHandle(name, { create: true });
        await this.copyDirectoryRecursive(handle as FileSystemDirectoryHandle, next);
      } else {
        const sourceFile = await (handle as FileSystemFileHandle).getFile();
        const destinationFile = await destination.getFileHandle(name, { create: true });
        const writable = await destinationFile.createWritable();
        await writable.write(sourceFile);
        await writable.close();
      }
    }
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
        const isRsi = name.toLowerCase().endsWith('.rsi');
        nodes.push({
          name,
          path: childPath,
          kind: 'directory',
          fileKind: isRsi ? 'rsi' : 'unknown',
          children: isRsi ? [] : await this.scanDirectory(handle as FileSystemDirectoryHandle, childPath),
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
  if (normalized.endsWith('.rsi')) return 'rsi';
  if (normalized.endsWith('.ftl')) return 'localization';
  if (normalized.endsWith('.hui') || normalized.includes('.hui.')) return 'hui';
  if (normalized.endsWith('.hgraph') || normalized.includes('.hgraph.')) return 'behavior';
  if (normalized.endsWith('.hsm') || normalized.includes('.hsm.')) return 'state-machine';
  if (normalized.includes('/maps/') && /\.ya?ml$/.test(normalized)) return 'map';
  if (normalized.includes('component-schema') || normalized.includes('component_schemas') || normalized.includes('component-schemas')) return 'component-schema';
  if (normalized.includes('/prototypes/') && /\.ya?ml$/.test(normalized)) return 'prototype';
  if (/\.(png|webp|jpg|jpeg|gif|svg|ogg|wav|mp3|flac|json|woff2?|ttf|otf)$/.test(normalized)) return 'asset';
  if (/\.(ts|tsx|js|jsx|rs|toml|md|css|scss|html)$/.test(normalized)) return 'script';
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

async function ensurePermission(handle: FileSystemDirectoryHandle, mode: 'read' | 'readwrite'): Promise<void> {
  const extended = handle as DirectoryHandleExtended;
  const existing = await extended.queryPermission?.({ mode });
  if (existing === 'granted') return;
  const permission = await extended.requestPermission({ mode });
  if (permission !== 'granted') throw new Error('Write permission was not granted.');
}

async function openHandleDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(HANDLE_DB, 1);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(HANDLE_STORE)) database.createObjectStore(HANDLE_STORE);
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('Failed to open project history database.'));
  });
}

async function saveHandle(handle: FileSystemDirectoryHandle): Promise<void> {
  try {
    const database = await openHandleDb();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(HANDLE_STORE, 'readwrite');
      transaction.objectStore(HANDLE_STORE).put(handle, HANDLE_KEY);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error ?? new Error('Failed to remember the project.'));
    });
    database.close();
  } catch {
    // Recent-project persistence is optional. The open project remains usable.
  }
}

async function loadHandle(): Promise<FileSystemDirectoryHandle | null> {
  try {
    const database = await openHandleDb();
    const result = await new Promise<FileSystemDirectoryHandle | null>((resolve, reject) => {
      const transaction = database.transaction(HANDLE_STORE, 'readonly');
      const request = transaction.objectStore(HANDLE_STORE).get(HANDLE_KEY);
      request.onsuccess = () => resolve((request.result as FileSystemDirectoryHandle | undefined) ?? null);
      request.onerror = () => reject(request.error ?? new Error('Failed to read recent project.'));
    });
    database.close();
    return result;
  } catch {
    return null;
  }
}

async function deleteHandle(): Promise<void> {
  try {
    const database = await openHandleDb();
    await new Promise<void>((resolve, reject) => {
      const transaction = database.transaction(HANDLE_STORE, 'readwrite');
      transaction.objectStore(HANDLE_STORE).delete(HANDLE_KEY);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error ?? new Error('Failed to clear project history.'));
    });
    database.close();
  } catch {
    // Nothing to clear.
  }
}

declare global {
  interface Window {
    showDirectoryPicker?: (options?: { mode?: 'read' | 'readwrite' }) => Promise<FileSystemDirectoryHandle>;
  }
}

interface DirectoryHandleExtended extends FileSystemDirectoryHandle {
  entries(): AsyncIterableIterator<[string, FileSystemHandle]>;
  requestPermission(options?: { mode?: 'read' | 'readwrite' }): Promise<PermissionState>;
  queryPermission?(options?: { mode?: 'read' | 'readwrite' }): Promise<PermissionState>;
}

import { Assets, Rectangle, Texture } from 'pixi.js';

export type AssetManifest = {
  version: number;
  entries: readonly { path: string; bytes: number; sha256: string }[];
};

export type RsiMetadata = {
  version: number;
  size: { x: number; y: number } | [number, number];
  states: readonly RsiStateMetadata[];
};

export type RsiStateMetadata = {
  name: string;
  directions?: 1 | 4 | 8;
  delays?: readonly (readonly number[])[];
};

export class RsiResource {
  private readonly textures = new Map<string, readonly (readonly Texture[])[]>();

  public constructor(
    public readonly path: string,
    public readonly metadata: RsiMetadata,
  ) {}

  public async initialize(): Promise<void> {
    const [width, height] = size(this.metadata.size);
    for (const state of this.metadata.states) {
      const source = await Assets.load<Texture>(`${this.path}/${state.name}.png`);
      const directions = state.directions ?? 1;
      const frameCount = Math.max(1, ...(state.delays ?? [[]]).map((delays) => delays.length));
      const stateTextures: Texture[][] = [];
      for (let direction = 0; direction < directions; direction += 1) {
        const directionTextures: Texture[] = [];
        for (let frame = 0; frame < frameCount; frame += 1) {
          const index = direction * frameCount + frame;
          const columns = Math.max(1, Math.floor(source.width / width));
          const x = (index % columns) * width;
          const y = Math.floor(index / columns) * height;
          directionTextures.push(new Texture({
            source: source.source,
            frame: new Rectangle(x, y, width, height),
          }));
        }
        stateTextures.push(directionTextures);
      }
      this.textures.set(state.name, stateTextures);
    }
  }

  public getTexture(state: string, direction = 0, frame = 0): Texture {
    const directions = this.textures.get(state);
    if (!directions) throw new Error(`RSI ${this.path} has no state ${state}`);
    const directionTextures = directions[direction % directions.length];
    return directionTextures[frame % directionTextures.length];
  }

  public getState(state: string): RsiStateMetadata | undefined {
    return this.metadata.states.find((candidate) => candidate.name === state);
  }
}

export class ResourceManager {
  private readonly rsi = new Map<string, Promise<RsiResource>>();
  private readonly textures = new Map<string, Promise<Texture>>();
  private manifest: AssetManifest | null = null;

  public async initialize(manifestPath = '/asset-manifest.json'): Promise<void> {
    const response = await fetch(manifestPath);
    if (!response.ok) throw new Error(`Failed to load asset manifest: ${response.status}`);
    this.manifest = await response.json() as AssetManifest;
  }

  public has(path: string): boolean {
    return this.manifest?.entries.some((entry) => entry.path === path) ?? false;
  }

  public loadTexture(path: string): Promise<Texture> {
    let pending = this.textures.get(path);
    if (!pending) {
      pending = Assets.load<Texture>(path);
      this.textures.set(path, pending);
    }
    return pending;
  }

  public loadRsi(path: string): Promise<RsiResource> {
    let pending = this.rsi.get(path);
    if (!pending) {
      pending = this.fetchRsi(path);
      this.rsi.set(path, pending);
    }
    return pending;
  }

  private async fetchRsi(path: string): Promise<RsiResource> {
    const response = await fetch(`${path}/meta.json`);
    if (!response.ok) throw new Error(`Failed to load RSI metadata ${path}: ${response.status}`);
    const resource = new RsiResource(path, await response.json() as RsiMetadata);
    await resource.initialize();
    return resource;
  }
}

function size(value: RsiMetadata['size']): [number, number] {
  return Array.isArray(value) ? value : [value.x, value.y];
}

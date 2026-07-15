import {
  Application,
  Container,
  Graphics,
  Sprite,
  Text,
  Texture,
} from 'pixi.js';

import type { ClientEntity } from './clientEntity';
import type {
  EntityNetId,
  MapSnapshot,
  NetPosition,
  SpriteLayerSnapshot,
} from './protocol';
import {
  ResourceManager,
  type RsiResource,
} from './resources/resourceManager';

const SCALE = 48;
const CAMERA_SPEED = 12;

type LayerAnimation = {
  sprite: Sprite;
  resource: RsiResource;
  state: string;
  direction: number;
  delays: readonly number[];
  elapsed: number;
  frame: number;
};

type EntityView = {
  container: Container;
  visual: Container;
  fallback: Graphics;
  label: Text;
  signature: string;
  generation: number;
  animations: LayerAnimation[];
};

export type PixiRendererState = {
  map: MapSnapshot | null;
  playerEntityNetId: EntityNetId | null;
  predictedPlayerPosition: NetPosition | null;
  entities: ReadonlyMap<EntityNetId, ClientEntity>;
};

export class PixiRenderer {
  private readonly app = new Application();
  private readonly mapLayer = new Container();
  private readonly entityLayer = new Container();
  private readonly views = new Map<EntityNetId, EntityView>();
  private readonly resources = new ResourceManager();
  private state: PixiRendererState = {
    map: null,
    playerEntityNetId: null,
    predictedPlayerPosition: null,
    entities: new Map(),
  };
  private renderedMapSignature: string | null = null;
  private cameraX = 0;
  private cameraY = 0;
  private initialized = false;

  public constructor(private readonly host: HTMLElement) {}

  public async initialize(): Promise<void> {
    await this.app.init({
      resizeTo: this.host,
      antialias: false,
      background: '#070b11',
      preference: 'webgl',
    });
    await this.resources.initialize();
    this.app.stage.addChild(this.mapLayer);
    this.app.stage.addChild(this.entityLayer);
    this.host.appendChild(this.app.canvas);
    this.app.canvas.classList.add('pixi-canvas');
    this.app.ticker.add(this.frame);
    this.initialized = true;
  }

  public update(state: PixiRendererState): void {
    this.state = state;
    this.syncMap();
    this.syncEntities();
  }

  public destroy(): void {
    if (!this.initialized) return;
    this.app.ticker.remove(this.frame);
    this.app.destroy(true, { children: true });
    this.views.clear();
    this.initialized = false;
  }

  private syncMap(): void {
    if (!this.state.map) {
      if (this.renderedMapSignature !== null) {
        this.mapLayer.removeChildren().forEach((child) => child.destroy({ children: true }));
        this.renderedMapSignature = null;
      }
      return;
    }

    const map = this.state.map;
    const signature = JSON.stringify(map);
    if (this.renderedMapSignature === signature) {
      return;
    }

    this.mapLayer.removeChildren().forEach((child) => child.destroy({ children: true }));
    for (const grid of map.grids) {
      const gridContainer = new Container();
      gridContainer.label = `grid:${grid.id}`;
      gridContainer.position.set(grid.position[0] * SCALE, grid.position[1] * SCALE);
      gridContainer.rotation = grid.rotation;

      for (const chunk of grid.chunks) {
        for (let localY = 0; localY < chunk.height; localY += 1) {
          for (let localX = 0; localX < chunk.width; localX += 1) {
            const tileIndex = chunk.tiles[localY * chunk.width + localX];
            const definition = map.tile_definitions[tileIndex];
            if (!definition) continue;
            const x = chunk.position[0] + localX;
            const y = chunk.position[1] + localY;
            const graphic = new Graphics();
            graphic.rect(x * SCALE, y * SCALE, SCALE, SCALE);
            const color =
              (definition.color[0] << 16) |
              (definition.color[1] << 8) |
              definition.color[2];
            graphic.fill({ color, alpha: definition.color[3] / 255 });
            graphic.stroke({ width: 1, color: 0x202a36, alpha: 0.55 });
            gridContainer.addChild(graphic);
          }
        }
      }
      this.mapLayer.addChild(gridContainer);
    }

    this.renderedMapSignature = signature;
  }

  private syncEntities(): void {
    const active = new Set<EntityNetId>();

    for (const [netId, entity] of this.state.entities) {
      active.add(netId);
      const local = netId === this.state.playerEntityNetId;
      const signature = this.signature(entity, local);
      let view = this.views.get(netId);

      if (!view) {
        const container = new Container();
        const visual = new Container();
        const fallback = new Graphics();
        const label = new Text({
          text: '',
          style: { fill: 0xffffff, fontSize: 11, align: 'center' },
        });
        label.anchor.set(0.5, 0);
        label.position.set(0, 20);
        visual.addChild(fallback);
        container.addChild(visual, label);
        this.entityLayer.addChild(container);
        view = {
          container,
          visual,
          fallback,
          label,
          signature: '',
          generation: 0,
          animations: [],
        };
        this.views.set(netId, view);
      }

      if (view.signature !== signature) {
        view.signature = signature;
        view.generation += 1;
        void this.redraw(view, entity, local, view.generation);
      }
    }

    for (const [netId, view] of this.views) {
      if (!active.has(netId)) {
        view.container.destroy({ children: true });
        this.views.delete(netId);
      }
    }
  }

  private readonly frame = (ticker: { deltaMS: number }): void => {
    const delta = Math.min(ticker.deltaMS / 1000, 0.1);
    const cameraTarget = this.getCameraTarget();

    if (cameraTarget) {
      if (this.state.predictedPlayerPosition) {
        // The local player and camera must use the exact same render-space
        // position. Interpolating the camera independently makes the player
        // oscillate around the screen centre while moving.
        this.cameraX = cameraTarget.x;
        this.cameraY = cameraTarget.y;
      } else {
        const factor = 1 - Math.exp(-CAMERA_SPEED * delta);
        this.cameraX += (cameraTarget.x - this.cameraX) * factor;
        this.cameraY += (cameraTarget.y - this.cameraY) * factor;
      }
    }

    for (const [netId, entity] of this.state.entities) {
      const view = this.views.get(netId);
      if (!view) continue;

      const position = netId === this.state.playerEntityNetId && this.state.predictedPlayerPosition
        ? this.state.predictedPlayerPosition
        : entity.renderPosition;
      view.container.position.set(position.x * SCALE, position.y * SCALE);
      view.container.rotation = entity.rotation;
      view.container.alpha = entity.player?.online === false ? 0.45 : 1;

      for (const animation of view.animations) {
        if (animation.delays.length <= 1) continue;
        animation.elapsed += delta;
        while (animation.elapsed >= animation.delays[animation.frame]) {
          animation.elapsed -= animation.delays[animation.frame];
          animation.frame = (animation.frame + 1) % animation.delays.length;
          animation.sprite.texture = animation.resource.getTexture(
            animation.state,
            animation.direction,
            animation.frame,
          );
        }
      }
    }

    const screen = this.app.renderer.screen;
    const offsetX = screen.width / 2 - this.cameraX * SCALE;
    const offsetY = screen.height / 2 - this.cameraY * SCALE;
    this.mapLayer.position.set(offsetX, offsetY);
    this.entityLayer.position.set(offsetX, offsetY);
  };

  private getCameraTarget(): NetPosition | undefined {
    if (this.state.predictedPlayerPosition) {
      return this.state.predictedPlayerPosition;
    }
    if (this.state.playerEntityNetId === null) {
      return undefined;
    }
    return this.state.entities.get(this.state.playerEntityNetId)?.renderPosition;
  }

  private signature(entity: ClientEntity, local: boolean): string {
    return JSON.stringify({
      prototype: entity.prototype,
      local,
      door: entity.door,
      item: entity.item,
      player: entity.player,
      sprite: entity.sprite,
    });
  }

  private async redraw(
    view: EntityView,
    entity: ClientEntity,
    local: boolean,
    generation: number,
  ): Promise<void> {
    view.animations = [];
    view.visual.removeChildren().forEach((child) => child.destroy());
    view.fallback = new Graphics();
    view.visual.addChild(view.fallback);
    this.redrawFallback(view.fallback, entity, local);
    view.label.text = entity.player?.display_name ?? entity.item?.name ?? entity.prototype;

    const layers = [...(entity.sprite?.layers ?? [])]
      .filter((layer) => layer.visible)
      .sort((left, right) => left.z_index - right.z_index);
    if (layers.length === 0) return;

    const sprites: Sprite[] = [];
    const animations: LayerAnimation[] = [];

    try {
      for (const layer of layers) {
        const result = await this.createLayer(layer);
        if (generation !== view.generation) {
          result.sprite.destroy();
          return;
        }
        sprites.push(result.sprite);
        if (result.animation) animations.push(result.animation);
      }
    } catch (error) {
      console.error(`Failed to load sprite for ${entity.prototype}`, error);
      for (const sprite of sprites) sprite.destroy();
      return;
    }

    if (generation !== view.generation) {
      for (const sprite of sprites) sprite.destroy();
      return;
    }

    view.fallback.destroy();
    view.visual.removeChildren();
    for (const sprite of sprites) view.visual.addChild(sprite);
    view.animations = animations;
  }

  private async createLayer(layer: SpriteLayerSnapshot): Promise<{
    sprite: Sprite;
    animation?: LayerAnimation;
  }> {
    if (layer.source.kind === 'texture') {
      const texture = await this.resources.loadTexture(layer.source.path);
      const sprite = new Sprite(texture);
      this.configureSprite(sprite, layer);
      return { sprite };
    }

    const resource = await this.resources.loadRsi(layer.source.path);
    const metadata = resource.getState(layer.source.state);
    if (!metadata) throw new Error(`Missing RSI state ${layer.source.state}`);
    const direction = layer.direction % (metadata.directions ?? 1);
    const delays = metadata.delays?.[direction] ?? [1];
    const sprite = new Sprite(resource.getTexture(layer.source.state, direction, 0));
    this.configureSprite(sprite, layer);
    return {
      sprite,
      animation: {
        sprite,
        resource,
        state: layer.source.state,
        direction,
        delays: delays.length > 0 ? delays : [1],
        elapsed: 0,
        frame: 0,
      },
    };
  }

  private configureSprite(sprite: Sprite, layer: SpriteLayerSnapshot): void {
    sprite.anchor.set(0.5);
    sprite.position.set(layer.offset[0] * SCALE, layer.offset[1] * SCALE);
    sprite.scale.set(layer.scale[0], layer.scale[1]);
    sprite.rotation = layer.rotation;
    sprite.tint = (layer.color[0] << 16) | (layer.color[1] << 8) | layer.color[2];
    sprite.alpha = layer.color[3] / 255;
  }

  private redrawFallback(graphics: Graphics, entity: ClientEntity, local: boolean): void {
    if (entity.door) {
      if (entity.door.open) {
        graphics.rect(-4, -20, 8, 40).fill(0x67d391);
      } else {
        graphics.rect(-20, -6, 40, 12).fill(0xc78555);
      }
      return;
    }

    if (entity.item) {
      graphics.circle(0, 0, 9).fill(0xffd166);
      graphics.stroke({ width: 2, color: 0xffffff });
      return;
    }

    graphics.circle(0, 0, 15).fill(local ? 0x6ee7ff : 0xff8f70);
    graphics.stroke({ width: 3, color: 0xffffff, alpha: 0.8 });
  }
}

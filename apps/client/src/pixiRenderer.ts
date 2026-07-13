import {
  Application,
  Container,
  Graphics,
  Text,
} from 'pixi.js';

import type { ClientEntity } from './clientEntity';
import type {
  EntityNetId,
  MapSnapshot,
  NetPosition,
} from './protocol';

const SCALE = 48;
const CAMERA_SPEED = 12;

type EntityView = {
  container: Container;
  graphics: Graphics;
  label: Text;
  signature: string;
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
  private state: PixiRendererState = {
    map: null,
    playerEntityNetId: null,
    predictedPlayerPosition: null,
    entities: new Map(),
  };
  private renderedMapId: string | null = null;
  private cameraX = 0;
  private cameraY = 0;
  private initialized = false;

  public constructor(private readonly host: HTMLElement) {}

  public async initialize(): Promise<void> {
    await this.app.init({
      resizeTo: this.host,
      antialias: true,
      background: '#070b11',
    });
    this.app.stage.addChild(this.mapLayer);
    this.app.stage.addChild(this.entityLayer);
    this.host.appendChild(this.app.canvas);
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
    if (!this.state.map || this.renderedMapId === this.state.map.id) {
      return;
    }

    this.mapLayer.removeChildren().forEach((child) => child.destroy());
    const map = this.state.map;

    for (let y = 0; y < map.height; y += 1) {
      for (let x = 0; x < map.width; x += 1) {
        const tile = map.tiles[y * map.width + x];
        const graphic = new Graphics();
        graphic.rect(x * SCALE, y * SCALE, SCALE, SCALE);
        graphic.fill(tile === 1 ? 0x303743 : 0x101720);
        graphic.stroke({ width: 1, color: 0x202a36, alpha: 0.65 });
        this.mapLayer.addChild(graphic);
      }
    }

    this.renderedMapId = map.id;
  }

  private syncEntities(): void {
    const active = new Set<EntityNetId>();

    for (const [netId, entity] of this.state.entities) {
      active.add(netId);
      const signature = this.signature(entity, netId === this.state.playerEntityNetId);
      let view = this.views.get(netId);

      if (!view) {
        view = {
          container: new Container(),
          graphics: new Graphics(),
          label: new Text({
            text: '',
            style: { fill: 0xffffff, fontSize: 11, align: 'center' },
          }),
          signature: '',
        };
        view.label.anchor.set(0.5, 0);
        view.label.position.set(0, 20);
        view.container.addChild(view.graphics, view.label);
        this.entityLayer.addChild(view.container);
        this.views.set(netId, view);
      }

      if (view.signature !== signature) {
        view.signature = signature;
        this.redraw(view, entity, netId === this.state.playerEntityNetId);
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
      const factor = 1 - Math.exp(-CAMERA_SPEED * delta);
      this.cameraX += (cameraTarget.x - this.cameraX) * factor;
      this.cameraY += (cameraTarget.y - this.cameraY) * factor;
    }

    for (const [netId, entity] of this.state.entities) {
      const view = this.views.get(netId);
      if (!view) continue;

      const position = netId === this.state.playerEntityNetId && this.state.predictedPlayerPosition
        ? this.state.predictedPlayerPosition
        : entity.renderPosition;
      view.container.position.set(position.x * SCALE, position.y * SCALE);
      view.container.alpha = entity.player?.online === false ? 0.45 : 1;
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
    });
  }

  private redraw(view: EntityView, entity: ClientEntity, local: boolean): void {
    const graphics = view.graphics;
    graphics.clear();

    if (entity.door) {
      if (entity.door.open) {
        graphics.rect(-4, -20, 8, 40).fill(0x67d391);
      } else {
        graphics.rect(-20, -6, 40, 12).fill(0xc78555);
      }
      view.label.text = entity.door.open ? 'Open door' : 'Closed door';
      return;
    }

    if (entity.item) {
      graphics.circle(0, 0, 9).fill(0xffd166);
      graphics.stroke({ width: 2, color: 0xffffff });
      view.label.text = entity.item.name;
      return;
    }

    graphics.circle(0, 0, 15).fill(local ? 0x6ee7ff : 0xff8f70);
    graphics.stroke({ width: 3, color: 0xffffff, alpha: 0.8 });
    view.label.text = entity.player?.display_name ?? entity.prototype;
  }
}

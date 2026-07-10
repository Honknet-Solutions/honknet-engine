import {
  Application,
  Container,
  Graphics,
} from 'pixi.js';

import type {
  EntityNetId,
  EntitySnapshot,
  Vec2,
} from './protocol';

const WORLD_SCALE = 32;

export type PixiRendererState = {
  serverTick: number | null;
  playerEntityNetId: EntityNetId | null;
  movement: Vec2;
  entities: ReadonlyMap<EntityNetId, EntitySnapshot>;
};

type EntityView = {
  container: Container;
  body: Graphics;
  isPlayer: boolean;
};

export class PixiRenderer {
  private readonly application = new Application();
  private readonly worldContainer = new Container();
  private readonly gridContainer = new Container();
  private readonly entityContainer = new Container();

  private readonly entityViews = new Map<
    EntityNetId,
    EntityView
  >();

  private state: PixiRendererState = {
    serverTick: null,
    playerEntityNetId: null,
    movement: { x: 0, y: 0 },
    entities: new Map(),
  };

  private initialized = false;
  private resizeObserver: ResizeObserver | null = null;

  public constructor(
    private readonly hostElement: HTMLElement,
  ) {}

  public async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    await this.application.init({
      width: 800,
      height: 480,
      backgroundColor: 0x071019,
      antialias: true,
      autoDensity: true,
      resolution: window.devicePixelRatio || 1,
      powerPreference: 'high-performance',
    });

    const canvas =
      this.application.canvas as HTMLCanvasElement;

    canvas.classList.add('pixi-canvas');

    this.hostElement.replaceChildren(canvas);

    this.application.stage.addChild(
      this.worldContainer,
    );

    this.worldContainer.addChild(
      this.gridContainer,
      this.entityContainer,
    );

    this.resizeObserver = new ResizeObserver(() => {
      this.resizeToHost();
    });

    this.resizeObserver.observe(this.hostElement);

    this.initialized = true;

    this.resizeToHost();
    this.synchronizeScene();
  }

  public update(state: PixiRendererState): void {
    this.state = state;

    if (!this.initialized) {
      return;
    }

    this.synchronizeScene();
  }

  public destroy(): void {
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;

    for (const entityView of this.entityViews.values()) {
      entityView.container.destroy({
        children: true,
      });
    }

    this.entityViews.clear();

    this.application.destroy({
      removeView: true,
    });

    this.initialized = false;
  }

  private resizeToHost(): void {
    if (!this.initialized) {
      return;
    }

    const width = Math.max(
      1,
      Math.floor(this.hostElement.clientWidth),
    );

    const height = Math.max(
      1,
      Math.floor(this.hostElement.clientHeight),
    );

    this.application.renderer.resize(
      width,
      height,
    );

    this.redrawGrid();
    this.updateEntityViews();
  }

  private synchronizeScene(): void {
    this.removeMissingEntityViews();
    this.createMissingEntityViews();
    this.updateEntityViews();
  }

  private removeMissingEntityViews(): void {
    for (
      const [entityNetId, entityView]
      of this.entityViews
    ) {
      if (this.state.entities.has(entityNetId)) {
        continue;
      }

      this.entityContainer.removeChild(
        entityView.container,
      );

      entityView.container.destroy({
        children: true,
      });

      this.entityViews.delete(entityNetId);
    }
  }

  private createMissingEntityViews(): void {
    for (const entity of this.state.entities.values()) {
      if (this.entityViews.has(entity.net_id)) {
        continue;
      }

      const isPlayer =
        entity.net_id ===
        this.state.playerEntityNetId;

      const entityView =
        this.createEntityView(isPlayer);

      this.entityViews.set(
        entity.net_id,
        entityView,
      );

      this.entityContainer.addChild(
        entityView.container,
      );
    }
  }

  private updateEntityViews(): void {
    const viewportWidth =
      this.application.renderer.width /
      this.application.renderer.resolution;

    const viewportHeight =
      this.application.renderer.height /
      this.application.renderer.resolution;

    for (const entity of this.state.entities.values()) {
      const entityView = this.entityViews.get(
        entity.net_id,
      );

      if (!entityView) {
        continue;
      }

      const isPlayer =
        entity.net_id ===
        this.state.playerEntityNetId;

      if (entityView.isPlayer !== isPlayer) {
        this.redrawEntityBody(
          entityView.body,
          isPlayer,
        );

        entityView.isPlayer = isPlayer;
      }

      entityView.container.position.set(
        viewportWidth / 2 +
          entity.position.x * WORLD_SCALE,
        viewportHeight / 2 +
          entity.position.y * WORLD_SCALE,
      );
    }
  }

  private createEntityView(
    isPlayer: boolean,
  ): EntityView {
    const container = new Container();
    const body = new Graphics();

    this.redrawEntityBody(body, isPlayer);
    container.addChild(body);

    return {
      container,
      body,
      isPlayer,
    };
  }

  private redrawEntityBody(
    body: Graphics,
    isPlayer: boolean,
  ): void {
    body.clear();

    const radius = isPlayer ? 12 : 10;

    const fillColor = isPlayer
      ? 0x7cffc4
      : 0xffcc66;

    const strokeColor = isPlayer
      ? 0xeafff6
      : 0xfff0c2;

    body
      .circle(0, 0, radius)
      .fill(fillColor)
      .stroke({
        color: strokeColor,
        width: 2,
      });

    if (isPlayer) {
      body
        .circle(0, 0, radius + 5)
        .stroke({
          color: 0x7cffc4,
          width: 1,
          alpha: 0.45,
        });
    }
  }

  private redrawGrid(): void {
    this.gridContainer.removeChildren();

    const grid = new Graphics();

    const viewportWidth =
      this.application.renderer.width /
      this.application.renderer.resolution;

    const viewportHeight =
      this.application.renderer.height /
      this.application.renderer.resolution;

    const centerX = viewportWidth / 2;
    const centerY = viewportHeight / 2;

    for (
      let x = centerX % WORLD_SCALE;
      x <= viewportWidth;
      x += WORLD_SCALE
    ) {
      grid.moveTo(x, 0);
      grid.lineTo(x, viewportHeight);
    }

    for (
      let y = centerY % WORLD_SCALE;
      y <= viewportHeight;
      y += WORLD_SCALE
    ) {
      grid.moveTo(0, y);
      grid.lineTo(viewportWidth, y);
    }

    grid.stroke({
      color: 0xffffff,
      width: 1,
      alpha: 0.08,
    });

    grid.moveTo(centerX, 0);
    grid.lineTo(centerX, viewportHeight);

    grid.moveTo(0, centerY);
    grid.lineTo(viewportWidth, centerY);

    grid.stroke({
      color: 0xffffff,
      width: 1,
      alpha: 0.3,
    });

    this.gridContainer.addChild(grid);
  }
}
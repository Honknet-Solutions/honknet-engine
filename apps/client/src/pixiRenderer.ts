import {
  Application,
  Container,
  Graphics,
} from 'pixi.js';

import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
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

  private cameraPosition: NetPosition = {
    x: 0,
    y: 0,
    z: 0,
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

    this.updateCameraPosition();
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

    this.gridContainer.destroy({
      children: true,
    });

    this.entityContainer.destroy({
      children: true,
    });

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
    this.redrawGrid();
  }

  private updateCameraPosition(): void {
    const playerEntityNetId =
      this.state.playerEntityNetId;

    if (playerEntityNetId === null) {
      return;
    }

    const playerEntity =
      this.state.entities.get(playerEntityNetId);

    if (!playerEntity) {
      return;
    }

    this.cameraPosition = {
      x: playerEntity.position.x,
      y: playerEntity.position.y,
      z: playerEntity.position.z,
    };
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
    const viewportSize = this.getViewportSize();

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

      const screenPosition =
        this.worldToScreen(
          entity.position,
          viewportSize,
        );

      entityView.container.position.set(
        screenPosition.x,
        screenPosition.y,
      );

      entityView.container.visible =
        entity.position.z ===
        this.cameraPosition.z;
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
    const viewportSize = this.getViewportSize();

    const cameraPixelX =
      this.cameraPosition.x * WORLD_SCALE;

    const cameraPixelY =
      this.cameraPosition.y * WORLD_SCALE;

    const centerX = viewportSize.x / 2;
    const centerY = viewportSize.y / 2;

    const horizontalOffset =
      positiveModulo(
        centerX - cameraPixelX,
        WORLD_SCALE,
      );

    const verticalOffset =
      positiveModulo(
        centerY - cameraPixelY,
        WORLD_SCALE,
      );

    for (
      let x = horizontalOffset;
      x <= viewportSize.x;
      x += WORLD_SCALE
    ) {
      grid.moveTo(x, 0);
      grid.lineTo(x, viewportSize.y);
    }

    for (
      let y = verticalOffset;
      y <= viewportSize.y;
      y += WORLD_SCALE
    ) {
      grid.moveTo(0, y);
      grid.lineTo(viewportSize.x, y);
    }

    grid.stroke({
      color: 0xffffff,
      width: 1,
      alpha: 0.08,
    });

    const worldOriginScreen =
      this.worldToScreen(
        {
          x: 0,
          y: 0,
          z: this.cameraPosition.z,
        },
        viewportSize,
      );

    if (
      worldOriginScreen.x >= 0 &&
      worldOriginScreen.x <= viewportSize.x
    ) {
      grid.moveTo(worldOriginScreen.x, 0);
      grid.lineTo(
        worldOriginScreen.x,
        viewportSize.y,
      );
    }

    if (
      worldOriginScreen.y >= 0 &&
      worldOriginScreen.y <= viewportSize.y
    ) {
      grid.moveTo(0, worldOriginScreen.y);
      grid.lineTo(
        viewportSize.x,
        worldOriginScreen.y,
      );
    }

    grid.stroke({
      color: 0xffffff,
      width: 1,
      alpha: 0.3,
    });

    this.gridContainer.addChild(grid);
  }

  private worldToScreen(
    position: NetPosition,
    viewportSize: Vec2,
  ): Vec2 {
    return {
      x:
        viewportSize.x / 2 +
        (
          position.x -
          this.cameraPosition.x
        ) *
          WORLD_SCALE,

      y:
        viewportSize.y / 2 +
        (
          position.y -
          this.cameraPosition.y
        ) *
          WORLD_SCALE,
    };
  }

  private getViewportSize(): Vec2 {
    return {
      x:
        this.application.renderer.width /
        this.application.renderer.resolution,

      y:
        this.application.renderer.height /
        this.application.renderer.resolution,
    };
  }
}

function positiveModulo(
  value: number,
  divisor: number,
): number {
  return ((value % divisor) + divisor) % divisor;
}
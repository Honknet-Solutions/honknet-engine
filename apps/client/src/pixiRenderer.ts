import {
  Application,
  Container,
  Graphics,
  Ticker,
} from 'pixi.js';

import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
  Vec2,
} from './protocol';

const WORLD_SCALE = 32;
const INTERPOLATION_SPEED = 14;

export type PixiRendererState = {
  serverTick: number | null;
  playerEntityNetId: EntityNetId | null;
  movement: Vec2;
  predictedPlayerPosition:
    NetPosition | null;
  entities: ReadonlyMap<
    EntityNetId,
    EntitySnapshot
  >;
};

type EntityView = {
  container: Container;
  body: Graphics;
  isPlayer: boolean;
  renderedPosition: NetPosition;
  targetPosition: NetPosition;
};

export class PixiRenderer {
  private readonly application =
    new Application();

  private readonly gridContainer =
    new Container();

  private readonly entityContainer =
    new Container();

  private readonly gridGraphics =
    new Graphics();

  private readonly entityViews =
    new Map<EntityNetId, EntityView>();

  private state: PixiRendererState = {
    serverTick: null,
    playerEntityNetId: null,
    movement: {
      x: 0,
      y: 0,
    },
    predictedPlayerPosition: null,
    entities: new Map(),
  };

  private cameraRenderedPosition:
    NetPosition = {
      x: 0,
      y: 0,
      z: 0,
    };

  private cameraTargetPosition:
    NetPosition = {
      x: 0,
      y: 0,
      z: 0,
    };

  private cameraInitialized = false;
  private initialized = false;

  private resizeObserver:
    ResizeObserver | null = null;

  public constructor(
    private readonly hostElement:
      HTMLElement,
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
      resolution:
        window.devicePixelRatio || 1,
      powerPreference:
        'high-performance',
    });

    const canvas =
      this.application.canvas as HTMLCanvasElement;

    canvas.classList.add(
      'pixi-canvas',
    );

    this.hostElement.replaceChildren(
      canvas,
    );

    this.gridContainer.addChild(
      this.gridGraphics,
    );

    this.application.stage.addChild(
      this.gridContainer,
      this.entityContainer,
    );

    this.application.ticker.add(
      this.updateFrame,
    );

    this.resizeObserver =
      new ResizeObserver(() => {
        this.resizeToHost();
      });

    this.resizeObserver.observe(
      this.hostElement,
    );

    this.initialized = true;

    this.resizeToHost();
    this.synchronizeScene();
  }

  public update(
    state: PixiRendererState,
  ): void {
    this.state = state;

    if (!this.initialized) {
      return;
    }

    this.updateCameraTarget();
    this.synchronizeScene();
  }

  public destroy(): void {
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;

    this.application.ticker.remove(
      this.updateFrame,
    );

    for (
      const entityView
      of this.entityViews.values()
    ) {
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

  private readonly updateFrame = (
    ticker: Ticker,
  ): void => {
    if (!this.initialized) {
      return;
    }

    const deltaSeconds = Math.min(
      ticker.deltaMS / 1000,
      0.1,
    );

    const interpolationFactor =
      1 -
      Math.exp(
        -INTERPOLATION_SPEED *
          deltaSeconds,
      );

    this.interpolateCamera(
      interpolationFactor,
    );

    this.interpolateEntities(
      interpolationFactor,
    );

    this.updateEntityTransforms();
    this.redrawGrid();
  };

  private resizeToHost(): void {
    if (!this.initialized) {
      return;
    }

    const width = Math.max(
      1,
      Math.floor(
        this.hostElement.clientWidth,
      ),
    );

    const height = Math.max(
      1,
      Math.floor(
        this.hostElement.clientHeight,
      ),
    );

    this.application.renderer.resize(
      width,
      height,
    );

    this.updateEntityTransforms();
    this.redrawGrid();
  }

  private synchronizeScene(): void {
    this.removeMissingEntityViews();
    this.createMissingEntityViews();
    this.updateEntityTargets();
    this.updateCameraTarget();
  }

  private updateCameraTarget(): void {
    const predictedPosition =
      this.state.predictedPlayerPosition;

    if (predictedPosition !== null) {
      this.cameraTargetPosition = {
        ...predictedPosition,
      };

      if (!this.cameraInitialized) {
        this.cameraRenderedPosition = {
          ...predictedPosition,
        };

        this.cameraInitialized = true;
      }

      return;
    }

    const playerEntityNetId =
      this.state.playerEntityNetId;

    if (playerEntityNetId === null) {
      return;
    }

    const playerEntity =
      this.state.entities.get(
        playerEntityNetId,
      );

    if (!playerEntity) {
      return;
    }

    this.cameraTargetPosition = {
      x: playerEntity.position.x,
      y: playerEntity.position.y,
      z: playerEntity.position.z,
    };

    if (!this.cameraInitialized) {
      this.cameraRenderedPosition = {
        ...this.cameraTargetPosition,
      };

      this.cameraInitialized = true;
    }
  }

  private removeMissingEntityViews(): void {
    for (
      const [
        entityNetId,
        entityView,
      ] of this.entityViews
    ) {
      if (
        this.state.entities.has(
          entityNetId,
        )
      ) {
        continue;
      }

      this.entityContainer.removeChild(
        entityView.container,
      );

      entityView.container.destroy({
        children: true,
      });

      this.entityViews.delete(
        entityNetId,
      );
    }
  }

  private createMissingEntityViews(): void {
    for (
      const entity
      of this.state.entities.values()
    ) {
      if (
        this.entityViews.has(
          entity.net_id,
        )
      ) {
        continue;
      }

      const isPlayer =
        entity.net_id ===
        this.state.playerEntityNetId;

      const entityView =
        this.createEntityView(
          entity,
          isPlayer,
        );

      this.entityViews.set(
        entity.net_id,
        entityView,
      );

      this.entityContainer.addChild(
        entityView.container,
      );
    }
  }

  private updateEntityTargets(): void {
    for (
      const entity
      of this.state.entities.values()
    ) {
      const entityView =
        this.entityViews.get(
          entity.net_id,
        );

      if (!entityView) {
        continue;
      }

      const isPlayer =
        entity.net_id ===
        this.state.playerEntityNetId;

      if (
        entityView.isPlayer !== isPlayer
      ) {
        this.redrawEntityBody(
          entityView.body,
          isPlayer,
        );

        entityView.isPlayer =
          isPlayer;
      }

      if (
        isPlayer &&
        this.state
          .predictedPlayerPosition !== null
      ) {
        entityView.targetPosition = {
          ...this.state
            .predictedPlayerPosition,
        };

        continue;
      }

      entityView.targetPosition = {
        x: entity.position.x,
        y: entity.position.y,
        z: entity.position.z,
      };
    }
  }

  private interpolateCamera(
    interpolationFactor: number,
  ): void {
    if (!this.cameraInitialized) {
      return;
    }

    if (
      this.state
        .predictedPlayerPosition !== null
    ) {
      this.cameraRenderedPosition = {
        ...this.cameraTargetPosition,
      };

      return;
    }

    this.cameraRenderedPosition.x =
      lerp(
        this.cameraRenderedPosition.x,
        this.cameraTargetPosition.x,
        interpolationFactor,
      );

    this.cameraRenderedPosition.y =
      lerp(
        this.cameraRenderedPosition.y,
        this.cameraTargetPosition.y,
        interpolationFactor,
      );

    this.cameraRenderedPosition.z =
      this.cameraTargetPosition.z;
  }

  private interpolateEntities(
    interpolationFactor: number,
  ): void {
    for (
      const entityView
      of this.entityViews.values()
    ) {
      if (
        entityView.isPlayer &&
        this.state
          .predictedPlayerPosition !== null
      ) {
        entityView.renderedPosition = {
          ...entityView.targetPosition,
        };

        continue;
      }

      entityView.renderedPosition.x =
        lerp(
          entityView.renderedPosition.x,
          entityView.targetPosition.x,
          interpolationFactor,
        );

      entityView.renderedPosition.y =
        lerp(
          entityView.renderedPosition.y,
          entityView.targetPosition.y,
          interpolationFactor,
        );

      entityView.renderedPosition.z =
        entityView.targetPosition.z;
    }
  }

  private updateEntityTransforms(): void {
    const viewportSize =
      this.getViewportSize();

    for (
      const entityView
      of this.entityViews.values()
    ) {
      const screenPosition =
        this.worldToScreen(
          entityView.renderedPosition,
          viewportSize,
        );

      entityView.container.position.set(
        screenPosition.x,
        screenPosition.y,
      );

      entityView.container.visible =
        entityView.renderedPosition.z ===
        this.cameraRenderedPosition.z;
    }
  }

  private createEntityView(
    entity: EntitySnapshot,
    isPlayer: boolean,
  ): EntityView {
    const container =
      new Container();

    const body =
      new Graphics();

    this.redrawEntityBody(
      body,
      isPlayer,
    );

    container.addChild(body);

    const initialPosition:
      NetPosition = {
        x: entity.position.x,
        y: entity.position.y,
        z: entity.position.z,
      };

    return {
      container,
      body,
      isPlayer,
      renderedPosition: {
        ...initialPosition,
      },
      targetPosition: {
        ...initialPosition,
      },
    };
  }

  private redrawEntityBody(
    body: Graphics,
    isPlayer: boolean,
  ): void {
    body.clear();

    const radius =
      isPlayer ? 12 : 10;

    const fillColor =
      isPlayer
        ? 0x7cffc4
        : 0xffcc66;

    const strokeColor =
      isPlayer
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
        .circle(
          0,
          0,
          radius + 5,
        )
        .stroke({
          color: 0x7cffc4,
          width: 1,
          alpha: 0.45,
        });
    }
  }

  private redrawGrid(): void {
    const viewportSize =
      this.getViewportSize();

    const cameraPixelX =
      this.cameraRenderedPosition.x *
      WORLD_SCALE;

    const cameraPixelY =
      this.cameraRenderedPosition.y *
      WORLD_SCALE;

    const centerX =
      viewportSize.x / 2;

    const centerY =
      viewportSize.y / 2;

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

    this.gridGraphics.clear();

    for (
      let x = horizontalOffset;
      x <= viewportSize.x;
      x += WORLD_SCALE
    ) {
      this.gridGraphics.moveTo(x, 0);

      this.gridGraphics.lineTo(
        x,
        viewportSize.y,
      );
    }

    for (
      let y = verticalOffset;
      y <= viewportSize.y;
      y += WORLD_SCALE
    ) {
      this.gridGraphics.moveTo(0, y);

      this.gridGraphics.lineTo(
        viewportSize.x,
        y,
      );
    }

    this.gridGraphics.stroke({
      color: 0xffffff,
      width: 1,
      alpha: 0.08,
    });

    const worldOriginScreen =
      this.worldToScreen(
        {
          x: 0,
          y: 0,
          z:
            this.cameraRenderedPosition.z,
        },
        viewportSize,
      );

    if (
      worldOriginScreen.x >= 0 &&
      worldOriginScreen.x <=
        viewportSize.x
    ) {
      this.gridGraphics.moveTo(
        worldOriginScreen.x,
        0,
      );

      this.gridGraphics.lineTo(
        worldOriginScreen.x,
        viewportSize.y,
      );
    }

    if (
      worldOriginScreen.y >= 0 &&
      worldOriginScreen.y <=
        viewportSize.y
    ) {
      this.gridGraphics.moveTo(
        0,
        worldOriginScreen.y,
      );

      this.gridGraphics.lineTo(
        viewportSize.x,
        worldOriginScreen.y,
      );
    }

    this.gridGraphics.stroke({
      color: 0xffffff,
      width: 1,
      alpha: 0.3,
    });
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
          this.cameraRenderedPosition.x
        ) *
          WORLD_SCALE,

      y:
        viewportSize.y / 2 +
        (
          position.y -
          this.cameraRenderedPosition.y
        ) *
          WORLD_SCALE,
    };
  }

  private getViewportSize(): Vec2 {
    return {
      x:
        this.application.renderer
          .screen.width,

      y:
        this.application.renderer
          .screen.height,
    };
  }
}

function lerp(
  current: number,
  target: number,
  factor: number,
): number {
  return (
    current +
    (target - current) * factor
  );
}

function positiveModulo(
  value: number,
  divisor: number,
): number {
  return (
    (value % divisor + divisor) %
    divisor
  );
}
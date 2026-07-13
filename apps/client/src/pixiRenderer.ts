import {
  Application,
  Container,
  Graphics,
  Text,
} from 'pixi.js';

import type { ClientEntity } from './clientEntity';
import type {
  EntityNetId,
  NetPosition,
  Vec2,
} from './protocol';

const WORLD_SCALE = 64;
const ENTITY_RADIUS = 18;
const POSITION_INTERPOLATION_SPEED = 14;
const CAMERA_INTERPOLATION_SPEED = 12;

type RenderEntity = {
  container: Container;
  body: Graphics;
  label: Text;

  currentX: number;
  currentY: number;

  targetX: number;
  targetY: number;

  z: number;
};

export type PixiRendererState = {
  serverTick: number | null;

  playerEntityNetId:
    EntityNetId | null;

  movement: Vec2;

  predictedPlayerPosition:
    NetPosition | null;

  entities: ReadonlyMap<
    EntityNetId,
    ClientEntity
  >;
};

export class PixiRenderer {
  private readonly host:
    HTMLElement;

  private readonly application =
    new Application();

  private readonly worldContainer =
    new Container();

  private readonly entityViews =
    new Map<
      EntityNetId,
      RenderEntity
    >();

  private state:
    PixiRendererState = {
      serverTick: null,
      playerEntityNetId: null,
      movement: {
        x: 0,
        y: 0,
      },
      predictedPlayerPosition: null,
      entities: new Map(),
    };

  private cameraX = 0;
  private cameraY = 0;

  private initialized = false;

  public constructor(
    host: HTMLElement,
  ) {
    this.host = host;
  }

  public async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    await this.application.init({
      resizeTo: this.host,
      antialias: true,
      background: '#10131a',
    });

    this.application.stage.addChild(
      this.worldContainer,
    );

    this.host.appendChild(
      this.application.canvas,
    );

    this.application.ticker.add(
      this.updateFrame,
    );

    this.initialized = true;
  }

  public update(
    state: PixiRendererState,
  ): void {
    this.state = state;

    this.synchronizeEntities();
  }

  public destroy(): void {
    if (!this.initialized) {
      return;
    }

    this.application.ticker.remove(
      this.updateFrame,
    );

    this.entityViews.clear();

    this.application.destroy(
      true,
      {
        children: true,
      },
    );

    this.initialized = false;
  }

  private synchronizeEntities(): void {
    const activeEntityIds =
      new Set<EntityNetId>();

    for (
      const [
        entityNetId,
        entity,
      ] of this.state.entities
    ) {
      activeEntityIds.add(
        entityNetId,
      );

      let view =
        this.entityViews.get(
          entityNetId,
        );

      if (!view) {
        view =
          this.createEntityView(
            entityNetId,
            entity.prototype,
          );

        this.entityViews.set(
          entityNetId,
          view,
        );

        this.worldContainer.addChild(
          view.container,
        );

        view.currentX =
          entity.position.x;

        view.currentY =
          entity.position.y;
      }

      view.label.text =
        `${entity.prototype}\n#${entityNetId}`;

      const targetPosition =
        entityNetId ===
          this.state.playerEntityNetId &&
        this.state.predictedPlayerPosition
          ? this.state
              .predictedPlayerPosition
          : entity.position;

      view.targetX =
        targetPosition.x;

      view.targetY =
        targetPosition.y;

      view.z =
        targetPosition.z;

      view.container.visible =
        targetPosition.z ===
        this.getCameraZ();
    }

    for (
      const [
        entityNetId,
        view,
      ] of this.entityViews
    ) {
      if (
        activeEntityIds.has(
          entityNetId,
        )
      ) {
        continue;
      }

      this.worldContainer.removeChild(
        view.container,
      );

      view.container.destroy({
        children: true,
      });

      this.entityViews.delete(
        entityNetId,
      );
    }
  }

  private readonly updateFrame = (
    ticker: {
      deltaMS: number;
    },
  ): void => {
    const deltaSeconds =
      Math.min(
        ticker.deltaMS / 1000,
        0.1,
      );

    this.updateEntities(
      deltaSeconds,
    );

    this.updateCamera(
      deltaSeconds,
    );
  };

  private updateEntities(
    deltaSeconds: number,
  ): void {
    for (
      const [
        entityNetId,
        view,
      ] of this.entityViews
    ) {
      const isLocalPlayer =
        entityNetId ===
        this.state.playerEntityNetId;

      if (
        isLocalPlayer &&
        this.state.predictedPlayerPosition
      ) {
        view.currentX =
          view.targetX;

        view.currentY =
          view.targetY;
      } else {
        const factor =
          1 -
          Math.exp(
            -POSITION_INTERPOLATION_SPEED *
              deltaSeconds,
          );

        view.currentX +=
          (
            view.targetX -
            view.currentX
          ) *
          factor;

        view.currentY +=
          (
            view.targetY -
            view.currentY
          ) *
          factor;
      }

      view.container.position.set(
        view.currentX *
          WORLD_SCALE,
        view.currentY *
          WORLD_SCALE,
      );
    }
  }

  private updateCamera(
    deltaSeconds: number,
  ): void {
    const cameraTarget =
      this.getCameraTarget();

    if (cameraTarget) {
      if (
        this.state
          .predictedPlayerPosition
      ) {
        this.cameraX =
          cameraTarget.x;

        this.cameraY =
          cameraTarget.y;
      } else {
        const factor =
          1 -
          Math.exp(
            -CAMERA_INTERPOLATION_SPEED *
              deltaSeconds,
          );

        this.cameraX +=
          (
            cameraTarget.x -
            this.cameraX
          ) *
          factor;

        this.cameraY +=
          (
            cameraTarget.y -
            this.cameraY
          ) *
          factor;
      }
    }

    const viewportSize =
      this.getViewportSize();

    this.worldContainer.position.set(
      viewportSize.width / 2 -
        this.cameraX *
          WORLD_SCALE,

      viewportSize.height / 2 -
        this.cameraY *
          WORLD_SCALE,
    );
  }

  private getCameraTarget():
    NetPosition | undefined {
    if (
      this.state
        .predictedPlayerPosition
    ) {
      return this.state
        .predictedPlayerPosition;
    }

    if (
      this.state.playerEntityNetId ===
      null
    ) {
      return undefined;
    }

    return this.state.entities.get(
      this.state.playerEntityNetId,
    )?.position;
  }

  private getCameraZ(): number {
    if (
      this.state
        .predictedPlayerPosition
    ) {
      return this.state
        .predictedPlayerPosition.z;
    }

    if (
      this.state.playerEntityNetId ===
      null
    ) {
      return 0;
    }

    return (
      this.state.entities.get(
        this.state.playerEntityNetId,
      )?.position.z ?? 0
    );
  }

  private createEntityView(
    entityNetId: EntityNetId,
    prototype: string,
  ): RenderEntity {
    const container =
      new Container();

    const body =
      new Graphics();

    body.circle(
      0,
      0,
      ENTITY_RADIUS,
    );

    body.fill(
      entityNetId ===
        this.state.playerEntityNetId
        ? 0x6ee7ff
        : 0xffc857,
    );

    body.stroke({
      width: 3,
      color: 0xffffff,
      alpha: 0.8,
    });

    const label =
      new Text({
        text:
          `${prototype}\n#${entityNetId}`,

        style: {
          fill: 0xffffff,
          fontSize: 12,
          align: 'center',
        },
      });

    label.anchor.set(
      0.5,
      0,
    );

    label.position.set(
      0,
      ENTITY_RADIUS + 8,
    );

    container.addChild(
      body,
    );

    container.addChild(
      label,
    );

    return {
      container,
      body,
      label,

      currentX: 0,
      currentY: 0,

      targetX: 0,
      targetY: 0,

      z: 0,
    };
  }

  private getViewportSize(): {
    width: number;
    height: number;
  } {
    return {
      width:
        this.application.renderer
          .screen.width,

      height:
        this.application.renderer
          .screen.height,
    };
  }
}
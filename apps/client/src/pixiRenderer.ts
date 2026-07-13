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

const CAMERA_INTERPOLATION_SPEED = 12;
const MAX_RENDER_DELTA_SECONDS = 0.1;

type RenderEntity = {
  container: Container;
  body: Graphics;
  label: Text;

  prototype: string;
  isLocalPlayer: boolean;
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
  private cameraZ = 0;

  private initialized = false;

  public constructor(
    host: HTMLElement,
  ) {
    this.host = host;
  }

  public async initialize():
    Promise<void> {
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

    this.synchronizeEntityViews();
  }

  public destroy(): void {
    if (!this.initialized) {
      return;
    }

    this.application.ticker.remove(
      this.updateFrame,
    );

    for (
      const view
      of this.entityViews.values()
    ) {
      view.container.destroy({
        children: true,
      });
    }

    this.entityViews.clear();

    this.application.destroy(
      true,
      {
        children: true,
      },
    );

    this.initialized = false;
  }

  private synchronizeEntityViews(): void {
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

      const isLocalPlayer =
        entityNetId ===
        this.state.playerEntityNetId;

      let view =
        this.entityViews.get(
          entityNetId,
        );

      if (!view) {
        view =
          this.createEntityView(
            entityNetId,
            entity.prototype,
            isLocalPlayer,
          );

        this.entityViews.set(
          entityNetId,
          view,
        );

        this.worldContainer.addChild(
          view.container,
        );
      }

      if (
        view.prototype !==
        entity.prototype
      ) {
        view.prototype =
          entity.prototype;

        this.updateEntityLabel(
          view,
          entityNetId,
          entity.prototype,
        );
      }

      if (
        view.isLocalPlayer !==
        isLocalPlayer
      ) {
        view.isLocalPlayer =
          isLocalPlayer;

        this.redrawEntityBody(
          view.body,
          isLocalPlayer,
        );
      }
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
        Math.max(
          ticker.deltaMS / 1000,
          0,
        ),
        MAX_RENDER_DELTA_SECONDS,
      );

    this.updateEntityPresentation(
      deltaSeconds,
    );

    this.updateCamera(
      deltaSeconds,
    );
  };

  private updateEntityPresentation(
    deltaSeconds: number,
  ): void {
    const cameraZ =
      this.getCameraZ();

    for (
      const [
        entityNetId,
        entity,
      ] of this.state.entities
    ) {
      const view =
        this.entityViews.get(
          entityNetId,
        );

      if (!view) {
        continue;
      }

      const isLocalPlayer =
        entityNetId ===
        this.state.playerEntityNetId;

      if (
        isLocalPlayer &&
        this.state
          .predictedPlayerPosition
      ) {
        entity.setRenderPosition(
          this.state
            .predictedPlayerPosition,
        );
      } else {
        entity.updateRenderPosition(
          deltaSeconds,
        );
      }

      view.container.visible =
        entity.renderPosition.z ===
        cameraZ;

      view.container.position.set(
        entity.renderPosition.x *
          WORLD_SCALE,

        entity.renderPosition.y *
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
      this.cameraZ =
        cameraTarget.z;

      const usingPrediction =
        this.state
          .predictedPlayerPosition !==
        null;

      if (usingPrediction) {
        this.cameraX =
          cameraTarget.x;

        this.cameraY =
          cameraTarget.y;
      } else {
        const interpolationFactor =
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
          interpolationFactor;

        this.cameraY +=
          (
            cameraTarget.y -
            this.cameraY
          ) *
          interpolationFactor;
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
    )?.renderPosition;
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
      return this.cameraZ;
    }

    return (
      this.state.entities.get(
        this.state.playerEntityNetId,
      )?.renderPosition.z ??
      this.cameraZ
    );
  }

  private createEntityView(
    entityNetId: EntityNetId,
    prototype: string,
    isLocalPlayer: boolean,
  ): RenderEntity {
    const container =
      new Container();

    const body =
      new Graphics();

    this.redrawEntityBody(
      body,
      isLocalPlayer,
    );

    const label =
      new Text({
        text: '',

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

    const view: RenderEntity = {
      container,
      body,
      label,
      prototype,
      isLocalPlayer,
    };

    this.updateEntityLabel(
      view,
      entityNetId,
      prototype,
    );

    return view;
  }

  private redrawEntityBody(
    body: Graphics,
    isLocalPlayer: boolean,
  ): void {
    body.clear();

    body.circle(
      0,
      0,
      ENTITY_RADIUS,
    );

    body.fill(
      isLocalPlayer
        ? 0x6ee7ff
        : 0xffc857,
    );

    body.stroke({
      width: 3,
      color: 0xffffff,
      alpha: 0.8,
    });
  }

  private updateEntityLabel(
    view: RenderEntity,
    entityNetId: EntityNetId,
    prototype: string,
  ): void {
    view.label.text =
      `${prototype}\n#${entityNetId}`;
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
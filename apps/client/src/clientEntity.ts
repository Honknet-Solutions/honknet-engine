import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
} from './protocol';

const POSITION_INTERPOLATION_SPEED = 14;

const MAX_RENDER_DELTA_SECONDS = 0.1;

export class ClientEntity {
  public readonly netId: EntityNetId;

  public prototype: string;

  public readonly authoritativePosition:
    NetPosition;

  public readonly renderPosition:
    NetPosition;

  public lastServerTick: number;

  public constructor(
    snapshot: EntitySnapshot,
    serverTick: number,
  ) {
    this.netId =
      snapshot.net_id;

    this.prototype =
      snapshot.prototype;

    this.authoritativePosition = {
      x: snapshot.position.x,
      y: snapshot.position.y,
      z: snapshot.position.z,
    };

    this.renderPosition = {
      x: snapshot.position.x,
      y: snapshot.position.y,
      z: snapshot.position.z,
    };

    this.lastServerTick =
      serverTick;
  }

  /**
   * Временная совместимость для систем,
   * которые пока читают entity.position.
   *
   * position всегда означает авторитетную
   * серверную позицию.
   */
  public get position(): NetPosition {
    return this.authoritativePosition;
  }

  public applySnapshot(
    snapshot: EntitySnapshot,
    serverTick: number,
  ): boolean {
    let changed = false;

    if (
      this.prototype !==
      snapshot.prototype
    ) {
      this.prototype =
        snapshot.prototype;

      changed = true;
    }

    const zChanged =
      this.authoritativePosition.z !==
      snapshot.position.z;

    if (
      this.authoritativePosition.x !==
      snapshot.position.x
    ) {
      this.authoritativePosition.x =
        snapshot.position.x;

      changed = true;
    }

    if (
      this.authoritativePosition.y !==
      snapshot.position.y
    ) {
      this.authoritativePosition.y =
        snapshot.position.y;

      changed = true;
    }

    if (zChanged) {
      this.authoritativePosition.z =
        snapshot.position.z;

      changed = true;

      /*
       * При смене этажа нельзя плавно
       * интерполировать сущность между слоями.
       */
      this.snapRenderToAuthoritative();
    }

    this.lastServerTick =
      serverTick;

    return changed;
  }

  public updateRenderPosition(
    deltaSeconds: number,
  ): void {
    const safeDeltaSeconds =
      Math.min(
        Math.max(deltaSeconds, 0),
        MAX_RENDER_DELTA_SECONDS,
      );

    if (
      this.renderPosition.z !==
      this.authoritativePosition.z
    ) {
      this.snapRenderToAuthoritative();

      return;
    }

    const interpolationFactor =
      1 -
      Math.exp(
        -POSITION_INTERPOLATION_SPEED *
          safeDeltaSeconds,
      );

    this.renderPosition.x +=
      (
        this.authoritativePosition.x -
        this.renderPosition.x
      ) *
      interpolationFactor;

    this.renderPosition.y +=
      (
        this.authoritativePosition.y -
        this.renderPosition.y
      ) *
      interpolationFactor;
  }

  public setRenderPosition(
    position: NetPosition,
  ): void {
    this.renderPosition.x =
      position.x;

    this.renderPosition.y =
      position.y;

    this.renderPosition.z =
      position.z;
  }

  public snapRenderToAuthoritative(): void {
    this.renderPosition.x =
      this.authoritativePosition.x;

    this.renderPosition.y =
      this.authoritativePosition.y;

    this.renderPosition.z =
      this.authoritativePosition.z;
  }

  public toSnapshot(): EntitySnapshot {
    return {
      net_id:
        this.netId,

      prototype:
        this.prototype,

      position: {
        x: this.authoritativePosition.x,
        y: this.authoritativePosition.y,
        z: this.authoritativePosition.z,
      },
    };
  }
}
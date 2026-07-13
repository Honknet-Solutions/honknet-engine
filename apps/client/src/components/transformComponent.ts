import type { ClientComponent } from './clientComponent';
import type {
  EntitySnapshot,
  NetPosition,
} from '../protocol';

const POSITION_INTERPOLATION_SPEED = 14;

const MAX_RENDER_DELTA_SECONDS = 0.1;

export class TransformComponent
  implements ClientComponent
{
  public static readonly type =
    'transform';

  public readonly componentType =
    TransformComponent.type;

  public readonly authoritativePosition:
    NetPosition;

  public readonly renderPosition:
    NetPosition;

  public constructor(
    snapshot: EntitySnapshot,
  ) {
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
  }

  public applySnapshot(
    snapshot: EntitySnapshot,
  ): boolean {
    let changed = false;

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

      this.snapRenderToAuthoritative();
    }

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
}
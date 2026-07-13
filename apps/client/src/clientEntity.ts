import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
} from './protocol';

export class ClientEntity {
  public readonly netId: EntityNetId;

  public prototype: string;

  public readonly position: NetPosition;

  public lastServerTick: number;

  public constructor(
    snapshot: EntitySnapshot,
    serverTick: number,
  ) {
    this.netId = snapshot.net_id;

    this.prototype =
      snapshot.prototype;

    this.position = {
      x: snapshot.position.x,
      y: snapshot.position.y,
      z: snapshot.position.z,
    };

    this.lastServerTick =
      serverTick;
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

    if (
      this.position.x !==
      snapshot.position.x
    ) {
      this.position.x =
        snapshot.position.x;

      changed = true;
    }

    if (
      this.position.y !==
      snapshot.position.y
    ) {
      this.position.y =
        snapshot.position.y;

      changed = true;
    }

    if (
      this.position.z !==
      snapshot.position.z
    ) {
      this.position.z =
        snapshot.position.z;

      changed = true;
    }

    this.lastServerTick =
      serverTick;

    return changed;
  }

  public toSnapshot(): EntitySnapshot {
    return {
      net_id: this.netId,

      prototype:
        this.prototype,

      position: {
        x: this.position.x,
        y: this.position.y,
        z: this.position.z,
      },
    };
  }
}
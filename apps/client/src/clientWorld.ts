import type {
  EntityNetId,
  EntitySnapshot,
} from './protocol';

export type ClientWorldState = {
  serverTick: number | null;
  entities: ReadonlyMap<
    EntityNetId,
    EntitySnapshot
  >;
};

export class ClientWorld {
  private serverTick:
    number | null = null;

  private readonly entities =
    new Map<
      EntityNetId,
      EntitySnapshot
    >();

  public applySnapshot(
    serverTick: number,
    snapshots: readonly EntitySnapshot[],
  ): void {
    this.serverTick = serverTick;

    this.entities.clear();

    for (const snapshot of snapshots) {
      this.entities.set(
        snapshot.net_id,
        cloneEntitySnapshot(snapshot),
      );
    }
  }

  public clear(): void {
    this.serverTick = null;
    this.entities.clear();
  }

  public getServerTick():
    number | null {
    return this.serverTick;
  }

  public getEntity(
    entityNetId: EntityNetId,
  ): EntitySnapshot | undefined {
    return this.entities.get(
      entityNetId,
    );
  }

  public hasEntity(
    entityNetId: EntityNetId,
  ): boolean {
    return this.entities.has(
      entityNetId,
    );
  }

  public getEntityCount(): number {
    return this.entities.size;
  }

  public getEntities(): ReadonlyMap<
    EntityNetId,
    EntitySnapshot
  > {
    return this.entities;
  }

  public getState(): ClientWorldState {
    return {
      serverTick: this.serverTick,
      entities: this.entities,
    };
  }
}

function cloneEntitySnapshot(
  snapshot: EntitySnapshot,
): EntitySnapshot {
  return {
    net_id: snapshot.net_id,
    prototype: snapshot.prototype,
    position: {
      x: snapshot.position.x,
      y: snapshot.position.y,
      z: snapshot.position.z,
    },
  };
}
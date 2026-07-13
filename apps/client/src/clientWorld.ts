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

export type ClientWorldSnapshotResult = {
  serverTick: number;

  createdEntityIds: EntityNetId[];

  updatedEntityIds: EntityNetId[];

  removedEntityIds: EntityNetId[];
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
  ): ClientWorldSnapshotResult {
    const createdEntityIds:
      EntityNetId[] = [];

    const updatedEntityIds:
      EntityNetId[] = [];

    const removedEntityIds:
      EntityNetId[] = [];

    const snapshotEntityIds =
      new Set<EntityNetId>();

    for (const snapshot of snapshots) {
      snapshotEntityIds.add(
        snapshot.net_id,
      );

      const existingEntity =
        this.entities.get(
          snapshot.net_id,
        );

      if (!existingEntity) {
        this.entities.set(
          snapshot.net_id,
          cloneEntitySnapshot(
            snapshot,
          ),
        );

        createdEntityIds.push(
          snapshot.net_id,
        );

        continue;
      }

      if (
        updateEntitySnapshot(
          existingEntity,
          snapshot,
        )
      ) {
        updatedEntityIds.push(
          snapshot.net_id,
        );
      }
    }

    for (
      const entityNetId
      of this.entities.keys()
    ) {
      if (
        snapshotEntityIds.has(
          entityNetId,
        )
      ) {
        continue;
      }

      this.entities.delete(
        entityNetId,
      );

      removedEntityIds.push(
        entityNetId,
      );
    }

    this.serverTick =
      serverTick;

    return {
      serverTick,
      createdEntityIds,
      updatedEntityIds,
      removedEntityIds,
    };
  }

  public clear(): EntityNetId[] {
    const removedEntityIds = [
      ...this.entities.keys(),
    ];

    this.serverTick = null;
    this.entities.clear();

    return removedEntityIds;
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

    prototype:
      snapshot.prototype,

    position: {
      x: snapshot.position.x,
      y: snapshot.position.y,
      z: snapshot.position.z,
    },
  };
}

function updateEntitySnapshot(
  target: EntitySnapshot,
  source: EntitySnapshot,
): boolean {
  let changed = false;

  if (
    target.prototype !==
    source.prototype
  ) {
    target.prototype =
      source.prototype;

    changed = true;
  }

  if (
    target.position.x !==
    source.position.x
  ) {
    target.position.x =
      source.position.x;

    changed = true;
  }

  if (
    target.position.y !==
    source.position.y
  ) {
    target.position.y =
      source.position.y;

    changed = true;
  }

  if (
    target.position.z !==
    source.position.z
  ) {
    target.position.z =
      source.position.z;

    changed = true;
  }

  return changed;
}
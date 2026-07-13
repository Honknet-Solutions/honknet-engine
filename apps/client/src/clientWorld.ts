import { ClientEntity } from './clientEntity';
import type {
  EntityNetId,
  EntitySnapshot,
} from './protocol';

export type ClientWorldState = {
  serverTick: number | null;

  entities: ReadonlyMap<
    EntityNetId,
    ClientEntity
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
      ClientEntity
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

    const receivedEntityIds =
      new Set<EntityNetId>();

    for (const snapshot of snapshots) {
      receivedEntityIds.add(
        snapshot.net_id,
      );

      const existingEntity =
        this.entities.get(
          snapshot.net_id,
        );

      if (!existingEntity) {
        const entity =
          new ClientEntity(
            snapshot,
            serverTick,
          );

        this.entities.set(
          entity.netId,
          entity,
        );

        createdEntityIds.push(
          entity.netId,
        );

        continue;
      }

      const changed =
        existingEntity.applySnapshot(
          snapshot,
          serverTick,
        );

      if (changed) {
        updatedEntityIds.push(
          existingEntity.netId,
        );
      }
    }

    for (
      const entityNetId
      of this.entities.keys()
    ) {
      if (
        receivedEntityIds.has(
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
  ): ClientEntity | undefined {
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
    ClientEntity
  > {
    return this.entities;
  }

  public getState(): ClientWorldState {
    return {
      serverTick:
        this.serverTick,

      entities:
        this.entities,
    };
  }
}
import { ClientEntity } from './clientEntity';
import type {
  EntityNetId,
  EntitySnapshot,
} from './protocol';
import type { ClientSystem } from './systems/clientSystem';
import { ClientSystemManager } from './systems/clientSystemManager';

export type SnapshotResult = {
  created: number;
  updated: number;
  removed: number;
};

export class ClientWorld {
  private serverTick: number | null = null;
  private readonly entities = new Map<EntityNetId, ClientEntity>();
  private readonly systems = new ClientSystemManager();

  public addSystem(system: ClientSystem): void {
    this.systems.add(this, system);
  }

  public applySnapshot(
    serverTick: number,
    snapshots: readonly EntitySnapshot[],
  ): SnapshotResult {
    const received = new Set<EntityNetId>();
    let created = 0;
    let updated = 0;
    let removed = 0;

    for (const snapshot of snapshots) {
      received.add(snapshot.net_id);
      const existing = this.entities.get(snapshot.net_id);

      if (!existing) {
        const entity = new ClientEntity(snapshot, serverTick);
        this.entities.set(entity.netId, entity);
        this.systems.created(this, entity);
        created += 1;
      } else if (existing.applySnapshot(snapshot, serverTick)) {
        this.systems.updated(this, existing);
        updated += 1;
      }
    }

    for (const [netId, entity] of this.entities) {
      if (!received.has(netId)) {
        this.systems.removed(this, entity);
        this.entities.delete(netId);
        removed += 1;
      }
    }

    this.serverTick = serverTick;
    return { created, updated, removed };
  }


  public applyDelta(
    serverTick: number,
    spawns: readonly EntitySnapshot[],
    updates: readonly EntitySnapshot[],
    despawns: readonly EntityNetId[],
  ): SnapshotResult {
    let created = 0;
    let updated = 0;
    let removed = 0;

    for (const snapshot of spawns) {
      const existing = this.entities.get(snapshot.net_id);
      if (existing) {
        if (existing.applySnapshot(snapshot, serverTick)) {
          this.systems.updated(this, existing);
          updated += 1;
        }
        continue;
      }
      const entity = new ClientEntity(snapshot, serverTick);
      this.entities.set(entity.netId, entity);
      this.systems.created(this, entity);
      created += 1;
    }

    for (const snapshot of updates) {
      const existing = this.entities.get(snapshot.net_id);
      if (!existing) {
        const entity = new ClientEntity(snapshot, serverTick);
        this.entities.set(entity.netId, entity);
        this.systems.created(this, entity);
        created += 1;
      } else if (existing.applySnapshot(snapshot, serverTick)) {
        this.systems.updated(this, existing);
        updated += 1;
      }
    }

    for (const netId of despawns) {
      const entity = this.entities.get(netId);
      if (!entity) continue;
      this.systems.removed(this, entity);
      this.entities.delete(netId);
      removed += 1;
    }

    this.serverTick = serverTick;
    return { created, updated, removed };
  }

  public update(deltaSeconds: number): void {
    this.systems.update(this, deltaSeconds);
  }

  public clear(): void {
    for (const entity of this.entities.values()) {
      this.systems.removed(this, entity);
    }
    this.entities.clear();
    this.serverTick = null;
  }

  public getServerTick(): number | null {
    return this.serverTick;
  }

  public getEntity(netId: EntityNetId): ClientEntity | undefined {
    return this.entities.get(netId);
  }

  public getEntities(): ReadonlyMap<EntityNetId, ClientEntity> {
    return this.entities;
  }
}

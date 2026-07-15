import type { ClientEntity } from '../clientEntity';
import type { ClientWorld } from '../clientWorld';
import type { ClientSystem } from './clientSystem';

export class ClientSystemManager {
  private readonly systems = new Map<string, ClientSystem>();

  public add(world: ClientWorld, system: ClientSystem): void {
    if (this.systems.has(system.systemType)) {
      throw new Error(`Duplicate system ${system.systemType}`);
    }
    this.systems.set(system.systemType, system);
    for (const entity of world.getEntities().values()) {
      system.onEntityCreated?.(world, entity);
    }
  }

  public update(world: ClientWorld, deltaSeconds: number): void {
    for (const system of this.systems.values()) {
      system.update?.(world, deltaSeconds);
    }
  }

  public created(world: ClientWorld, entity: ClientEntity): void {
    for (const system of this.systems.values()) {
      system.onEntityCreated?.(world, entity);
    }
  }

  public updated(world: ClientWorld, entity: ClientEntity): void {
    for (const system of this.systems.values()) {
      system.onEntityUpdated?.(world, entity);
    }
  }

  public removed(world: ClientWorld, entity: ClientEntity): void {
    for (const system of this.systems.values()) {
      system.onEntityRemoved?.(world, entity);
    }
  }
}

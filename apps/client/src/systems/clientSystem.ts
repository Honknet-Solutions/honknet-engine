import type { ClientEntity } from '../clientEntity';
import type { ClientWorld } from '../clientWorld';

export interface ClientSystem {
  readonly systemType: string;
  onEntityCreated?(world: ClientWorld, entity: ClientEntity): void;
  onEntityUpdated?(world: ClientWorld, entity: ClientEntity): void;
  onEntityRemoved?(world: ClientWorld, entity: ClientEntity): void;
  update?(world: ClientWorld, deltaSeconds: number): void;
}

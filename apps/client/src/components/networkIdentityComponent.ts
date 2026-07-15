import type { ClientComponent } from './clientComponent';
import type {
  EntityNetId,
  EntitySnapshot,
} from '../protocol';

export class NetworkIdentityComponent implements ClientComponent {
  public static readonly type = 'networkIdentity';
  public readonly componentType = NetworkIdentityComponent.type;
  public readonly netId: EntityNetId;
  public prototype: string;
  public lastServerTick: number;

  public constructor(snapshot: EntitySnapshot, serverTick: number) {
    this.netId = snapshot.net_id;
    this.prototype = snapshot.prototype;
    this.lastServerTick = serverTick;
  }

  public applySnapshot(snapshot: EntitySnapshot, serverTick: number): boolean {
    const changed = this.prototype !== snapshot.prototype;
    this.prototype = snapshot.prototype;
    this.lastServerTick = serverTick;
    return changed;
  }
}

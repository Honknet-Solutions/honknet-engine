import type { ClientComponent } from './components/clientComponent';
import { NetworkIdentityComponent } from './components/networkIdentityComponent';
import { ReplicatedStateComponent } from './components/replicatedStateComponent';
import { TransformComponent } from './components/transformComponent';
import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
} from './protocol';

export class ClientEntity {
  private readonly components = new Map<string, ClientComponent>();

  public constructor(snapshot: EntitySnapshot, serverTick: number) {
    this.addComponent(new NetworkIdentityComponent(snapshot, serverTick));
    this.addComponent(new TransformComponent(snapshot));
    this.addComponent(new ReplicatedStateComponent(snapshot));
  }

  public get netId(): EntityNetId {
    return this.identity.netId;
  }

  public get prototype(): string {
    return this.identity.prototype;
  }

  public get position(): NetPosition {
    return this.transform.authoritativePosition;
  }

  public get renderPosition(): NetPosition {
    return this.transform.renderPosition;
  }

  public get player() {
    return this.replicated.get('Player')?.data;
  }

  public get door() {
    return this.replicated.get('Door')?.data;
  }

  public get item() {
    return this.replicated.get('Item')?.data;
  }

  public get inventory() {
    return this.replicated.get('Inventory')?.data;
  }

  public addComponent(component: ClientComponent): void {
    if (this.components.has(component.componentType)) {
      throw new Error(`Duplicate component ${component.componentType}`);
    }
    this.components.set(component.componentType, component);
  }

  public getComponent<T extends ClientComponent>(type: string): T | undefined {
    return this.components.get(type) as T | undefined;
  }

  public applySnapshot(snapshot: EntitySnapshot, serverTick: number): boolean {
    if (snapshot.net_id !== this.netId) {
      throw new Error('Snapshot entity id mismatch');
    }

    const identityChanged = this.identity.applySnapshot(snapshot, serverTick);
    const transformChanged = this.transform.applySnapshot(snapshot);
    const replicatedChanged = this.replicated.applySnapshot(snapshot);

    return identityChanged || transformChanged || replicatedChanged;
  }

  public updateRenderPosition(deltaSeconds: number): void {
    this.transform.update(deltaSeconds);
  }

  private get identity(): NetworkIdentityComponent {
    return this.require(NetworkIdentityComponent.type);
  }

  private get transform(): TransformComponent {
    return this.require(TransformComponent.type);
  }

  private get replicated(): ReplicatedStateComponent {
    return this.require(ReplicatedStateComponent.type);
  }

  private require<T extends ClientComponent>(type: string): T {
    const component = this.getComponent<T>(type);
    if (!component) {
      throw new Error(`Missing component ${type}`);
    }
    return component;
  }
}

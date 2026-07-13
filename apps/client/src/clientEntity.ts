import type { ClientComponent } from './components/clientComponent';
import { NetworkIdentityComponent } from './components/networkIdentityComponent';
import { TransformComponent } from './components/transformComponent';
import type {
  EntityNetId,
  EntitySnapshot,
  NetPosition,
} from './protocol';

export class ClientEntity {
  private readonly components =
    new Map<
      string,
      ClientComponent
    >();

  public constructor(
    snapshot: EntitySnapshot,
    serverTick: number,
  ) {
    this.addComponent(
      new NetworkIdentityComponent(
        snapshot,
        serverTick,
      ),
    );

    this.addComponent(
      new TransformComponent(
        snapshot,
      ),
    );
  }

  public get netId(): EntityNetId {
    return this.networkIdentity.netId;
  }

  public get prototype(): string {
    return this.networkIdentity.prototype;
  }

  public get lastServerTick(): number {
    return this.networkIdentity
      .lastServerTick;
  }

  public get authoritativePosition():
    NetPosition {
    return this.transform
      .authoritativePosition;
  }

  public get renderPosition():
    NetPosition {
    return this.transform
      .renderPosition;
  }

  /**
   * Временная совместимость для систем,
   * которые читают entity.position.
   *
   * position означает последнюю
   * авторитетную серверную позицию.
   */
  public get position(): NetPosition {
    return this.authoritativePosition;
  }

  public addComponent(
    component: ClientComponent,
  ): void {
    if (
      this.components.has(
        component.componentType,
      )
    ) {
      throw new Error(
        `Entity already has component: ${component.componentType}`,
      );
    }

    this.components.set(
      component.componentType,
      component,
    );
  }

  public removeComponent(
    componentType: string,
  ): boolean {
    return this.components.delete(
      componentType,
    );
  }

  public hasComponent(
    componentType: string,
  ): boolean {
    return this.components.has(
      componentType,
    );
  }

  public getComponent<
    TComponent extends ClientComponent,
  >(
    componentType: string,
  ): TComponent | undefined {
    return this.components.get(
      componentType,
    ) as TComponent | undefined;
  }

  public requireComponent<
    TComponent extends ClientComponent,
  >(
    componentType: string,
  ): TComponent {
    const component =
      this.getComponent<TComponent>(
        componentType,
      );

    if (!component) {
      throw new Error(
        `Entity ${this.netId} is missing component: ${componentType}`,
      );
    }

    return component;
  }

  public getComponents():
    readonly ClientComponent[] {
    return [
      ...this.components.values(),
    ];
  }

  public applySnapshot(
    snapshot: EntitySnapshot,
    serverTick: number,
  ): boolean {
    if (
      snapshot.net_id !==
      this.netId
    ) {
      throw new Error(
        `Cannot apply snapshot for entity ${snapshot.net_id} to entity ${this.netId}`,
      );
    }

    const identityChanged =
      this.networkIdentity
        .applySnapshot(
          snapshot,
          serverTick,
        );

    const transformChanged =
      this.transform
        .applySnapshot(
          snapshot,
        );

    return (
      identityChanged ||
      transformChanged
    );
  }

  public updateRenderPosition(
    deltaSeconds: number,
  ): void {
    this.transform
      .updateRenderPosition(
        deltaSeconds,
      );
  }

  public setRenderPosition(
    position: NetPosition,
  ): void {
    this.transform
      .setRenderPosition(
        position,
      );
  }

  public snapRenderToAuthoritative(): void {
    this.transform
      .snapRenderToAuthoritative();
  }

  public toSnapshot(): EntitySnapshot {
    return {
      net_id:
        this.netId,

      prototype:
        this.prototype,

      position: {
        x:
          this.authoritativePosition.x,

        y:
          this.authoritativePosition.y,

        z:
          this.authoritativePosition.z,
      },
    };
  }

  private get networkIdentity():
    NetworkIdentityComponent {
    return this.requireComponent<
      NetworkIdentityComponent
    >(
      NetworkIdentityComponent.type,
    );
  }

  private get transform():
    TransformComponent {
    return this.requireComponent<
      TransformComponent
    >(
      TransformComponent.type,
    );
  }
}
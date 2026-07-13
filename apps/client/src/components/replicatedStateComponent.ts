import type { ClientComponent } from './clientComponent';
import type {
  ComponentSnapshot,
  EntitySnapshot,
} from '../protocol';

export class ReplicatedStateComponent implements ClientComponent {
  public static readonly type = 'replicatedState';
  public readonly componentType = ReplicatedStateComponent.type;
  private components: ComponentSnapshot[];

  public constructor(snapshot: EntitySnapshot) {
    this.components = structuredClone(snapshot.components);
  }

  public applySnapshot(snapshot: EntitySnapshot): boolean {
    const before = JSON.stringify(this.components);
    const after = JSON.stringify(snapshot.components);
    this.components = structuredClone(snapshot.components);
    return before !== after;
  }

  public get<T extends ComponentSnapshot['component']>(
    component: T,
  ): Extract<ComponentSnapshot, { component: T }> | undefined {
    return this.components.find(
      (entry) => entry.component === component,
    ) as Extract<ComponentSnapshot, { component: T }> | undefined;
  }
}

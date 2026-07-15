import type { ClientComponent } from './clientComponent';
import type {
  EntitySnapshot,
  NetPosition,
} from '../protocol';

const INTERPOLATION_SPEED = 14;

export class TransformComponent implements ClientComponent {
  public static readonly type = 'transform';
  public readonly componentType = TransformComponent.type;
  public readonly authoritativePosition: NetPosition;
  public readonly renderPosition: NetPosition;
  public mapId: string;
  public gridId: string | null;
  public rotation: number;

  public constructor(snapshot: EntitySnapshot) {
    this.authoritativePosition = { ...snapshot.position };
    this.renderPosition = { ...snapshot.position };
    this.mapId = snapshot.map_id;
    this.gridId = snapshot.grid;
    this.rotation = snapshot.rotation;
  }

  public applySnapshot(snapshot: EntitySnapshot): boolean {
    const changed =
      this.authoritativePosition.x !== snapshot.position.x ||
      this.authoritativePosition.y !== snapshot.position.y ||
      this.authoritativePosition.z !== snapshot.position.z ||
      this.mapId !== snapshot.map_id ||
      this.gridId !== snapshot.grid ||
      this.rotation !== snapshot.rotation;
    const discontinuity =
      this.authoritativePosition.z !== snapshot.position.z ||
      this.mapId !== snapshot.map_id ||
      this.gridId !== snapshot.grid;

    Object.assign(this.authoritativePosition, snapshot.position);
    this.mapId = snapshot.map_id;
    this.gridId = snapshot.grid;
    this.rotation = snapshot.rotation;
    if (discontinuity) {
      this.snap();
    }

    return changed;
  }

  public update(deltaSeconds: number): void {
    if (this.renderPosition.z !== this.authoritativePosition.z) {
      this.snap();
      return;
    }

    const factor = 1 - Math.exp(-INTERPOLATION_SPEED * Math.min(deltaSeconds, 0.1));
    this.renderPosition.x +=
      (this.authoritativePosition.x - this.renderPosition.x) * factor;
    this.renderPosition.y +=
      (this.authoritativePosition.y - this.renderPosition.y) * factor;
  }

  public snap(): void {
    Object.assign(this.renderPosition, this.authoritativePosition);
  }
}

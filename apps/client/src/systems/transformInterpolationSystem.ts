import type { ClientWorld } from '../clientWorld';
import type { ClientSystem } from './clientSystem';

export class TransformInterpolationSystem implements ClientSystem {
  public readonly systemType = 'transformInterpolation';

  public update(world: ClientWorld, deltaSeconds: number): void {
    for (const entity of world.getEntities().values()) {
      entity.updateRenderPosition(deltaSeconds);
    }
  }
}

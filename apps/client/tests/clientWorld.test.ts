import { describe, expect, it } from 'vitest';

import { ClientWorld } from '../src/clientWorld';
import type { EntitySnapshot } from '../src/protocol';

function entity(netId: number, x = 1): EntitySnapshot {
  return {
    net_id: netId,
    prototype: 'TestEntity',
    map_id: 'test-map',
    grid: 'main',
    position: { x, y: 2, z: 0 },
    rotation: 0,
    components: [
      {
        component: 'Dynamic',
        data: { name: 'Health', state: { current: 100 } },
      },
    ],
  };
}

describe('ClientWorld replication baselines', () => {
  it('applies a full state and a matching delta', () => {
    const world = new ClientWorld();
    expect(world.applySnapshot(10, [entity(1)])).toEqual({
      created: 1,
      updated: 0,
      removed: 0,
    });

    const result = world.applyDelta(11, 10, [entity(2)], [entity(1, 3)], []);
    expect(result.baselineMatched).toBe(true);
    expect(world.getServerTick()).toBe(11);
    expect(world.getEntity(1)?.position.x).toBe(3);
    expect(world.getEntity(2)).toBeDefined();
  });

  it('rejects a delta whose baseline was not applied', () => {
    const world = new ClientWorld();
    world.applySnapshot(10, [entity(1)]);

    const result = world.applyDelta(12, 11, [], [entity(1, 5)], []);
    expect(result.baselineMatched).toBe(false);
    expect(world.getServerTick()).toBe(10);
    expect(world.getEntity(1)?.position.x).toBe(1);
  });

  it('rejects updates for unknown entities and waits for a full state', () => {
    const world = new ClientWorld();
    world.applySnapshot(10, [entity(1)]);

    const result = world.applyDelta(11, 10, [], [entity(999)], []);
    expect(result.baselineMatched).toBe(false);
    expect(world.getServerTick()).toBe(10);
  });
});

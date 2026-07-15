import { describe, expect, it } from 'vitest';
import { CommandBuffer, GameWorldView, executeBehavior } from '../src/index.js';

describe('GameWorldView', () => {
  it('applies full and incremental world deltas', () => {
    const world = new GameWorldView();
    world.apply({
      full: true,
      upserts: [
        { entity: 1, prototype: 'Player', components: { Health: { current: 100 } } },
        { entity: 2, prototype: 'Door', components: { Door: { open: false } } },
      ],
      removals: [],
    });
    expect(world.entities()).toHaveLength(2);
    expect(world.hasComponent(2, 'Door')).toBe(true);

    world.apply({
      full: false,
      upserts: [{ entity: 2, prototype: 'Door', components: { Door: { open: true } } }],
      removals: [1],
    });
    expect(world.get(1)).toBeUndefined();
    expect(world.getComponent(2, 'Door')).toEqual({ open: true });
  });
});

describe('CommandBuffer', () => {
  it('records validated command-shaped values without mutating the world', () => {
    const commands = new CommandBuffer();
    commands.spawn('Item', { x: 1, y: 2, z: 0 });
    commands.setComponent(5, 'Health', { current: 42 });
    commands.openUi(1, 5, 'status', { tab: 'main' });
    expect(commands.commands).toHaveLength(3);
    expect(commands.commands[0]).toEqual({
      command: 'Spawn',
      data: { prototype: 'Item', x: 1, y: 2, z: 0 },
    });
  });
});

describe('behavior runtime', () => {
  it('executes deterministic command nodes', async () => {
    const commands = new CommandBuffer();
    await executeBehavior(
      {
        id: 'test',
        events: {
          use: [
            { node: 'SetComponent', entity: '$self', component: 'Door', state: { open: true } },
            { node: 'EmitSystemMessage', text: 'opened' },
          ],
        },
      },
      'use',
      { self: 7, event: {}, state: {}, commands },
    );
    expect(commands.commands).toEqual([
      { command: 'SetComponent', data: { entity: 7, component: 'Door', state: { open: true } } },
      { command: 'EmitSystemMessage', data: { text: 'opened' } },
    ]);
  });
});

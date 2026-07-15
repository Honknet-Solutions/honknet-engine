import { createInterface } from 'node:readline';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { CommandBuffer, GameWorldView, type ServerGameModule } from '@honknet/server';
import type { GameEvent, ScriptWorldDelta } from '@honknet/shared';

type EngineMessage =
  | { type: 'Initialize'; data: { engine_version: string; module_path: string } }
  | { type: 'Tick'; data: { tick: number; delta_seconds: number; events: GameEvent[]; world: ScriptWorldDelta } }
  | { type: 'Shutdown' };

let module: ServerGameModule | null = null;
const world = new GameWorldView();

const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) {
  if (!line.trim()) continue;
  try {
    const message = JSON.parse(line) as EngineMessage;
    if (message.type === 'Initialize') {
      const moduleUrl = pathToFileURL(resolve(message.data.module_path)).href;
      const imported = await import(moduleUrl) as { default?: ServerGameModule; gameModule?: ServerGameModule };
      module = imported.default ?? imported.gameModule ?? null;
      if (!module) throw new Error('Game module must export default or gameModule');
      const commands = new CommandBuffer();
      await module.initialize?.(commands);
      write({ type: 'Ready', data: { module_id: module.id } });
      continue;
    }
    if (message.type === 'Tick') {
      if (!module) throw new Error('Script host is not initialized');
      world.apply(message.data.world);
      const commands = new CommandBuffer();
      await module.tick({
        tick: message.data.tick,
        deltaSeconds: message.data.delta_seconds,
        events: message.data.events,
        world,
        commands,
      });
      write({ type: 'TickResult', data: { tick: message.data.tick, commands: commands.commands } });
      continue;
    }
    if (message.type === 'Shutdown') {
      await module?.shutdown?.();
      write({ type: 'Log', data: { level: 'info', message: 'Script host stopped' } });
      process.exit(0);
    }
  } catch (error) {
    write({
      type: 'Error',
      data: {
        message: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined,
      },
    });
  }
}

function write(message: unknown): void {
  process.stdout.write(`${JSON.stringify(message)}\n`);
}

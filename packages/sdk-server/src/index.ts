import type { EntityId, GameEvent, JsonValue, Vec3 } from '@honknet/shared';

export type ScriptCommand =
  | { command: 'Log'; data: { level: string; message: string } }
  | { command: 'EmitSystemMessage'; data: { text: string } }
  | { command: 'Spawn'; data: { prototype: string; x: number; y: number; z: number } }
  | { command: 'Delete'; data: { entity: EntityId } }
  | { command: 'EmitEvent'; data: { name: string; entity?: EntityId; payload: JsonValue } }
  | { command: 'SetComponent'; data: { entity: EntityId; component: string; state: JsonValue } }
  | { command: 'RemoveComponent'; data: { entity: EntityId; component: string } }
  | { command: 'OpenUi'; data: { player: EntityId; target: EntityId; key: string; state: JsonValue } }
  | { command: 'PlaySound'; data: { path: string; x: number; y: number; z: number } };

export class CommandBuffer {
  readonly commands: ScriptCommand[] = [];

  log(level: string, message: string): void {
    this.commands.push({ command: 'Log', data: { level, message } });
  }

  systemMessage(text: string): void {
    this.commands.push({ command: 'EmitSystemMessage', data: { text } });
  }

  spawn(prototype: string, position: Vec3): void {
    this.commands.push({ command: 'Spawn', data: { prototype, ...position } });
  }

  delete(entity: EntityId): void {
    this.commands.push({ command: 'Delete', data: { entity } });
  }

  setComponent(entity: EntityId, component: string, state: JsonValue): void {
    this.commands.push({ command: 'SetComponent', data: { entity, component, state } });
  }

  removeComponent(entity: EntityId, component: string): void {
    this.commands.push({ command: 'RemoveComponent', data: { entity, component } });
  }

  openUi(player: EntityId, target: EntityId, key: string, state: JsonValue): void {
    this.commands.push({ command: 'OpenUi', data: { player, target, key, state } });
  }

  emitEvent(name: string, entity: EntityId | undefined, payload: JsonValue): void {
    this.commands.push({ command: 'EmitEvent', data: { name, entity, payload } });
  }

  playSound(path: string, position: Vec3): void {
    this.commands.push({ command: 'PlaySound', data: { path, ...position } });
  }
}

export type ServerTickContext = Readonly<{
  tick: number;
  deltaSeconds: number;
  events: readonly GameEvent[];
  commands: CommandBuffer;
}>;

export type ServerGameModule = Readonly<{
  id: string;
  initialize?: (commands: CommandBuffer) => void | Promise<void>;
  tick: (context: ServerTickContext) => void | Promise<void>;
  shutdown?: () => void | Promise<void>;
}>;

export function defineGameModule(module: ServerGameModule): ServerGameModule {
  return module;
}

export * from './behavior.js';

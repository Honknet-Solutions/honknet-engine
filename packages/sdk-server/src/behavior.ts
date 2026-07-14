import type { EntityId, JsonValue } from '@honknet/shared';
import type { CommandBuffer } from './index.js';

export type BehaviorGraph = Readonly<{
  id: string;
  events: Readonly<Record<string, readonly BehaviorNode[]>>;
}>;

export type BehaviorNode = Readonly<{
  node: string;
  [key: string]: unknown;
}>;

export type BehaviorContext = Readonly<{
  self: EntityId;
  event: Readonly<Record<string, unknown>>;
  state: Readonly<Record<string, unknown>>;
  commands: CommandBuffer;
  readComponent?: (entity: EntityId, component: string) => JsonValue | undefined;
}>;

export async function executeBehavior(
  graph: BehaviorGraph,
  eventName: string,
  context: BehaviorContext,
): Promise<void> {
  const nodes = graph.events[eventName] ?? [];
  await executeNodes(nodes, context);
}

async function executeNodes(
  nodes: readonly BehaviorNode[],
  context: BehaviorContext,
): Promise<void> {
  for (const node of nodes) {
    await executeNode(node, context);
  }
}

async function executeNode(node: BehaviorNode, context: BehaviorContext): Promise<void> {
  switch (node.node) {
    case 'Sequence':
      await executeNodes(asNodes(node.children), context);
      return;

    case 'Branch': {
      const condition = asNode(node.condition);
      const result = condition ? evaluateCondition(condition, context) : false;
      await executeNodes(asNodes(result ? node.success : node.failure), context);
      return;
    }

    case 'Log':
      context.commands.log(String(node.level ?? 'info'), String(resolve(node.message, context) ?? ''));
      return;

    case 'EmitSystemMessage':
      context.commands.systemMessage(String(resolve(node.text, context) ?? ''));
      return;

    case 'SetComponent': {
      const entity = entityValue(node.entity, context);
      const component = String(node.component ?? '');
      const state = resolve(node.state, context) as JsonValue;
      context.commands.setComponent(entity, component, state);
      return;
    }

    case 'RemoveComponent':
      context.commands.removeComponent(
        entityValue(node.entity, context),
        String(node.component ?? ''),
      );
      return;

    case 'ToggleDoor': {
      const entity = entityValue(node.entity, context);
      const current = context.readComponent?.(entity, 'Door');
      const open = Boolean(
        current &&
        typeof current === 'object' &&
        !Array.isArray(current) &&
        (current as Record<string, JsonValue>).open,
      );
      context.commands.setComponent(entity, 'Door', { open: !open });
      return;
    }

    case 'Spawn':
      context.commands.spawn(String(node.prototype ?? ''), {
        x: numberValue(node.x, context),
        y: numberValue(node.y, context),
        z: numberValue(node.z ?? 0, context),
      });
      return;

    case 'Delete':
      context.commands.delete(entityValue(node.entity, context));
      return;

    case 'EmitEvent':
      context.commands.emitEvent(
        String(node.name ?? ''),
        optionalEntityValue(node.entity, context),
        (resolve(node.payload, context) ?? null) as JsonValue,
      );
      return;

    case 'OpenUi':
      context.commands.openUi(
        entityValue(node.player, context),
        entityValue(node.target ?? '$self', context),
        String(node.key ?? ''),
        (resolve(node.state, context) ?? null) as JsonValue,
      );
      return;

    case 'PlaySound':
      context.commands.playSound(String(node.path ?? ''), {
        x: numberValue(node.x, context),
        y: numberValue(node.y, context),
        z: numberValue(node.z ?? 0, context),
      });
      return;

    case 'Delay': {
      const milliseconds = Math.max(0, numberValue(node.milliseconds ?? 0, context));
      await new Promise<void>((resolvePromise) => setTimeout(resolvePromise, milliseconds));
      return;
    }

    default:
      throw new Error(`Unsupported behavior node ${node.node}`);
  }
}

function evaluateCondition(node: BehaviorNode, context: BehaviorContext): boolean {
  switch (node.node) {
    case 'Compare': {
      const left = resolve(node.left, context);
      const right = resolve(node.right, context);
      switch (node.operator) {
        case '==': return left === right;
        case '!=': return left !== right;
        case '>': return Number(left) > Number(right);
        case '>=': return Number(left) >= Number(right);
        case '<': return Number(left) < Number(right);
        case '<=': return Number(left) <= Number(right);
        default: return false;
      }
    }
    case 'HasComponent': {
      const entity = entityValue(node.entity, context);
      const component = String(node.component ?? '');
      return context.readComponent?.(entity, component) !== undefined;
    }
    case 'And':
      return asNodes(node.conditions).every((condition) => evaluateCondition(condition, context));
    case 'Or':
      return asNodes(node.conditions).some((condition) => evaluateCondition(condition, context));
    case 'Not': {
      const condition = asNode(node.condition);
      return condition ? !evaluateCondition(condition, context) : true;
    }
    case 'Chance': {
      const chance = Math.min(1, Math.max(0, numberValue(node.value, context)));
      return Math.random() < chance;
    }
    default:
      return Boolean(resolve(node.value, context));
  }
}

function resolve(value: unknown, context: BehaviorContext): unknown {
  if (typeof value === 'string' && value.startsWith('$')) {
    if (value === '$self') return context.self;
    if (value.startsWith('$event.')) return readPath(context.event, value.slice(7));
    if (value.startsWith('$state.')) return readPath(context.state, value.slice(7));
  }
  if (Array.isArray(value)) return value.map((entry) => resolve(entry, context));
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, resolve(entry, context)]),
    );
  }
  return value;
}

function readPath(value: unknown, path: string): unknown {
  let current = value;
  for (const segment of path.split('.')) {
    if (!current || typeof current !== 'object') return undefined;
    current = (current as Record<string, unknown>)[segment];
  }
  return current;
}

function asNode(value: unknown): BehaviorNode | undefined {
  return value && typeof value === 'object' && typeof (value as { node?: unknown }).node === 'string'
    ? value as BehaviorNode
    : undefined;
}

function asNodes(value: unknown): readonly BehaviorNode[] {
  return Array.isArray(value) ? value.map(asNode).filter((entry): entry is BehaviorNode => entry !== undefined) : [];
}

function entityValue(value: unknown, context: BehaviorContext): EntityId {
  const resolved = resolve(value ?? '$self', context);
  const entity = Number(resolved);
  if (!Number.isSafeInteger(entity) || entity < 0) {
    throw new Error(`Invalid entity value ${String(resolved)}`);
  }
  return entity;
}

function optionalEntityValue(value: unknown, context: BehaviorContext): EntityId | undefined {
  return value == null ? undefined : entityValue(value, context);
}

function numberValue(value: unknown, context: BehaviorContext): number {
  const number = Number(resolve(value, context));
  if (!Number.isFinite(number)) throw new Error(`Invalid numeric value ${String(value)}`);
  return number;
}

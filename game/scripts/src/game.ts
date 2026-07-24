type Entity = {
    index: number;
    generation: number;
};

type GameApi = Readonly<{
    build_version: string;
    capabilities: readonly string[];
}>;

type ScriptEvent = Readonly<{
    name: string;
    target?: Entity;
    payload: unknown;
}>;

type EntitySnapshot = Readonly<{
    entity: Entity;
    components: Readonly<Record<string, unknown>>;
}>;

type RelationSnapshot = Readonly<{
    kind: string;
    source: Entity;
    target: Entity;
}>;

type WorldSnapshot = Readonly<{
    entities: readonly EntitySnapshot[];
    relations: readonly RelationSnapshot[];
}>;

type EventContext = Readonly<{
    event: ScriptEvent;
    world: WorldSnapshot;
}>;

type TickContext = Readonly<{
    tick: number;
    dt: number;
    world: WorldSnapshot;
}>;

type SignalTarget =
    | { type: "global" }
    | { type: "entity"; entity: Entity }
    | { type: "component"; entity: Entity; component: string };

type SignalContext = {
    id: string;
    target: SignalTarget;
    payload: unknown;
    cancellable: boolean;
    cancelled: boolean;
    propagation_stopped: boolean;
};

type ScriptCommand =
    | { type: "log"; level: "debug" | "info" | "warn" | "error"; message: string }
    | { type: "despawn"; entity: Entity }
    | { type: "setComponent"; entity: Entity; component: string; value: unknown }
    | { type: "removeComponent"; entity: Entity; component: string }
    | { type: "addRelation"; kind: string; source: Entity; target: Entity }
    | { type: "removeRelation"; kind: string; source: Entity; target: Entity };

type SignalResult = {
    commands: ScriptCommand[];
    signal: SignalContext;
};

type GameLifecycle = Readonly<{
    initialize(api: GameApi): ScriptCommand[];
    dispatchEvent(context: EventContext): ScriptCommand[];
    dispatchSignal(signal: SignalContext): SignalResult;
    update(context: TickContext): ScriptCommand[];
    shutdown(): ScriptCommand[];
}>;

type SignalHandler = (signal: SignalContext) => ScriptCommand[];
type SignalSubscription = {
    priority: number;
    sequence: number;
    handler: SignalHandler;
};

let currentTick = 0;
let nextSignalSequence = 0;
const signalSubscriptions = new Map<string, SignalSubscription[]>();

function subscribeSignal(
    id: string,
    priority: number,
    handler: SignalHandler,
): void {
    const subscriptions = signalSubscriptions.get(id) ?? [];
    subscriptions.push({ priority, sequence: nextSignalSequence++, handler });
    subscriptions.sort(
        (left, right) =>
            right.priority - left.priority || left.sequence - right.sequence,
    );
    signalSubscriptions.set(id, subscriptions);
}

subscribeSignal("game.damageAttempt", 100, (signal) => {
    if (
        typeof signal.payload !== "object" ||
        signal.payload === null ||
        !("blocked" in signal.payload) ||
        signal.payload.blocked !== true
    ) {
        return [];
    }
    if (signal.cancellable) {
        signal.cancelled = true;
    }
    signal.propagation_stopped = true;
    return [];
});

subscribeSignal("game.damageAttempt", 0, (signal) => {
    if (
        typeof signal.payload === "object" &&
        signal.payload !== null &&
        "damage" in signal.payload &&
        typeof signal.payload.damage === "number"
    ) {
        signal.payload.damage = Math.max(0, signal.payload.damage);
    }
    return [];
});

(globalThis as typeof globalThis & { Game: GameLifecycle }).Game = Object.freeze({
    initialize(api: GameApi): ScriptCommand[] {
        Object.freeze(api);
        return [{
            type: "log",
            level: "info",
            message: `Honknet game script ${api.build_version} initialized`,
        }];
    },

    dispatchEvent(context: EventContext): ScriptCommand[] {
        const event = context.event;
        if (event.name === "despawnRequested" && event.target) {
            return [{ type: "despawn", entity: event.target }];
        }
        if (event.name === "setStatusRequested" && event.target) {
            return [{
                type: "setComponent",
                entity: event.target,
                component: "game.status",
                value: event.payload,
            }];
        }
        return [];
    },

    dispatchSignal(signal: SignalContext): SignalResult {
        const commands: ScriptCommand[] = [];
        for (const subscription of signalSubscriptions.get(signal.id) ?? []) {
            commands.push(...subscription.handler(signal));
            if (signal.propagation_stopped) {
                break;
            }
        }
        return { commands, signal };
    },

    update(context: TickContext): ScriptCommand[] {
        currentTick = context.tick;
        return [];
    },

    shutdown(): ScriptCommand[] {
        return [{
            type: "log",
            level: "debug",
            message: `Game script stopped at tick ${currentTick}`,
        }];
    },
});

export type EntityId = number;
export type Tick = number;
export type Vec2 = Readonly<{ x: number; y: number }>;
export type Vec3 = Readonly<{ x: number; y: number; z: number }>;
export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { readonly [key: string]: JsonValue };

export type ComponentState = Readonly<Record<string, JsonValue>>;

export type GameEvent<TPayload extends JsonValue = JsonValue> = Readonly<{
  name: string;
  entity?: EntityId;
  payload: TPayload;
}>;

export type UiSessionState = Readonly<{
  sessionId: string;
  key: string;
  target: EntityId;
  state: JsonValue;
}>;

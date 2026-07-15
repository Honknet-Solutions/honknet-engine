# Content and game-module API

## Responsibility boundary

Rust owns authoritative engine state. TypeScript receives an immutable world view and returns commands. A command never mutates engine memory directly; Rust validates the entity, component, values, resource paths, state size and ownership before applying it.

## Prototype spawning

Every non-abstract YAML entity prototype can be spawned without adding a Rust `spawn_*` function. Native components are parsed by the engine; custom components require a component schema.

```yaml
- type: entity
  id: ExampleMedkit
  parent: BaseItem
  name: example-medkit-name
  components:
    - type: Item
      size: Small
    - type: Health
      current: 100
      maximum: 100
```

Unknown custom components are rejected. This prevents misspelled component names from silently entering a save or network state.

## Custom component schemas

```yaml
type: component-schema
id: Health
replication:
  mode: server-to-client
fields:
  current:
    type: number
    default: 100
    minimum: 0
  maximum:
    type: number
    default: 100
    minimum: 1
```

Replication modes:

- `none`: server/script only;
- `server-to-client`: visible to clients in PVS;
- `owner-only`: visible only to the owning player.

The build pipeline generates TypeScript declarations from schemas. Schema changes should be treated as API changes and accompanied by save/content migrations.

## Script world view

Each TypeScript tick receives entities keyed by network entity ID, prototype and component state. Native components are normalized to JSON objects; custom components use their schema-normalized state.

The game module emits commands through `CommandBuffer`, including spawn, delete, component changes, messages, UI, sound and custom events. The engine applies command and payload limits before mutation.

## Native component mutation policy

Script code may safely update selected fields:

- `Transform`: position, z, rotation and an existing grid ID;
- `Player`: display name only;
- `Sprite`: validated layers and resource paths;
- `Collider`: radius, layer, mask and sensor flag;
- `Door`: open state;
- `Inventory`: capacity;
- `Item`: display name and size.

`NetworkIdentity` is immutable. Required identity/transform/player components cannot be removed. `PhysicsBody` remains reserved for the native physics runtime.

## UI sessions

A script opens an HUI document for a player and target. The server creates an opaque session ID and records ownership. Client actions are accepted only from the owning player and are returned to TypeScript as `ui.action` events. Payloads and action/session identifiers are bounded.

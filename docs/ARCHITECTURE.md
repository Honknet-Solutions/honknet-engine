# Architecture

Honknet is one fixed game product. It keeps engine subsystems in separate
Rust crates for maintainability, but there is no runtime game selection, external game module,
or configurable content project.

`honknet-server` constructs `GameApplication`, which registers the fixed gameplay
components, loads the bundled content and selected built-in map, starts the round, and then
accepts clients. The authoritative server owns gameplay state. The browser client shares the
same build, content and protocol identity. Structural ECS changes are deferred through command
buffers; replication works from immutable state frames.

Game rules execute from the single bundled TypeScript program in `game/scripts`. The server
runs its compiled JavaScript in an isolated worker with deterministic time, bounded loops,
recursion, stack and buffers, and no Node, filesystem, socket, process or timer APIs. Scripts
cannot borrow ECS memory. They receive serialized events and tick contexts, then return typed
world commands that `GameApplication` applies through an ECS command buffer.

Mutable signals provide cancellable, targeted hooks for global, entity and component-level
gameplay decisions. Native subscribers run first, ordered by descending priority and then
registration order. Unless propagation was stopped, the detached signal is passed to TypeScript,
whose subscribers use the same ordering rule. Payload changes are copied back, while the signal
ID, target and cancellable flag remain native-owned. Cancellation and propagation stopping are
separate, monotonic states. Authoritative interaction, access, damage and healing actions enter
through `GameApplication`; each emits its corresponding `game.*Attempt` signal before changing
ECS state. Legacy world-level helpers remain deterministic primitives and do not dispatch scripts.

Clients submit typed `GameActionRequestPayload` messages for interact, attack, pickup and drop.
The server derives the actor from the authenticated peer, rejects stale or out-of-order sequences,
bounds the shared action queue, validates target lifetime and authoritative physics distance, and
enforces tick-based cooldowns. Accepted actions execute at the start of the next game tick through
the signal-aware `GameApplication` methods. A typed result reports success, cancellation, denial,
invalid target, range, cooldown, duplicate or queue overflow before the resulting state snapshot.
The browser WASM bridge encodes these shared Rust protocol types and retains pending sequences
until their results arrive. The Pixi client selects the nearest rendered entity under the cursor;
keys 1/2/3 choose interact, attack or pickup, Shift-click attacks, and Q drops the held item.
Medical actions use the same protocol: B begins bandaging and C begins CPR on the entity under
the cursor.

Script-defined state uses dynamic ECS components with stable, registered identifiers such as
`game.status`. Entity relations use the same registration rules for kinds such as
`game.parent`, `game.containedIn`, `game.equippedTo` and `game.attachedTo`. Each script call
receives a detached world snapshot containing dynamic component values and relations. Despawning
an entity removes its dynamic state and every incoming or outgoing relation.

Spawned players own a body graph made from separate body-part entities. Local damage resolves
against the selected target zone and creates a wound entity attached to that part. Open cuts
contribute continuous blood loss during physiology processing; blood-volume thresholds currently
drive the authoritative mob state. `HealthComponent` is retained only as a compatibility projection
for systems that have not yet migrated to physiology.
The graph also contains brain, heart and lung entities. Functional lungs and active breathing
restore blood oxygen; absent breathing, lung failure or heart failure lowers saturation. Wounds
produce pain, while pain, hypoxia and blood loss combine into shock and consciousness. Sustained
severe hypoxia or brain failure is fatal. Bandaging a wound stops its continuous blood loss.
Bandaging and CPR are server-owned `do_after` actions rather than instant mutations. Their result
is withheld until the required tick duration completes, and the action is cancelled if either
participant moves too far, disappears, or the acting character becomes incapacitated. A completed
CPR pulse raises blood oxygen and reduces accumulated hypoxia but cannot compensate for fatal
blood loss.
The underlying tick timer queue is generic: callbacks are ordered by due tick and registration
sequence, have stable cancellation IDs, and may repeat at deterministic intervals. Medical
`do_after` entries use those timer IDs while retaining ECS state for interruption and progress.
Wound treatment is type-specific: gauze closes cuts, bruise packs reduce blunt wounds, and burn gel
reduces burns. The server requires the matching charged supply to remain in the actor's hand and
consumes one charge only after a successful completion.

Physical character interactions are authoritative ECS state. A first grab is passive and a second
grab on the same target becomes aggressive. Pulling requires an active grab and breaks beyond its
maximum distance. Carrying requires an aggressive grab, an incapacitated target and a completed
interruptible action; carried bodies follow their carrier and reduce carrier movement speed.
Buckle fixtures enforce capacity, keep occupants at the fixture transform and prevent independent
movement until unbuckled. Equipment validates wearable slot compatibility, while containers account
for item size rather than item count.

The runtime is the single source of replicated entity state; the server no longer reconstructs a
second manual snapshot. Game state extends each runtime entity with mob, hands, equipment, medical
and interaction components. Private HUD components use owner-only replication, snapshots remove
unauthorized fields before budget calculation, and snapshot history is keyed by client and tick so
one client's baseline cannot be reused for another. The browser client decodes this state into its
local ECS and displays blood, oxygen, pain, shock, consciousness, grabs and timed actions.
Entity updates contain only components whose serialized state changed relative to the acknowledged
per-client baseline. Leaving PVS produces a client despawn even when the entity remains alive on the
server. Input acknowledgements are owner-only state; reconciliation resets the controlled entity to
the authoritative transform and replays only commands newer than the acknowledged sequence.

Maps support explicit Z-levels, nested and moving grids, local/world coordinate conversion, area
entities, bidirectional level transitions and sealed docking ports. Atmosphere topology is expressed
as entity connections whose conductance may be blocked by a door. Gas exchange conserves species,
space acts as a sink, breathing consumes environmental oxygen and produces carbon dioxide, and the
result feeds the physiology system. Power consumers, APCs and SMES units are grouped by network and
channel; stored energy is conserved across multiple APCs. Powered doors additionally enforce bolts,
access and safe pressure differential before opening.

Chemicals metabolize through bloodstream holders and affect local wounds, burns, toxin load, shock
and stabilization rather than mutating compatibility HP. Surgery uses a server-authoritative ordered
tool sequence (incision, haemostasis, retraction, repair and closure), with every step represented by
the same interruptible timed-action system as other medical work.

The round controller has explicit lobby, countdown, active, ending and ended phases. Ready state and
job preferences are server-owned, job capacities are assigned deterministically, and assignments
create the character job plus an access-bearing ID card. Lobby state is a protocol message rendered
by the browser client. Production server startup enables health/metrics HTTP endpoints, persistence
journaling and replay recording; graceful shutdown finalizes both. The optional remote admin endpoint
requires a server-side token and routes permission-checked commands through the main tick.

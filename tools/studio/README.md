# Honknet Studio Suite — Visual Development Tools

Honknet Studio is the official integrated development environment and visual tool suite for **Honknet Engine 1.0** as specified in `.agent/HONKNET_STUDIO_1.0_DESIGN_SPEC.md`.

## Included Tool Suite

1. **Project & Build Manager** (`tools/studio` / `project-manager`):
   - Workspace & `game.toml` manifest inspector.
   - Dependency & `honknet.lock` status checker.
   - Release Target Exporter (Server, Desktop Client, Web WASM/WebGPU, Docker).

2. **Map & Grid Editor** (`tools/map-editor`):
   - Tile Palette (Pencil, Eraser, Line, Rect, Fill, Stamp, Eyedropper).
   - Chunk inspection & Grid controls (collision, render, nav dirty states).
   - Real engine map serializer format export (`honknet-map`).

3. **UI / HUI Retained Editor** (`tools/ui-editor`):
   - Retained HUI document tree editor (Window, Panel, Label, Button, VirtualList, Viewport).
   - Flex/Grid layout properties, theme & binding inspector (`$state.property`).
   - Interactive live WASM HUI preview stage.

4. **Prototype & Schema Editor** (`tools/prototype-editor`):
   - Schema-driven component forms (Transform, Physics, Health, SpriteState, Networked, Predicted).
   - Multiple inheritance graph visualizer (`SecurityOfficer` -> `BaseCharacter` -> `BaseEntity`).
   - Component descriptor validation.

5. **Profiler & Timings Inspector** (`tools/profiler`):
   - Real-time TPS and Frame time duration ticker.
   - ECS System Execution Timings bar charts (Physics, Replication, PVS, Movement).
   - Memory RSS & GPU texture usage metrics.

6. **Network Inspector & PVS** (`tools/network-inspector`):
   - Live Binary Packet Inspector stream.
   - PVS (Potentially Visible Set) spatial visibility map & byte budget stats.

7. **Migration Assistant** (`tools/migration-assistant`):
   - Schema version checker and automated tile ID / component migration wizard.

# Architecture

Honknet is one fixed game product. It keeps engine subsystems in separate
Rust crates for maintainability, but there is no runtime game selection, external game module,
or configurable content project.

`honknet-server` constructs `GameApplication`, which registers the fixed gameplay
components, loads the bundled content and selected built-in map, starts the round, and then
accepts clients. The authoritative server owns gameplay state. The browser client shares the
same build, content and protocol identity. Structural ECS changes are deferred through command
buffers; replication works from immutable state frames.

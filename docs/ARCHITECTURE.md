# Architecture

Honknet separates core crates, engine subsystems, runtime applications and game SDK. The authoritative server owns gameplay state. Desktop and web clients share binary protocol semantics, interpolation and prediction. Structural ECS changes are deferred through command buffers; replication works from immutable state frames.

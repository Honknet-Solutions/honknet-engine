//! Setting-agnostic ECS primitives for Honknet.

mod components;
mod entity;
mod spatial;
mod system;
mod world;

pub use components::{NetworkIdentity, PrototypeRef, Transform};
pub use entity::{Component, EntityId};
pub use spatial::{SpatialHash, SpatialKey};
pub use system::{System, SystemManager};
pub use world::{EntityRef, World, WorldError};

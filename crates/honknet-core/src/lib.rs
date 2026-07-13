//! Setting-agnostic ECS primitives for Honknet.

mod components;
mod entity;
mod system;
mod world;

pub use components::{NetworkIdentity, PrototypeRef, Transform};
pub use entity::{Component, Entity, EntityId};
pub use system::{System, SystemManager};
pub use world::{World, WorldError};

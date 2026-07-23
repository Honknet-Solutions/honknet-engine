use honknet_core::Entity;
use honknet_math::Transform2;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformNode {
    pub local: Transform2,
    pub world: Transform2,
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
    pub map: u32,
    pub grid: Option<Entity>,
    pub anchored: bool,
    pub dirty: bool,
}

impl Default for TransformNode {
    fn default() -> Self {
        Self {
            local: Default::default(),
            world: Default::default(),
            parent: None,
            children: vec![],
            map: 0,
            grid: None,
            anchored: false,
            dirty: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum TransformError {
    #[error("node missing")]
    Missing,
    #[error("parent cycle")]
    Cycle,
}

#[derive(Default)]
pub struct TransformGraph {
    nodes: HashMap<Entity, TransformNode>,
}

impl TransformGraph {
    pub fn insert(&mut self, e: Entity, node: TransformNode) {
        self.nodes.insert(e, node);
    }
    pub fn get(&self, e: Entity) -> Option<&TransformNode> {
        self.nodes.get(&e)
    }
    pub fn get_mut(&mut self, e: Entity) -> Option<&mut TransformNode> {
        self.nodes.get_mut(&e)
    }
    pub fn set_parent(&mut self, e: Entity, parent: Option<Entity>) -> Result<(), TransformError> {
        if !self.nodes.contains_key(&e) {
            return Err(TransformError::Missing);
        }
        if let Some(p) = parent {
            let mut cur = Some(p);
            let mut seen = HashSet::new();
            while let Some(x) = cur {
                if x == e || !seen.insert(x) {
                    return Err(TransformError::Cycle);
                }
                cur = self.nodes.get(&x).and_then(|n| n.parent)
            }
        }
        let old = self.nodes[&e].parent;
        if let Some(o) = old {
            if let Some(n) = self.nodes.get_mut(&o) {
                n.children.retain(|x| *x != e)
            }
        }
        if let Some(p) = parent {
            self.nodes
                .get_mut(&p)
                .ok_or(TransformError::Missing)?
                .children
                .push(e)
        }
        self.nodes.get_mut(&e).unwrap().parent = parent;
        self.mark_dirty(e);
        Ok(())
    }
    pub fn mark_dirty(&mut self, e: Entity) {
        let children = self
            .nodes
            .get(&e)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        if let Some(n) = self.nodes.get_mut(&e) {
            n.dirty = true
        }
        for c in children {
            self.mark_dirty(c)
        }
    }
    pub fn update_world(&mut self) {
        let roots: Vec<_> = self
            .nodes
            .iter()
            .filter_map(|(e, n)| n.parent.is_none().then_some(*e))
            .collect();
        for r in roots {
            self.update_branch(r, Transform2::default())
        }
    }
    fn update_branch(&mut self, e: Entity, parent: Transform2) {
        let (children, world) = {
            let n = self.nodes.get_mut(&e).unwrap();
            if n.dirty {
                n.world = parent.combine(n.local);
                n.dirty = false
            }
            (n.children.clone(), n.world)
        };
        for c in children {
            self.update_branch(c, world)
        }
    }
    pub fn remove(&mut self, e: Entity) {
        if let Some(n) = self.nodes.remove(&e) {
            if let Some(p) = n.parent {
                if let Some(x) = self.nodes.get_mut(&p) {
                    x.children.retain(|c| *c != e)
                }
            }
            for c in n.children {
                if let Some(x) = self.nodes.get_mut(&c) {
                    x.parent = None;
                    x.dirty = true
                }
            }
        }
    }
}

#[derive(Default)]
pub struct ContainerManager {
    containers: HashMap<(Entity, String), Vec<Entity>>,
    contained: HashMap<Entity, (Entity, String)>,
}

impl ContainerManager {
    pub fn insert(
        &mut self,
        owner: Entity,
        name: &str,
        entity: Entity,
    ) -> Result<(), TransformError> {
        if self.contained.contains_key(&entity) {
            return Err(TransformError::Cycle);
        }
        self.containers
            .entry((owner, name.into()))
            .or_default()
            .push(entity);
        self.contained.insert(entity, (owner, name.into()));
        Ok(())
    }
    pub fn remove(&mut self, entity: Entity) -> bool {
        let Some((owner, name)) = self.contained.remove(&entity) else {
            return false;
        };
        if let Some(v) = self.containers.get_mut(&(owner, name)) {
            v.retain(|x| *x != entity)
        }
        true
    }
    pub fn contents(&self, owner: Entity, name: &str) -> &[Entity] {
        self.containers
            .get(&(owner, name.into()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

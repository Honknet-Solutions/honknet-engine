use std::collections::{HashMap, HashSet};

use crate::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpatialKey {
    pub map_hash: u64,
    pub z: i32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct SpatialHash {
    cell_size: f32,
    cells: HashMap<SpatialKey, Vec<EntityId>>,
    entity_cells: HashMap<EntityId, Vec<SpatialKey>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size.is_finite() && cell_size > 0.0);
        Self {
            cell_size,
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    pub fn insert_circle(
        &mut self,
        entity: EntityId,
        map_hash: u64,
        z: i32,
        x: f32,
        y: f32,
        radius: f32,
    ) {
        let keys =
            self.keys_for_bounds(map_hash, z, x - radius, y - radius, x + radius, y + radius);
        if self
            .entity_cells
            .get(&entity)
            .is_some_and(|current| current == &keys)
        {
            return;
        }
        self.remove(entity);
        for key in &keys {
            self.cells.entry(*key).or_default().push(entity);
        }
        self.entity_cells.insert(entity, keys);
    }

    /// Removes entities that are no longer alive without rebuilding every
    /// occupied cell. Intended for the once-per-tick maintenance pass.
    pub fn retain_entities(&mut self, alive: &HashSet<EntityId>) {
        let stale = self
            .entity_cells
            .keys()
            .copied()
            .filter(|entity| !alive.contains(entity))
            .collect::<Vec<_>>();
        for entity in stale {
            self.remove(entity);
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entity_cells.len()
    }

    pub fn remove(&mut self, entity: EntityId) {
        let Some(keys) = self.entity_cells.remove(&entity) else {
            return;
        };
        for key in keys {
            if let Some(entries) = self.cells.get_mut(&key) {
                entries.retain(|candidate| *candidate != entity);
                if entries.is_empty() {
                    self.cells.remove(&key);
                }
            }
        }
    }

    pub fn query_circle(
        &self,
        map_hash: u64,
        z: i32,
        x: f32,
        y: f32,
        radius: f32,
    ) -> Vec<EntityId> {
        let mut unique = HashSet::new();
        for key in self.keys_for_bounds(map_hash, z, x - radius, y - radius, x + radius, y + radius)
        {
            if let Some(entries) = self.cells.get(&key) {
                unique.extend(entries.iter().copied());
            }
        }
        let mut result = unique.into_iter().collect::<Vec<_>>();
        result.sort_unstable_by_key(|id| id.value());
        result
    }

    fn keys_for_bounds(
        &self,
        map_hash: u64,
        z: i32,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    ) -> Vec<SpatialKey> {
        let min_cell_x = (min_x / self.cell_size).floor() as i32;
        let min_cell_y = (min_y / self.cell_size).floor() as i32;
        let max_cell_x = (max_x / self.cell_size).floor() as i32;
        let max_cell_y = (max_y / self.cell_size).floor() as i32;
        let mut keys = Vec::new();
        for y in min_cell_y..=max_cell_y {
            for x in min_cell_x..=max_cell_x {
                keys.push(SpatialKey { map_hash, z, x, y });
            }
        }
        keys
    }
}

impl Default for SpatialHash {
    fn default() -> Self {
        Self::new(8.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{EntityId, SpatialHash};

    #[test]
    fn spatial_hash_deduplicates_multicell_entities() {
        let mut index = SpatialHash::new(2.0);
        let entity = EntityId::new(1);
        index.insert_circle(entity, 7, 0, 2.0, 2.0, 1.5);
        assert_eq!(index.query_circle(7, 0, 2.0, 2.0, 3.0), vec![entity]);
        index.remove(entity);
        assert!(index.query_circle(7, 0, 2.0, 2.0, 3.0).is_empty());
    }
}

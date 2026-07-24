use honknet_core::Entity;
use honknet_math::{Aabb, Transform2, Vec2};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColliderId {
    pub map_id: u32,
    pub grid_id: u32,
    pub chunk_x: i32,
    pub chunk_y: i32,
    pub generation: u64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DirtyChunkQueue {
    pub queue: Vec<ColliderId>,
}

impl DirtyChunkQueue {
    pub fn push(&mut self, id: ColliderId) {
        if !self.queue.contains(&id) {
            self.queue.push(id);
        }
    }
    pub fn pop(&mut self) -> Option<ColliderId> {
        self.queue.pop()
    }
}
pub const CHUNK_SIZE: i32 = 32;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileDef {
    pub id: String,
    pub solid: bool,
    pub friction: f32,
    pub resource: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub position: (i32, i32),
    pub tiles: Vec<u16>,
    pub revision: u64,
    pub collision_dirty: bool,
    pub render_dirty: bool,
    pub occlusion_dirty: bool,
    pub nav_dirty: bool,
}

impl Chunk {
    pub fn new(position: (i32, i32), fill: u16) -> Self {
        Self {
            position,
            tiles: vec![fill; (CHUNK_SIZE * CHUNK_SIZE) as usize],
            revision: 0,
            collision_dirty: true,
            render_dirty: true,
            occlusion_dirty: true,
            nav_dirty: true,
        }
    }
    fn idx(x: i32, y: i32) -> usize {
        (y * CHUNK_SIZE + x) as usize
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub entity: Entity,
    pub transform: Transform2,
    #[serde(default)]
    pub z_level: i32,
    #[serde(default)]
    pub parent: Option<Entity>,
    #[serde(default)]
    pub linear_velocity: Vec2,
    pub chunks: HashMap<(i32, i32), Chunk>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    pub entity: Entity,
    pub grid: Entity,
    pub name: String,
    pub bounds: Aabb,
    pub atmosphere_zone: Option<Entity>,
    pub power_channel: Option<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelTransition {
    pub entity: Entity,
    pub source_grid: Entity,
    pub source: Vec2,
    pub destination_grid: Entity,
    pub destination: Vec2,
    pub bidirectional: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockingPort {
    pub entity: Entity,
    pub grid: Entity,
    pub local_position: Vec2,
    pub normal: Vec2,
    pub docked_to: Option<Entity>,
    pub sealed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub id: u32,
    pub tile_size: f32,
    pub tiles: Vec<TileDef>,
    pub grids: HashMap<Entity, Grid>,
    #[serde(default)]
    pub areas: HashMap<Entity, Area>,
    #[serde(default)]
    pub transitions: HashMap<Entity, LevelTransition>,
    #[serde(default)]
    pub docking_ports: HashMap<Entity, DockingPort>,
    pub metadata: HashMap<String, String>,
    pub streaming_regions: Vec<Aabb>,
    pub dirty_chunks: DirtyChunkQueue,
}

#[derive(Debug, Error)]
pub enum MapError {
    #[error("grid missing")]
    GridMissing,
    #[error("invalid tile {0}")]
    InvalidTile(u16),
    #[error("transition missing or disabled")]
    TransitionUnavailable,
    #[error("docking port missing or already occupied")]
    DockUnavailable,
    #[error("docking ports are incompatible")]
    DockIncompatible,
}

impl Map {
    pub fn grid_world_transform(&self, grid: Entity) -> Option<Transform2> {
        let mut chain = Vec::new();
        let mut current = Some(grid);
        let mut visited = HashSet::new();
        while let Some(entity) = current {
            if !visited.insert(entity) {
                return None;
            }
            let grid = self.grids.get(&entity)?;
            chain.push(grid.transform);
            current = grid.parent;
        }
        Some(
            chain
                .into_iter()
                .rev()
                .fold(Transform2::IDENTITY, Transform2::combine),
        )
    }

    pub fn grid_to_world(&self, grid: Entity, local: Vec2) -> Option<Vec2> {
        Some(
            self.grid_world_transform(grid)?
                .point(local * self.tile_size),
        )
    }

    pub fn transfer_between_grids(
        &self,
        source_grid: Entity,
        destination_grid: Entity,
        local: Vec2,
    ) -> Option<Vec2> {
        let world = self.grid_to_world(source_grid, local)?;
        Some(self.world_to_grid(destination_grid, world))
    }

    pub fn traverse_transition(
        &self,
        transition: Entity,
        from_grid: Entity,
    ) -> Result<(Entity, Vec2), MapError> {
        let transition = self
            .transitions
            .get(&transition)
            .filter(|transition| transition.enabled)
            .ok_or(MapError::TransitionUnavailable)?;
        if transition.source_grid == from_grid {
            return Ok((transition.destination_grid, transition.destination));
        }
        if transition.bidirectional && transition.destination_grid == from_grid {
            return Ok((transition.source_grid, transition.source));
        }
        Err(MapError::TransitionUnavailable)
    }

    pub fn area_at(&self, grid: Entity, local: Vec2) -> Option<&Area> {
        self.areas
            .values()
            .find(|area| area.grid == grid && area.bounds.contains(local))
    }

    pub fn dock(&mut self, first: Entity, second: Entity) -> Result<(), MapError> {
        let a = self
            .docking_ports
            .get(&first)
            .filter(|port| port.docked_to.is_none())
            .cloned()
            .ok_or(MapError::DockUnavailable)?;
        let b = self
            .docking_ports
            .get(&second)
            .filter(|port| port.docked_to.is_none())
            .cloned()
            .ok_or(MapError::DockUnavailable)?;
        if first == second || a.grid == b.grid || a.normal.dot(b.normal) > -0.9 {
            return Err(MapError::DockIncompatible);
        }
        self.docking_ports.get_mut(&first).unwrap().docked_to = Some(second);
        self.docking_ports.get_mut(&second).unwrap().docked_to = Some(first);
        Ok(())
    }

    pub fn undock(&mut self, port: Entity) -> Result<(), MapError> {
        let peer = self
            .docking_ports
            .get_mut(&port)
            .and_then(|port| port.docked_to.take())
            .ok_or(MapError::DockUnavailable)?;
        if let Some(peer_port) = self.docking_ports.get_mut(&peer) {
            peer_port.docked_to = None;
        }
        Ok(())
    }

    pub fn update_moving_grids(&mut self, delta_seconds: f32) {
        for grid in self.grids.values_mut() {
            grid.transform.translation += grid.linear_velocity * delta_seconds;
        }
    }

    pub fn set_tile(&mut self, grid: Entity, x: i32, y: i32, tile: u16) -> Result<u16, MapError> {
        if tile as usize >= self.tiles.len() {
            return Err(MapError::InvalidTile(tile));
        }
        let g = self.grids.get_mut(&grid).ok_or(MapError::GridMissing)?;
        let cx = x.div_euclid(CHUNK_SIZE);
        let cy = y.div_euclid(CHUNK_SIZE);
        let lx = x.rem_euclid(CHUNK_SIZE);
        let ly = y.rem_euclid(CHUNK_SIZE);
        let c = g
            .chunks
            .entry((cx, cy))
            .or_insert_with(|| Chunk::new((cx, cy), 0));
        let idx = Chunk::idx(lx, ly);
        let old = std::mem::replace(&mut c.tiles[idx], tile);
        c.revision += 1;
        c.collision_dirty = true;
        c.render_dirty = true;
        c.occlusion_dirty = true;
        c.nav_dirty = true;
        g.revision += 1;
        self.dirty_chunks.push(ColliderId {
            map_id: self.id,
            grid_id: grid.index,
            chunk_x: cx,
            chunk_y: cy,
            generation: c.revision,
        });
        Ok(old)
    }
    pub fn build_chunk_colliders(&self, grid: Entity, cx: i32, cy: i32) -> Vec<Aabb> {
        let mut colliders = vec![];
        let Some(g) = self.grids.get(&grid) else {
            return colliders;
        };
        let Some(chunk) = g.chunks.get(&(cx, cy)) else {
            return colliders;
        };

        let mut solid = vec![false; (CHUNK_SIZE * CHUNK_SIZE) as usize];
        for (i, &t) in chunk.tiles.iter().enumerate() {
            if let Some(def) = self.tiles.get(t as usize) {
                solid[i] = def.solid;
            }
        }

        let mut visited = vec![false; (CHUNK_SIZE * CHUNK_SIZE) as usize];

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let idx = Chunk::idx(x, y);
                if solid[idx] && !visited[idx] {
                    let mut w = 1;
                    while x + w < CHUNK_SIZE
                        && solid[Chunk::idx(x + w, y)]
                        && !visited[Chunk::idx(x + w, y)]
                    {
                        w += 1;
                    }
                    let mut h = 1;
                    let mut can_expand_y = true;
                    while y + h < CHUNK_SIZE && can_expand_y {
                        for ix in x..x + w {
                            if !solid[Chunk::idx(ix, y + h)] || visited[Chunk::idx(ix, y + h)] {
                                can_expand_y = false;
                                break;
                            }
                        }
                        if can_expand_y {
                            h += 1;
                        }
                    }
                    for iy in y..y + h {
                        for ix in x..x + w {
                            visited[Chunk::idx(ix, iy)] = true;
                        }
                    }
                    let min_x = (cx * CHUNK_SIZE + x) as f32 * self.tile_size;
                    let min_y = (cy * CHUNK_SIZE + y) as f32 * self.tile_size;
                    let max_x = (cx * CHUNK_SIZE + x + w) as f32 * self.tile_size;
                    let max_y = (cy * CHUNK_SIZE + y + h) as f32 * self.tile_size;
                    colliders.push(Aabb {
                        min: Vec2::new(min_x, min_y),
                        max: Vec2::new(max_x, max_y),
                    });
                }
            }
        }
        colliders
    }
    pub fn tile(&self, grid: Entity, x: i32, y: i32) -> Option<&TileDef> {
        let g = self.grids.get(&grid)?;
        let c = g
            .chunks
            .get(&(x.div_euclid(CHUNK_SIZE), y.div_euclid(CHUNK_SIZE)))?;
        self.tiles
            .get(c.tiles[Chunk::idx(x.rem_euclid(CHUNK_SIZE), y.rem_euclid(CHUNK_SIZE))] as usize)
    }
    pub fn solid_at(&self, grid: Entity, p: Vec2) -> bool {
        let local = self.world_to_grid(grid, p);
        self.tile(grid, local.x.floor() as i32, local.y.floor() as i32)
            .is_some_and(|t| t.solid)
    }
    pub fn world_to_grid(&self, grid: Entity, p: Vec2) -> Vec2 {
        let t = self.grid_world_transform(grid).unwrap_or_default();
        let local = (p - t.translation).rotate(-t.rotation);
        Vec2::new(local.x / t.scale.x, local.y / t.scale.y) / self.tile_size
    }
    pub fn stream_chunks(&self, area: Aabb) -> Vec<(Entity, &Chunk)> {
        let mut out = vec![];
        for (gid, g) in &self.grids {
            for c in g.chunks.values() {
                let min = g.transform.point(
                    Vec2::new(
                        (c.position.0 * CHUNK_SIZE) as f32,
                        (c.position.1 * CHUNK_SIZE) as f32,
                    ) * self.tile_size,
                );
                let a = Aabb {
                    min,
                    max: min
                        + Vec2::new(
                            (CHUNK_SIZE as f32) * self.tile_size,
                            (CHUNK_SIZE as f32) * self.tile_size,
                        ),
                };
                if a.intersects(area) {
                    out.push((*gid, c))
                }
            }
        }
        out
    }
    pub fn pathfind(
        &self,
        grid: Entity,
        start: (i32, i32),
        goal: (i32, i32),
        limit: usize,
    ) -> Option<Vec<(i32, i32)>> {
        #[derive(Copy, Clone, Eq, PartialEq)]
        struct N {
            f: i32,
            p: (i32, i32),
        }
        impl Ord for N {
            fn cmp(&self, o: &Self) -> Ordering {
                o.f.cmp(&self.f)
            }
        }
        impl PartialOrd for N {
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        let h = |p: (i32, i32)| (p.0 - goal.0).abs() + (p.1 - goal.1).abs();
        let mut open = BinaryHeap::new();
        let mut came = HashMap::new();
        let mut gscore = HashMap::from([(start, 0)]);
        let mut closed = HashSet::new();
        open.push(N {
            f: h(start),
            p: start,
        });
        while let Some(N { p, .. }) = open.pop() {
            if p == goal {
                let mut v = vec![p];
                let mut c = p;
                while let Some(n) = came.get(&c).copied() {
                    v.push(n);
                    c = n
                }
                v.reverse();
                return Some(v);
            }
            if closed.len() >= limit {
                return None;
            }
            if !closed.insert(p) {
                continue;
            }
            for n in [
                (p.0 + 1, p.1),
                (p.0 - 1, p.1),
                (p.0, p.1 + 1),
                (p.0, p.1 - 1),
            ] {
                if self.tile(grid, n.0, n.1).is_some_and(|t| t.solid) {
                    continue;
                }
                let ng = gscore[&p] + 1;
                if ng < *gscore.get(&n).unwrap_or(&i32::MAX) {
                    came.insert(n, p);
                    gscore.insert(n, ng);
                    open.push(N { f: ng + h(n), p: n })
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_map() -> Map {
        Map {
            id: 1,
            tile_size: 1.0,
            tiles: Vec::new(),
            grids: HashMap::new(),
            areas: HashMap::new(),
            transitions: HashMap::new(),
            docking_ports: HashMap::new(),
            metadata: HashMap::new(),
            streaming_regions: Vec::new(),
            dirty_chunks: DirtyChunkQueue::default(),
        }
    }

    fn grid(entity: Entity, x: f32, z_level: i32) -> Grid {
        Grid {
            entity,
            transform: Transform2 {
                translation: Vec2::new(x, 0.0),
                ..Transform2::IDENTITY
            },
            z_level,
            parent: None,
            linear_velocity: Vec2::ZERO,
            chunks: HashMap::new(),
            revision: 0,
        }
    }

    #[test]
    fn nested_grids_preserve_local_and_world_coordinates() {
        let parent = Entity::new(1, 0);
        let shuttle = Entity::new(2, 0);
        let mut map = empty_map();
        map.grids.insert(parent, grid(parent, 10.0, 0));
        let mut child = grid(shuttle, 5.0, 1);
        child.parent = Some(parent);
        map.grids.insert(shuttle, child);

        let world = map.grid_to_world(shuttle, Vec2::new(2.0, 0.0)).unwrap();
        assert_eq!(world, Vec2::new(17.0, 0.0));
        assert_eq!(map.world_to_grid(shuttle, world), Vec2::new(2.0, 0.0));
    }

    #[test]
    fn level_transitions_and_areas_resolve_on_their_grid() {
        let lower = Entity::new(1, 0);
        let upper = Entity::new(2, 0);
        let transition_entity = Entity::new(3, 0);
        let area_entity = Entity::new(4, 0);
        let mut map = empty_map();
        map.grids.insert(lower, grid(lower, 0.0, 0));
        map.grids.insert(upper, grid(upper, 0.0, 1));
        map.transitions.insert(
            transition_entity,
            LevelTransition {
                entity: transition_entity,
                source_grid: lower,
                source: Vec2::new(2.0, 2.0),
                destination_grid: upper,
                destination: Vec2::new(8.0, 8.0),
                bidirectional: true,
                enabled: true,
            },
        );
        map.areas.insert(
            area_entity,
            Area {
                entity: area_entity,
                grid: upper,
                name: "Medical".into(),
                bounds: Aabb {
                    min: Vec2::ZERO,
                    max: Vec2::new(10.0, 10.0),
                },
                atmosphere_zone: None,
                power_channel: None,
            },
        );

        assert_eq!(
            map.traverse_transition(transition_entity, lower).unwrap(),
            (upper, Vec2::new(8.0, 8.0))
        );
        assert_eq!(
            map.area_at(upper, Vec2::new(5.0, 5.0))
                .map(|area| area.name.as_str()),
            Some("Medical")
        );
    }

    #[test]
    fn docking_is_bidirectional_and_recoverable() {
        let station_grid = Entity::new(1, 0);
        let shuttle_grid = Entity::new(2, 0);
        let station_port = Entity::new(3, 0);
        let shuttle_port = Entity::new(4, 0);
        let mut map = empty_map();
        map.grids.insert(station_grid, grid(station_grid, 0.0, 0));
        map.grids.insert(shuttle_grid, grid(shuttle_grid, 20.0, 0));
        map.docking_ports.insert(
            station_port,
            DockingPort {
                entity: station_port,
                grid: station_grid,
                local_position: Vec2::ZERO,
                normal: Vec2::new(1.0, 0.0),
                docked_to: None,
                sealed: true,
            },
        );
        map.docking_ports.insert(
            shuttle_port,
            DockingPort {
                entity: shuttle_port,
                grid: shuttle_grid,
                local_position: Vec2::ZERO,
                normal: Vec2::new(-1.0, 0.0),
                docked_to: None,
                sealed: true,
            },
        );

        map.dock(station_port, shuttle_port).unwrap();
        assert_eq!(
            map.docking_ports[&station_port].docked_to,
            Some(shuttle_port)
        );
        assert_eq!(
            map.docking_ports[&shuttle_port].docked_to,
            Some(station_port)
        );
        map.undock(station_port).unwrap();
        assert_eq!(map.docking_ports[&station_port].docked_to, None);
        assert_eq!(map.docking_ports[&shuttle_port].docked_to, None);
    }
}

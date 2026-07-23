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
    pub chunks: HashMap<(i32, i32), Chunk>,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub id: u32,
    pub tile_size: f32,
    pub tiles: Vec<TileDef>,
    pub grids: HashMap<Entity, Grid>,
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
}

impl Map {
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
        let t = self
            .grids
            .get(&grid)
            .map(|g| g.transform)
            .unwrap_or_default();
        (p - t.translation).rotate(-t.rotation) / self.tile_size
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

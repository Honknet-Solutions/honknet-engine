use honknet_core::Entity;
use honknet_math::{Aabb, Vec2};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Cell(i32, i32);
pub struct SpatialIndex {
    cell: f32,
    cells: HashMap<Cell, HashSet<Entity>>,
    bounds: HashMap<Entity, Aabb>,
}

impl SpatialIndex {
    pub fn new(cell: f32) -> Self {
        Self {
            cell: cell.max(0.1),
            cells: HashMap::new(),
            bounds: HashMap::new(),
        }
    }
    fn range(&self, a: Aabb) -> (i32, i32, i32, i32) {
        (
            (a.min.x / self.cell).floor() as i32,
            (a.min.y / self.cell).floor() as i32,
            (a.max.x / self.cell).floor() as i32,
            (a.max.y / self.cell).floor() as i32,
        )
    }
    pub fn upsert(&mut self, e: Entity, a: Aabb) {
        self.remove(e);
        let (x0, y0, x1, y1) = self.range(a);
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cells.entry(Cell(x, y)).or_default().insert(e);
            }
        }
        self.bounds.insert(e, a);
    }
    pub fn remove(&mut self, e: Entity) {
        if let Some(a) = self.bounds.remove(&e) {
            let (x0, y0, x1, y1) = self.range(a);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if let Some(s) = self.cells.get_mut(&Cell(x, y)) {
                        s.remove(&e);
                    }
                }
            }
        }
    }
    pub fn query_aabb(&self, a: Aabb) -> Vec<Entity> {
        let (x0, y0, x1, y1) = self.range(a);
        let mut out = HashSet::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if let Some(s) = self.cells.get(&Cell(x, y)) {
                    out.extend(s.iter().copied())
                }
            }
        }
        out.into_iter()
            .filter(|e| self.bounds.get(e).is_some_and(|b| b.intersects(a)))
            .collect()
    }
    pub fn query_circle(&self, c: Vec2, r: f32) -> Vec<Entity> {
        self.query_aabb(Aabb::from_center_half(c, Vec2::new(r, r)))
            .into_iter()
            .filter(|e| {
                let b = self.bounds[e];
                let q = Vec2::new(c.x.clamp(b.min.x, b.max.x), c.y.clamp(b.min.y, b.max.y));
                (q - c).length_squared() <= r * r
            })
            .collect()
    }
    pub fn pairs(&self) -> Vec<(Entity, Entity)> {
        let mut p = HashSet::new();
        for s in self.cells.values() {
            let v: Vec<_> = s.iter().copied().collect();
            for i in 0..v.len() {
                for j in i + 1..v.len() {
                    let a = v[i].min(v[j]);
                    let b = v[i].max(v[j]);
                    if self.bounds[&a].intersects(self.bounds[&b]) {
                        p.insert((a, b));
                    }
                }
            }
        }
        p.into_iter().collect()
    }
}

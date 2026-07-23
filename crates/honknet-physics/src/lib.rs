use honknet_core::Entity;
use honknet_math::{Aabb, Vec2};
use honknet_spatial::SpatialIndex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyType {
    Static,
    Kinematic,
    Dynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Shape {
    Circle { radius: f32 },
    Capsule { half_height: f32, radius: f32 },
    Box { half: Vec2 },
    Polygon { vertices: Vec<Vec2> },
    Segment { a: Vec2, b: Vec2 },
    Chain { vertices: Vec<Vec2> },
    Compound { children: Vec<(Vec2, Shape)> },
}

impl Shape {
    pub fn aabb(&self, p: Vec2) -> Aabb {
        match self {
            Shape::Circle { radius } => Aabb::from_center_half(p, Vec2::new(*radius, *radius)),
            Shape::Capsule {
                half_height,
                radius,
            } => Aabb::from_center_half(p, Vec2::new(*radius, *half_height + *radius)),
            Shape::Box { half } => Aabb::from_center_half(p, *half),
            Shape::Polygon { vertices } | Shape::Chain { vertices } => {
                let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
                let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
                for v in vertices {
                    min = min.min(p + *v);
                    max = max.max(p + *v)
                }
                Aabb { min, max }
            }
            Shape::Segment { a, b } => Aabb {
                min: (p + *a).min(p + *b),
                max: (p + *a).max(p + *b),
            },
            Shape::Compound { children } => children
                .iter()
                .map(|(o, s)| s.aabb(p + *o))
                .reduce(|a, b| a.union(b))
                .unwrap_or(Aabb::from_center_half(p, Vec2::ZERO)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub shape: Shape,
    pub friction: f32,
    pub restitution: f32,
    pub sensor: bool,
    pub layer: u32,
    pub mask: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    pub entity: Entity,
    pub body_type: BodyType,
    pub position: Vec2,
    pub rotation: f32,
    pub velocity: Vec2,
    pub angular_velocity: f32,
    pub force: Vec2,
    pub torque: f32,
    pub mass: f32,
    pub inv_mass: f32,
    pub inertia: f32,
    pub inv_inertia: f32,
    pub damping: f32,
    pub gravity_scale: f32,
    pub fixed_rotation: bool,
    pub sleeping: bool,
    pub continuous: bool,
    pub fixtures: Vec<Fixture>,
}

impl Body {
    pub fn dynamic(entity: Entity, position: Vec2, mass: f32, fixture: Fixture) -> Self {
        let m = mass.max(0.001);
        Self {
            entity,
            body_type: BodyType::Dynamic,
            position,
            rotation: 0.,
            velocity: Vec2::ZERO,
            angular_velocity: 0.,
            force: Vec2::ZERO,
            torque: 0.,
            mass: m,
            inv_mass: 1. / m,
            inertia: m,
            inv_inertia: 1. / m,
            damping: 0.02,
            gravity_scale: 0.,
            fixed_rotation: false,
            sleeping: false,
            continuous: false,
            fixtures: vec![fixture],
        }
    }
    pub fn aabb(&self) -> Aabb {
        self.fixtures
            .iter()
            .map(|f| f.shape.aabb(self.position))
            .reduce(|a, b| a.union(b))
            .unwrap_or(Aabb::from_center_half(self.position, Vec2::ZERO))
    }
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub a: Entity,
    pub b: Entity,
    pub normal: Vec2,
    pub penetration: f32,
    pub point: Vec2,
    pub sensor: bool,
}

#[derive(Debug, Clone)]
pub enum CollisionEvent {
    Started(Contact),
    Persisted(Contact),
    Ended(Entity, Entity),
}

#[derive(Debug, Clone)]
pub enum Joint {
    Distance {
        a: Entity,
        b: Entity,
        length: f32,
        stiffness: f32,
    },
    Revolute {
        a: Entity,
        b: Entity,
        anchor: Vec2,
    },
}

pub struct PhysicsWorld {
    pub bodies: HashMap<Entity, Body>,
    pub gravity: Vec2,
    index: SpatialIndex,
    contacts: HashSet<(Entity, Entity)>,
    pub joints: Vec<Joint>,
    pub events: Vec<CollisionEvent>,
    pub iterations: u32,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            bodies: HashMap::new(),
            gravity: Vec2::ZERO,
            index: SpatialIndex::new(4.),
            contacts: HashSet::new(),
            joints: vec![],
            events: vec![],
            iterations: 8,
        }
    }
}

impl PhysicsWorld {
    pub fn insert(&mut self, b: Body) {
        self.index.upsert(b.entity, b.aabb());
        self.bodies.insert(b.entity, b);
    }
    pub fn remove(&mut self, e: Entity) {
        self.bodies.remove(&e);
        self.index.remove(e);
    }
    pub fn apply_force(&mut self, e: Entity, f: Vec2) {
        if let Some(b) = self.bodies.get_mut(&e) {
            b.force += f;
            b.sleeping = false
        }
    }
    pub fn step(&mut self, dt: f32) {
        self.events.clear();
        for b in self.bodies.values_mut() {
            if b.body_type == BodyType::Dynamic && !b.sleeping {
                let acc = self.gravity * b.gravity_scale + b.force * b.inv_mass;
                b.velocity = (b.velocity + acc * dt) * (1. - b.damping * dt).max(0.);
                b.position += b.velocity * dt;
                if !b.fixed_rotation {
                    b.angular_velocity += b.torque * b.inv_inertia * dt;
                    b.rotation += b.angular_velocity * dt
                }
                b.force = Vec2::ZERO;
                b.torque = 0.;
            }
        }
        for b in self.bodies.values() {
            self.index.upsert(b.entity, b.aabb())
        }
        let mut next = HashSet::new();
        let mut cs = Vec::new();
        for (a, b) in self.index.pairs() {
            let (ba, bb) = (&self.bodies[&a], &self.bodies[&b]);
            if ba.body_type == BodyType::Static && bb.body_type == BodyType::Static {
                continue;
            }
            if let Some(c) = collide(ba, bb) {
                let key = ordered(a, b);
                next.insert(key);
                self.events.push(if self.contacts.contains(&key) {
                    CollisionEvent::Persisted(c.clone())
                } else {
                    CollisionEvent::Started(c.clone())
                });
                if !c.sensor {
                    cs.push(c)
                }
            }
        }
        for key in self.contacts.difference(&next) {
            self.events.push(CollisionEvent::Ended(key.0, key.1))
        }
        self.contacts = next;
        for _ in 0..self.iterations {
            for c in &cs {
                self.solve(c)
            }
        }
        self.solve_joints();
    }
    fn solve(&mut self, c: &Contact) {
        if c.a == c.b {
            return;
        }
        let Some(mut a) = self.bodies.remove(&c.a) else {
            return;
        };
        let Some(mut b) = self.bodies.remove(&c.b) else {
            self.bodies.insert(c.a, a);
            return;
        };
        let rv = b.velocity - a.velocity;
        let vn = rv.dot(c.normal);
        if vn <= 0. {
            let e = a.fixtures[0].restitution.min(b.fixtures[0].restitution);
            let denom = a.inv_mass + b.inv_mass;
            if denom > 0. {
                let j = -(1. + e) * vn / denom;
                let impulse = c.normal * j;
                if a.body_type == BodyType::Dynamic {
                    a.velocity -= impulse * a.inv_mass;
                    a.position -= c.normal * (c.penetration * 0.5)
                }
                if b.body_type == BodyType::Dynamic {
                    b.velocity += impulse * b.inv_mass;
                    b.position += c.normal * (c.penetration * 0.5)
                }
                let tangent = (rv - c.normal * vn).normalized();
                let mu = (a.fixtures[0].friction * b.fixtures[0].friction).sqrt();
                let jt = (-rv.dot(tangent) / denom).clamp(-j * mu, j * mu);
                if a.body_type == BodyType::Dynamic {
                    a.velocity -= tangent * jt * a.inv_mass
                }
                if b.body_type == BodyType::Dynamic {
                    b.velocity += tangent * jt * b.inv_mass
                }
            }
        }
        self.bodies.insert(c.a, a);
        self.bodies.insert(c.b, b);
    }
    fn solve_joints(&mut self) {
        for j in self.joints.clone() {
            if let Joint::Distance {
                a,
                b,
                length,
                stiffness,
            } = j
            {
                if a == b {
                    continue;
                }
                let Some(mut ba) = self.bodies.remove(&a) else {
                    continue;
                };
                let Some(mut bb) = self.bodies.remove(&b) else {
                    self.bodies.insert(a, ba);
                    continue;
                };
                let d = bb.position - ba.position;
                let l = d.length();
                if l > 1e-5 {
                    let corr = d.normalized() * (l - length) * stiffness;
                    if ba.body_type == BodyType::Dynamic {
                        ba.position += corr * 0.5
                    }
                    if bb.body_type == BodyType::Dynamic {
                        bb.position -= corr * 0.5
                    }
                }
                self.bodies.insert(a, ba);
                self.bodies.insert(b, bb);
            }
        }
    }
    pub fn raycast(
        &self,
        origin: Vec2,
        dir: Vec2,
        max: f32,
        mask: u32,
    ) -> Option<(Entity, f32, Vec2)> {
        let d = dir.normalized();
        let mut best = None;
        for (e, b) in &self.bodies {
            for f in &b.fixtures {
                if f.layer & mask == 0 {
                    continue;
                }
                if let Some(t) = ray_aabb(origin, d, f.shape.aabb(b.position)) {
                    if t >= 0. && t <= max && best.is_none_or(|(_, bt, _)| t < bt) {
                        best = Some((*e, t, origin + d * t))
                    }
                }
            }
        }
        best
    }
    pub fn shape_cast(
        &self,
        shape: &Shape,
        start: Vec2,
        delta: Vec2,
        mask: u32,
    ) -> Option<(Entity, f32)> {
        let steps = 32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let a = shape.aabb(start + delta * t);
            for e in self.index.query_aabb(a) {
                let b = &self.bodies[&e];
                if b.fixtures
                    .iter()
                    .any(|f| f.layer & mask != 0 && a.intersects(f.shape.aabb(b.position)))
                {
                    return Some((e, t));
                }
            }
        }
        None
    }
}

fn ordered(a: Entity, b: Entity) -> (Entity, Entity) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn collide(a: &Body, b: &Body) -> Option<Contact> {
    for fa in &a.fixtures {
        for fb in &b.fixtures {
            if fa.mask & fb.layer == 0 || fb.mask & fa.layer == 0 {
                continue;
            }
            if let Some((n, p, pt)) = shape_contact(&fa.shape, a.position, &fb.shape, b.position) {
                return Some(Contact {
                    a: a.entity,
                    b: b.entity,
                    normal: n,
                    penetration: p,
                    point: pt,
                    sensor: fa.sensor || fb.sensor,
                });
            }
        }
    }
    None
}

fn shape_contact(a: &Shape, pa: Vec2, b: &Shape, pb: Vec2) -> Option<(Vec2, f32, Vec2)> {
    match (a, b) {
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            let d = pb - pa;
            let l = d.length();
            let p = ra + rb - l;
            (p > 0.).then(|| {
                let n = if l > 1e-5 { d / l } else { Vec2::new(1., 0.) };
                (n, p, pa + n * (*ra - p * 0.5))
            })
        }
        _ => {
            let aa = a.aabb(pa);
            let ab = b.aabb(pb);
            if !aa.intersects(ab) {
                return None;
            }
            let ox = (aa.max.x - ab.min.x).min(ab.max.x - aa.min.x);
            let oy = (aa.max.y - ab.min.y).min(ab.max.y - aa.min.y);
            if ox < oy {
                let n = Vec2::new(if pa.x < pb.x { 1. } else { -1. }, 0.);
                Some((n, ox, (pa + pb) * 0.5))
            } else {
                let n = Vec2::new(0., if pa.y < pb.y { 1. } else { -1. });
                Some((n, oy, (pa + pb) * 0.5))
            }
        }
    }
}

fn ray_aabb(o: Vec2, d: Vec2, a: Aabb) -> Option<f32> {
    let inv = Vec2::new(
        if d.x.abs() > 1e-6 {
            1. / d.x
        } else {
            f32::INFINITY
        },
        if d.y.abs() > 1e-6 {
            1. / d.y
        } else {
            f32::INFINITY
        },
    );
    let t1 = (a.min.x - o.x) * inv.x;
    let t2 = (a.max.x - o.x) * inv.x;
    let t3 = (a.min.y - o.y) * inv.y;
    let t4 = (a.max.y - o.y) * inv.y;
    let tmin = t1.min(t2).max(t3.min(t4));
    let tmax = t1.max(t2).min(t3.max(t4));
    (tmax >= tmin.max(0.)).then_some(tmin.max(0.))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn f(s: Shape) -> Fixture {
        Fixture {
            shape: s,
            friction: 0.5,
            restitution: 0.,
            sensor: false,
            layer: 1,
            mask: 1,
        }
    }
    #[test]
    fn circles_resolve() {
        let mut p = PhysicsWorld::default();
        p.insert(Body::dynamic(
            Entity::new(0, 0),
            Vec2::ZERO,
            1.,
            f(Shape::Circle { radius: 1. }),
        ));
        p.insert(Body::dynamic(
            Entity::new(1, 0),
            Vec2::new(1.5, 0.),
            1.,
            f(Shape::Circle { radius: 1. }),
        ));
        p.step(0.016);
        assert!(!p.events.is_empty())
    }
}

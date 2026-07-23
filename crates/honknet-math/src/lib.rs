use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0., y: 0. };
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: Self) -> f32 {
        self.x * o.y - self.y * o.x
    }
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }
    pub fn normalized(self) -> Self {
        let l = self.length();
        if l > 1e-6 {
            self / l
        } else {
            Self::ZERO
        }
    }
    pub fn perp(self) -> Self {
        Self::new(-self.y, self.x)
    }
    pub fn rotate(self, a: f32) -> Self {
        let (c, s) = (a.cos(), a.sin());
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    pub fn min(self, o: Self) -> Self {
        Self::new(self.x.min(o.x), self.y.min(o.y))
    }
    pub fn max(self, o: Self) -> Self {
        Self::new(self.x.max(o.x), self.y.max(o.y))
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, s: f32) -> Self {
        Self::new(self.x / s, self.y / s)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Self) {
        *self = *self + o
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Self) {
        *self = *self - o
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub fn from_center_half(c: Vec2, h: Vec2) -> Self {
        Self {
            min: c - h,
            max: c + h,
        }
    }
    pub fn union(self, o: Self) -> Self {
        Self {
            min: self.min.min(o.min),
            max: self.max.max(o.max),
        }
    }
    pub fn intersects(self, o: Self) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
    }
    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn expanded(self, v: f32) -> Self {
        Self {
            min: self.min - Vec2::new(v, v),
            max: self.max + Vec2::new(v, v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform2 {
    pub translation: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for Transform2 {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            rotation: 0.,
            scale: Vec2::new(1., 1.),
        }
    }
}

impl Transform2 {
    pub const IDENTITY: Self = Self {
        translation: Vec2::ZERO,
        rotation: 0.,
        scale: Vec2::new(1., 1.),
    };
    pub fn point(self, p: Vec2) -> Vec2 {
        Vec2::new(p.x * self.scale.x, p.y * self.scale.y).rotate(self.rotation) + self.translation
    }
    pub fn combine(self, child: Self) -> Self {
        Self {
            translation: self.point(child.translation),
            rotation: self.rotation + child.rotation,
            scale: Vec2::new(self.scale.x * child.scale.x, self.scale.y * child.scale.y),
        }
    }
}

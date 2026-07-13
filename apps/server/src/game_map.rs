use anyhow::{bail, Context, Result};
use honknet_protocol::MapSnapshot;
use serde::Deserialize;

pub const TILE_FLOOR: u8 = 0;
pub const TILE_WALL: u8 = 1;

#[derive(Debug, Clone)]
pub struct GameMap {
    pub id: String,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<u8>,
    pub door_spawn: (f32, f32),
}

#[derive(Debug, Deserialize)]
struct MapDefinition {
    id: String,
    rows: Vec<String>,
}

impl GameMap {
    pub fn load_debug() -> Result<Self> {
        let definition: MapDefinition =
            serde_json::from_str(include_str!("../../../content/maps/debug-map.json"))
                .context("failed to parse debug map")?;

        let height = definition.rows.len();
        if height == 0 {
            bail!("map has no rows");
        }

        let width = definition.rows[0].chars().count();
        if width == 0 {
            bail!("map has zero width");
        }

        let mut tiles = Vec::with_capacity(width * height);
        let mut door_spawn = None;

        for (y, row) in definition.rows.iter().enumerate() {
            if row.chars().count() != width {
                bail!("map row {y} has an inconsistent width");
            }

            for (x, character) in row.chars().enumerate() {
                match character {
                    '#' => tiles.push(TILE_WALL),
                    '.' => tiles.push(TILE_FLOOR),
                    'D' => {
                        tiles.push(TILE_FLOOR);
                        door_spawn = Some((x as f32 + 0.5, y as f32 + 0.5));
                    }
                    other => bail!("unsupported map character: {other}"),
                }
            }
        }

        let door_spawn = door_spawn.context("debug map has no door marker")?;

        Ok(Self {
            id: definition.id,
            width: width.try_into().context("map is too wide")?,
            height: height.try_into().context("map is too tall")?,
            tiles,
            door_spawn,
        })
    }

    pub fn snapshot(&self) -> MapSnapshot {
        MapSnapshot {
            id: self.id.clone(),
            width: self.width,
            height: self.height,
            tiles: self.tiles.clone(),
        }
    }

    pub fn circle_collides(&self, x: f32, y: f32, radius: f32) -> bool {
        let min_x = (x - radius).floor() as i32;
        let max_x = (x + radius).floor() as i32;
        let min_y = (y - radius).floor() as i32;
        let max_y = (y + radius).floor() as i32;

        for tile_y in min_y..=max_y {
            for tile_x in min_x..=max_x {
                if !self.is_wall(tile_x, tile_y) {
                    continue;
                }

                let nearest_x = x.clamp(tile_x as f32, tile_x as f32 + 1.0);
                let nearest_y = y.clamp(tile_y as f32, tile_y as f32 + 1.0);
                let dx = x - nearest_x;
                let dy = y - nearest_y;

                if dx * dx + dy * dy < radius * radius {
                    return true;
                }
            }
        }

        false
    }

    fn is_wall(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return true;
        }

        let index = y as usize * self.width as usize + x as usize;
        self.tiles.get(index).copied().unwrap_or(TILE_WALL) == TILE_WALL
    }
}

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use honknet_content::MapDocument;
use honknet_protocol::MapSnapshot;

pub const TILE_FLOOR: u8 = 0;
pub const TILE_WALL: u8 = 1;

#[derive(Debug, Clone)]
pub struct GameMap {
    pub id: String,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<u8>,
    pub door_spawn: (f32, f32),
    pub item_spawn: (f32, f32),
}

impl GameMap {
    pub fn load_debug() -> Result<Self> {
        let path =
            configured_workspace_path("HONKNET_MAP", "game/example-module/maps/debug-map.yml");

        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read map {}", path.display()))?;

        let document: MapDocument = serde_yaml::from_str(&text)
            .with_context(|| format!("failed to parse map {}", path.display()))?;

        let grid = document.map.grids.first().context("map has no grids")?;

        let chunk = grid.chunks.first().context("map grid has no chunks")?;

        let height = chunk.tiles.len();

        if height == 0 {
            bail!("map has no tile rows");
        }

        let width = chunk.tiles[0].len();

        if width == 0 {
            bail!("map has zero width");
        }

        let mut tiles = Vec::with_capacity(width * height);

        for (y, row) in chunk.tiles.iter().enumerate() {
            if row.len() != width {
                bail!("map row {y} has an inconsistent width");
            }

            for tile in row {
                match tile.as_str() {
                    "floor" => {
                        tiles.push(TILE_FLOOR);
                    }

                    "wall" => {
                        tiles.push(TILE_WALL);
                    }

                    other => {
                        bail!("unsupported tile id: {other}");
                    }
                }
            }
        }

        let mut door_spawn = None;
        let mut item_spawn = None;

        for entity in &document.map.entities {
            match entity.prototype.as_str() {
                "DebugDoor" => {
                    door_spawn = Some((entity.position[0], entity.position[1]));
                }

                "DebugWrench" => {
                    item_spawn = Some((entity.position[0], entity.position[1]));
                }

                _ => {}
            }
        }

        Ok(Self {
            id: document.map.id,

            width: width.try_into().context("map is too wide")?,

            height: height.try_into().context("map is too tall")?,

            tiles,

            door_spawn: door_spawn.context("map has no DebugDoor placement")?,

            item_spawn: item_spawn.unwrap_or((4.5, 4.5)),
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

fn configured_workspace_path(
    environment_variable: &str,
    default_relative_path: impl AsRef<Path>,
) -> PathBuf {
    match env::var_os(environment_variable) {
        Some(value) => {
            let configured_path = PathBuf::from(value);

            if configured_path.is_absolute() {
                configured_path
            } else {
                workspace_root().join(configured_path)
            }
        }

        None => workspace_root().join(default_relative_path),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

use std::{
    collections::{BTreeMap, HashMap},
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use honknet_content::{MapDocument, MapEntityDefinition, TileDefinition};
use honknet_protocol::{
    GridSnapshot, MapSnapshot, TileChunkSnapshot, TileDefinitionSnapshot,
};

#[derive(Debug, Clone)]
pub struct GameMap {
    pub id: String,
    pub tile_size: u16,
    pub map_hash: u64,
    pub entities: Vec<MapEntityDefinition>,
    tile_definitions: Vec<TileDefinitionSnapshot>,
    grids: BTreeMap<String, GameGrid>,
}

#[derive(Debug, Clone)]
struct GameGrid {
    id: String,
    position: [f32; 2],
    rotation: f32,
    chunks: Vec<TileChunkSnapshot>,
    tiles: HashMap<(i32, i32), u16>,
}

impl GameMap {
    pub fn load_debug() -> Result<Self> {
        let path = configured_workspace_path(
            "HONKNET_MAP",
            "examples/minimal-game/maps/debug-map.yml",
        );
        Self::load(path)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read map {}", path.display()))?;
        let document: MapDocument = serde_yaml::from_str(&text)
            .with_context(|| format!("failed to parse map {}", path.display()))?;
        if document.map.grids.is_empty() {
            bail!("map has no grids");
        }

        let mut definitions = document.map.tile_definitions;
        if definitions.is_empty() {
            definitions.extend(default_tile_definitions());
        }

        let mut known = definitions
            .iter()
            .map(|definition| definition.id.clone())
            .collect::<std::collections::HashSet<_>>();
        for grid in &document.map.grids {
            for chunk in &grid.chunks {
                for row in &chunk.tiles {
                    for tile in row {
                        if known.insert(tile.clone()) {
                            definitions.push(infer_tile_definition(tile));
                        }
                    }
                }
            }
        }
        definitions.sort_by(|left, right| left.id.cmp(&right.id));
        if definitions.len() > u16::MAX as usize {
            bail!("map has too many tile definitions");
        }

        let tile_indices = definitions
            .iter()
            .enumerate()
            .map(|(index, definition)| (definition.id.clone(), index as u16))
            .collect::<HashMap<_, _>>();
        let tile_definitions = definitions
            .into_iter()
            .map(|definition| TileDefinitionSnapshot {
                id: definition.id,
                solid: definition.solid,
                color: definition.color,
                texture: definition.texture,
            })
            .collect::<Vec<_>>();

        let mut grids = BTreeMap::new();
        for grid in document.map.grids {
            if grids.contains_key(&grid.id) {
                bail!("duplicate grid id {}", grid.id);
            }
            let mut tile_lookup = HashMap::new();
            let mut chunks = Vec::new();
            for chunk in grid.chunks {
                let height = chunk.tiles.len();
                if height == 0 {
                    continue;
                }
                let width = chunk.tiles[0].len();
                if width == 0 {
                    continue;
                }
                if chunk.tiles.iter().any(|row| row.len() != width) {
                    bail!("grid {} has a chunk with inconsistent row widths", grid.id);
                }
                let width_u16 = u16::try_from(width).context("map chunk is too wide")?;
                let height_u16 = u16::try_from(height).context("map chunk is too tall")?;
                let mut encoded = Vec::with_capacity(width * height);
                for (local_y, row) in chunk.tiles.iter().enumerate() {
                    for (local_x, tile) in row.iter().enumerate() {
                        let index = *tile_indices
                            .get(tile)
                            .with_context(|| format!("missing tile definition {tile}"))?;
                        encoded.push(index);
                        tile_lookup.insert(
                            (
                                chunk.position[0] + local_x as i32,
                                chunk.position[1] + local_y as i32,
                            ),
                            index,
                        );
                    }
                }
                chunks.push(TileChunkSnapshot {
                    position: chunk.position,
                    width: width_u16,
                    height: height_u16,
                    tiles: encoded,
                });
            }

            grids.insert(
                grid.id.clone(),
                GameGrid {
                    id: grid.id,
                    position: grid.position,
                    rotation: grid.rotation,
                    chunks,
                    tiles: tile_lookup,
                },
            );
        }

        for entity in &document.map.entities {
            if let Some(grid_id) = &entity.grid {
                if !grids.contains_key(grid_id) {
                    bail!(
                        "map entity {} references missing grid {}",
                        entity.prototype,
                        grid_id
                    );
                }
            }
        }

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        document.map.id.hash(&mut hasher);
        let map_hash = hasher.finish();

        Ok(Self {
            id: document.map.id,
            tile_size: document.map.tile_size.max(1),
            map_hash,
            entities: document.map.entities,
            tile_definitions,
            grids,
        })
    }

    pub fn snapshot(&self) -> MapSnapshot {
        MapSnapshot {
            id: self.id.clone(),
            tile_size: self.tile_size,
            tile_definitions: self.tile_definitions.clone(),
            grids: self
                .grids
                .values()
                .map(|grid| GridSnapshot {
                    id: grid.id.clone(),
                    position: grid.position,
                    rotation: grid.rotation,
                    chunks: grid.chunks.clone(),
                })
                .collect(),
        }
    }

    pub fn grid_ids(&self) -> impl Iterator<Item = &str> {
        self.grids.keys().map(String::as_str)
    }

    pub fn default_grid_id(&self) -> Option<&str> {
        self.grids.keys().next().map(String::as_str)
    }

    pub fn circle_collides(
        &self,
        grid_id: Option<&str>,
        world_x: f32,
        world_y: f32,
        radius: f32,
    ) -> bool {
        if let Some(grid_id) = grid_id {
            return self
                .grids
                .get(grid_id)
                .is_none_or(|grid| self.grid_circle_collides(grid, world_x, world_y, radius));
        }
        self.grids
            .values()
            .any(|grid| self.grid_circle_collides(grid, world_x, world_y, radius))
    }

    pub fn local_to_world(&self, grid_id: &str, local: [f32; 2]) -> Option<[f32; 2]> {
        let grid = self.grids.get(grid_id)?;
        let (sin, cos) = grid.rotation.sin_cos();
        Some([
            grid.position[0] + local[0] * cos - local[1] * sin,
            grid.position[1] + local[0] * sin + local[1] * cos,
        ])
    }

    fn grid_circle_collides(
        &self,
        grid: &GameGrid,
        world_x: f32,
        world_y: f32,
        radius: f32,
    ) -> bool {
        let translated_x = world_x - grid.position[0];
        let translated_y = world_y - grid.position[1];
        let (sin, cos) = (-grid.rotation).sin_cos();
        let x = translated_x * cos - translated_y * sin;
        let y = translated_x * sin + translated_y * cos;
        let min_x = (x - radius).floor() as i32;
        let max_x = (x + radius).floor() as i32;
        let min_y = (y - radius).floor() as i32;
        let max_y = (y + radius).floor() as i32;

        for tile_y in min_y..=max_y {
            for tile_x in min_x..=max_x {
                let Some(&tile_index) = grid.tiles.get(&(tile_x, tile_y)) else {
                    continue;
                };
                if !self
                    .tile_definitions
                    .get(tile_index as usize)
                    .is_some_and(|definition| definition.solid)
                {
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
}

fn default_tile_definitions() -> Vec<TileDefinition> {
    vec![
        TileDefinition {
            id: "floor".to_owned(),
            solid: false,
            color: [16, 23, 32, 255],
            texture: None,
        },
        TileDefinition {
            id: "wall".to_owned(),
            solid: true,
            color: [48, 55, 67, 255],
            texture: None,
        },
    ]
}

fn infer_tile_definition(id: &str) -> TileDefinition {
    let normalized = id.to_ascii_lowercase();
    TileDefinition {
        id: id.to_owned(),
        solid: normalized.contains("wall") || normalized.contains("solid"),
        color: if normalized.contains("water") {
            [32, 78, 112, 255]
        } else if normalized.contains("grass") {
            [34, 86, 55, 255]
        } else if normalized.contains("road") {
            [41, 44, 50, 255]
        } else {
            [24, 32, 43, 255]
        },
        texture: None,
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

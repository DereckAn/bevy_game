//! Colocación determinista de árboles: decide qué especie va en cada celda de
//! espaciado y la estampa en los chunks. También reporta al loader hasta qué
//! altura llega un árbol (para no marcar como "aire" los chunks que atraviesa).
//!
//! Todo es función pura de `(celda, seed)`, así cualquier chunk reconstruye los
//! mismos árboles → consistentes a través de bordes y regeneraciones.

use super::bush::bush_template;
use super::oak::oak_template;
use super::pine::pine_template;
use super::small::tree_template;
use crate::core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{BaseChunk, BiomeGenerator, VoxelType};
use bevy::prelude::*;

/// Tamaño de la celda de espaciado (en voxels). Como máximo un árbol por celda.
pub const TREE_CELL_SIZE: i32 = 12;

/// Probabilidad (0..1) de que una celda contenga un árbol.
const TREE_PROBABILITY: f32 = 0.45;

/// Alcance horizontal máximo de un árbol (voxels): debe ser >= lo que sobresale
/// el más ancho (ramas + hojas del pino). Lo usan el escaneo de celdas y el loader.
const MAX_CANOPY_RADIUS: i32 = 8;

/// Tipo de árbol: arbusto pequeño (esfera) o pino grande (cónico, con ramas).
#[derive(Clone, Copy, PartialEq)]
pub enum TreeKind {
    Small,
    Pine,
    Oak,
    Bush,
}

/// Un árbol candidato: su columna (x,z) en VOXELS de mundo y su forma.
pub struct TreeInstance {
    pub world_x: i32,
    pub world_z: i32,
    pub kind: TreeKind,
    pub trunk_height: i32,
    pub canopy_radius: i32,
    pub rng_seed: u32, // para variar la forma de cada pino (ramas + mechón)
}

impl TreeInstance {
    /// Altura del árbol SOBRE su base, en voxels (cima del follaje).
    pub fn height(&self) -> i32 {
        match self.kind {
            TreeKind::Small => self.trunk_height + self.canopy_radius,
            TreeKind::Pine => self.trunk_height + 2, // mechón en la punta
            TreeKind::Oak => self.trunk_height + self.trunk_height / 2,
            TreeKind::Bush => self.canopy_radius, // cúpula de radio = altura
        }
    }
}

/// Hash determinista de una celda + seed -> u32 bien mezclado.
fn hash_cell(cell_x: i32, cell_z: i32, seed: i32) -> u32 {
    let mut h = (cell_x as u32).wrapping_mul(0x9e37_79b9);
    h = (h ^ (cell_z as u32)).wrapping_mul(0x85eb_ca6b);
    h = (h ^ (seed as u32)).wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

/// Decide de forma DETERMINISTA si la celda `(cell_x, cell_z)` tiene un árbol.
///
/// Devuelve `Some(TreeInstance)` (posición con jitter dentro de la celda + forma)
/// o `None`. Función PURA de `(cell, seed)`: cualquier chunk que pregunte por la
/// misma celda obtiene el mismo resultado → árboles consistentes en los bordes.
pub fn tree_in_cell(cell_x: i32, cell_z: i32, seed: i32) -> Option<TreeInstance> {
    let h = hash_cell(cell_x, cell_z, seed);

    // Bits 0–11 → probabilidad. Si no pasa, no hay árbol en esta celda.
    let p = (h & 0xfff) as f32 / 0xfff as f32;
    if p > TREE_PROBABILITY {
        return None;
    }

    let jitter_x = ((h >> 12) & 0xff) as i32 % TREE_CELL_SIZE;
    let jitter_z = ((h >> 20) & 0xff) as i32 % TREE_CELL_SIZE;
    let world_x = cell_x * TREE_CELL_SIZE + jitter_x;
    let world_z = cell_z * TREE_CELL_SIZE + jitter_z;

    // Segundo hash con "sal" → bits independientes para tipo y forma del pino.
    let h2 = hash_cell(cell_x, cell_z, seed ^ 0x5f37_59df);

    if h2 % 6 == 0 {
        // pino (igual que antes)
        let trunk_height = 40 + ((h2 >> 8) % 20) as i32; // 40..=59
        Some(TreeInstance {
            world_x,
            world_z,
            kind: TreeKind::Pine,
            trunk_height,
            canopy_radius: 0,
            rng_seed: h2,
        })
    } else if h2 % 6 == 1 {
        // roble
        let trunk_height = 28 + ((h2 >> 8) % 14) as i32; // 28..=41
        Some(TreeInstance {
            world_x,
            world_z,
            kind: TreeKind::Oak,
            trunk_height,
            canopy_radius: 0,
            rng_seed: h2,
        })
    } else if h2 % 6 == 2 {
        // arbusto (follaje atravesable)
        let canopy_radius = 1 + ((h2 >> 8) % 2) as i32; // 1..=2
        Some(TreeInstance {
            world_x,
            world_z,
            kind: TreeKind::Bush,
            trunk_height: 0,
            canopy_radius,
            rng_seed: h2,
        })
    } else {
        // arbusto pequeño
        let trunk_height = 4 + ((h >> 28) & 0x3) as i32;
        let canopy_radius = 2 + ((h >> 30) & 0x1) as i32;
        Some(TreeInstance {
            world_x,
            world_z,
            kind: TreeKind::Small,
            trunk_height,
            canopy_radius,
            rng_seed: h2,
        })
    }
}

/// Estampa en `chunk` los árboles deterministas que caen dentro de él.
///
/// Recorre las celdas de espaciado que se solapan con el chunk (expandido por el
/// alcance de copa), calcula cada árbol con `tree_in_cell`, obtiene la altura de
/// su columna con `biome` (función pura → vale también para columnas fuera del
/// chunk) y escribe los voxels de la plantilla que caen DENTRO de este chunk.
/// Solo escribe sobre aire, para no perforar el terreno.
pub fn place_trees(chunk: &mut BaseChunk, biome: &mut BiomeGenerator, seed: i32) {
    let n = BASE_CHUNK_SIZE as i32;

    // Origen del chunk en voxels de mundo.
    let origin = chunk.position * n;

    // Rango de celdas que pueden alcanzar este chunk en XZ (expandido por la copa).
    let r = MAX_CANOPY_RADIUS;
    let cell_x_min = (origin.x - r).div_euclid(TREE_CELL_SIZE);
    let cell_x_max = (origin.x + n - 1 + r).div_euclid(TREE_CELL_SIZE);
    let cell_z_min = (origin.z - r).div_euclid(TREE_CELL_SIZE);
    let cell_z_max = (origin.z + n - 1 + r).div_euclid(TREE_CELL_SIZE);

    for cell_x in cell_x_min..=cell_x_max {
        for cell_z in cell_z_min..=cell_z_max {
            let Some(tree) = tree_in_cell(cell_x, cell_z, seed) else {
                continue;
            };

            // Altura de la superficie en la columna del árbol (metros → voxels).
            let world_x_m = tree.world_x as f32 * VOXEL_SIZE;
            let world_z_m = tree.world_z as f32 * VOXEL_SIZE;
            let surface_m = biome.generate_height(world_x_m, world_z_m);
            let surface_voxel_y = (surface_m / VOXEL_SIZE).floor() as i32;

            // Base = el voxel justo SOBRE el suelo (donde arranca el tronco).
            let base = IVec3::new(tree.world_x, surface_voxel_y + 1, tree.world_z);

            let template = match tree.kind {
                TreeKind::Small => tree_template(tree.trunk_height, tree.canopy_radius),
                TreeKind::Pine => pine_template(tree.rng_seed, tree.trunk_height),
                TreeKind::Oak => oak_template(tree.rng_seed, tree.trunk_height),
                TreeKind::Bush => bush_template(tree.canopy_radius),
            };
            for tv in template {
                let world = base + tv.offset;
                let local = world - origin;

                // ¿Cae este voxel dentro de ESTE chunk?
                if local.x < 0
                    || local.x >= n
                    || local.y < 0
                    || local.y >= n
                    || local.z < 0
                    || local.z >= n
                {
                    continue;
                }

                let (lx, ly, lz) = (local.x as usize, local.y as usize, local.z as usize);
                // Solo sobre aire: no perforamos el terreno ya generado.
                if chunk.voxel_types[lx][ly][lz] == VoxelType::Air {
                    chunk.voxel_types[lx][ly][lz] = tv.voxel_type;
                }
            }
        }
    }
}

/// Y de voxel más alto alcanzado por algún árbol cuya base cae en la huella XZ de
/// este chunk (expandida por el alcance horizontal). `None` si no hay árboles.
///
/// El loader lo consulta para NO marcar como "aire" un chunk que un árbol cruza.
pub fn tree_ceiling_for_chunk(
    chunk_pos: IVec3,
    biome: &mut BiomeGenerator,
    seed: i32,
) -> Option<i32> {
    let n = BASE_CHUNK_SIZE as i32;
    let origin = chunk_pos * n;
    let r = MAX_CANOPY_RADIUS;
    let cell_x_min = (origin.x - r).div_euclid(TREE_CELL_SIZE);
    let cell_x_max = (origin.x + n - 1 + r).div_euclid(TREE_CELL_SIZE);
    let cell_z_min = (origin.z - r).div_euclid(TREE_CELL_SIZE);
    let cell_z_max = (origin.z + n - 1 + r).div_euclid(TREE_CELL_SIZE);

    let mut ceiling: Option<i32> = None;
    for cell_x in cell_x_min..=cell_x_max {
        for cell_z in cell_z_min..=cell_z_max {
            let Some(tree) = tree_in_cell(cell_x, cell_z, seed) else {
                continue;
            };
            let wx_m = tree.world_x as f32 * VOXEL_SIZE;
            let wz_m = tree.world_z as f32 * VOXEL_SIZE;
            let surface_voxel_y = (biome.generate_height(wx_m, wz_m) / VOXEL_SIZE).floor() as i32;
            let top = surface_voxel_y + 1 + tree.height();
            ceiling = Some(ceiling.map_or(top, |c| c.max(top)));
        }
    }
    ceiling
}

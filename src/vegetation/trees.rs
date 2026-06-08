//! Plantillas de árboles: patrones de voxels (tronco + copa) que luego se
//! "estampan" en los chunks durante la generación.

use crate::core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{BaseChunk, BiomeGenerator, VoxelType};
use bevy::prelude::*;

/// Un voxel de una plantilla de árbol: su posición RELATIVA a la base del árbol
/// (la base = el voxel justo sobre el suelo) y su tipo de material.
#[derive(Clone, Copy)]
pub struct TreeVoxel {
    pub offset: IVec3,
    pub voxel_type: VoxelType,
}

/// Tamano de la celda de espaciado (en voxels). Como maximo un arbol por celda
pub const TREE_CELL_SIZE: i32 = 12;

/// Probabilidad (0..1) de que una celda contenga un arbol.
const TREE_PROBABILITY: f32 = 0.45;

/// Alcance horizontal máximo de una copa: dice qué celdas vecinas pueden invadir
/// este chunk. Debe ser >= el mayor `canopy_radius` posible.
const MAX_CANOPY_RADIUS: i32 = 3;

/// Un arbol candidato : su columna (x,z) en VOXELS de mundo y su forma
pub struct TreeInstance {
    pub world_x: i32,
    pub world_z: i32,
    pub trunk_height: i32,
    pub canopy_radius: i32,
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

    // Rangos de bits DISTINTOS = "canales" pseudo-aleatorios independientes.
    let jitter_x = ((h >> 12) & 0xff) as i32 % TREE_CELL_SIZE;
    let jitter_z = ((h >> 20) & 0xff) as i32 % TREE_CELL_SIZE;
    let trunk_height = 4 + ((h >> 28) & 0x3) as i32; // 4..=7
    let canopy_radius = 2 + ((h >> 30) & 0x1) as i32; // 2..=3

    Some(TreeInstance {
        world_x: cell_x * TREE_CELL_SIZE + jitter_x,
        world_z: cell_z * TREE_CELL_SIZE + jitter_z,
        trunk_height,
        canopy_radius,
    })
}

/// Genera la plantilla de un árbol simple: un tronco vertical de `Wood` y una
/// copa esférica de `Leaves` cerca de la cima.
///
/// Las posiciones son RELATIVAS a la base `(0, 0, 0)` = el voxel sobre el suelo.
/// Es una función PURA: misma entrada → misma salida, sin aleatoriedad ni estado.
pub fn tree_template(trunk_height: i32, canopy_radius: i32) -> Vec<TreeVoxel> {
    let mut voxels = Vec::new();

    // Tronco: columna vertical de Wood desde la base (y=0) hasta trunk_height-1.
    for y in 0..trunk_height {
        voxels.push(TreeVoxel {
            offset: IVec3::new(0, y, 0),
            voxel_type: VoxelType::Wood,
        });
    }

    // Copa: esfera de Leaves centrada cerca de la cima del tronco.
    let center_y = trunk_height;
    let r = canopy_radius;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                if dx * dx + dy * dy + dz * dz <= r * r {
                    voxels.push(TreeVoxel {
                        offset: IVec3::new(dx, center_y + dy, dz),
                        voxel_type: VoxelType::Leaves,
                    });
                }
            }
        }
    }

    voxels
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

            for tv in tree_template(tree.trunk_height, tree.canopy_radius) {
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

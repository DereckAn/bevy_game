//! Pasto: tufos cortos de follaje ATRAVESABLE (`VoxelType::Foliage`), densos,
//! sobre las columnas de pasto del chunk.
//!
//! A diferencia de los árboles (rejilla dispersa de celdas), el pasto es denso:
//! se decide por COLUMNA con un hash del mundo. Determinista → cada chunk
//! reconstruye el mismo pasto.

use crate::core::constants::BASE_CHUNK_SIZE;
use crate::voxel::{BaseChunk, VoxelType};

/// Fracción de columnas de pasto que reciben un tufo.
const GRASS_DENSITY: f32 = 0.35;

/// Hash determinista por columna mundial + seed.
fn column_hash(wx: i32, wz: i32, seed: i32) -> u32 {
    let mut h = (wx as u32).wrapping_mul(0x9e37_79b9);
    h = (h ^ (wz as u32)).wrapping_mul(0x85eb_ca6b);
    h = (h ^ (seed as u32)).wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

/// Estampa tufos de pasto sobre las columnas cuyo voxel de superficie (el sólido
/// más alto DENTRO de este chunk) es pasto. Se ejecuta después de los árboles, así
/// no crece pasto encima de troncos/copas.
pub fn place_grass(chunk: &mut BaseChunk, seed: i32) {
    let n = BASE_CHUNK_SIZE;
    let origin_x = chunk.position.x * n as i32;
    let origin_z = chunk.position.z * n as i32;

    for lz in 0..n {
        for lx in 0..n {
            // Voxel sólido más alto de la columna en este chunk = su superficie.
            let mut surface = None;
            for ly in (0..n).rev() {
                if chunk.voxel_types[lx][ly][lz].is_solid() {
                    surface = Some(ly);
                    break;
                }
            }
            let Some(sy) = surface else { continue };

            // Solo sobre pasto (no piedra/arena/madera/hojas).
            if chunk.voxel_types[lx][sy][lz] != VoxelType::Grass {
                continue;
            }

            // Decisión determinista por columna.
            let h = column_hash(origin_x + lx as i32, origin_z + lz as i32, seed);
            if (h & 0xff) as f32 / 255.0 > GRASS_DENSITY {
                continue;
            }

            // Tufo de 1 o 2 voxels de alto, justo sobre la superficie.
            let height = 1 + ((h >> 8) & 1) as usize;
            for dy in 1..=height {
                let ly = sy + dy;
                if ly >= n {
                    break; // se saldría por arriba del chunk
                }
                if chunk.voxel_types[lx][ly][lz] == VoxelType::Air {
                    chunk.voxel_types[lx][ly][lz] = VoxelType::Foliage;
                }
            }
        }
    }
}

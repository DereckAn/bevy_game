//! Pasto: tufos cortos de follaje ATRAVESABLE (`VoxelType::Foliage`), densos,
//! sobre las columnas de pasto del chunk.
//!
//! A diferencia de los árboles (rejilla dispersa de celdas), el pasto es denso:
//! se decide por COLUMNA con un hash del mundo. Determinista → cada chunk
//! reconstruye el mismo pasto.

use crate::core::constants::BASE_CHUNK_SIZE;
use crate::core::WorldKind;
use crate::voxel::{BaseChunk, VoxelType};

/// Fracción de columnas de pasto que reciben un tufo.
const GRASS_DENSITY: f32 = 0.35;

/// Fracción de columnas de arena con un tufo seco DENTRO de un parche (el desierto
/// agrupa el pasto en manchas, no lo esparce parejo).
const DESERT_GRASS_DENSITY: f32 = 0.10;

/// Lado del parche de pasto de desierto, en voxels (~4 m con VOXEL_SIZE=0.1).
const DESERT_PATCH_SIZE: i32 = 20;

/// Fracción de parches que tienen pasto (el resto es arena pelada).
const DESERT_PATCH_COVERAGE: f32 = 0.15;

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
pub fn place_grass(chunk: &mut BaseChunk, seed: i32, kind: WorldKind) {
    let n = BASE_CHUNK_SIZE;
    let origin_x = chunk.position.x * n as i32;
    let origin_z = chunk.position.z * n as i32;

    // El tufo crece sobre pasto (normal) o sobre arena (desierto), con densidades
    // distintas: el desierto es mucho más disperso.
    let (surface_needed, density, foliage) = match kind {
        WorldKind::Normal => (VoxelType::Grass, GRASS_DENSITY, VoxelType::Foliage),
        WorldKind::Desert => (
            VoxelType::Sand,
            DESERT_GRASS_DENSITY,
            VoxelType::DesertGrass,
        ),
        // El bioma helado no tiene tufos de vegetación sobre la nieve.
        WorldKind::Ice => return,
        // El manglar no tiene tufos de pasto sobre el fango.
        WorldKind::Mangrove => return,
    };

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

            if chunk.voxel_types[lx][sy][lz] != surface_needed {
                continue;
            }

            let wx = origin_x + lx as i32;
            let wz = origin_z + lz as i32;

            // Desierto: el pasto solo aparece en parches (manchas), no parejo.
            if kind == WorldKind::Desert {
                let patch = column_hash(
                    wx.div_euclid(DESERT_PATCH_SIZE),
                    wz.div_euclid(DESERT_PATCH_SIZE),
                    seed ^ 0x1234_5678,
                );
                if (patch & 0xff) as f32 / 255.0 > DESERT_PATCH_COVERAGE {
                    continue;
                }
            }

            // Decisión determinista por columna.
            let h = column_hash(wx, wz, seed);
            if (h & 0xff) as f32 / 255.0 > density {
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
                    chunk.voxel_types[lx][ly][lz] = foliage;
                }
            }
        }
    }
}

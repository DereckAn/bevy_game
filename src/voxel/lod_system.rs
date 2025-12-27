//! Sistema de level of detail (LOD) para chunks dinamicos
//! 

use bevy::prelude::*;
use crate::{core::constants::{BASE_CHUNK_SIZE, LOD_DISTANCES}, player::Player};

/// Niveles de detalle para chunks
#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkLOD{
    Ultra,   // 32³ individual (0-50m)
    High,    // 64³ (2x2x2 merged) (50-100m)
    Medium,  // 128³ (4x4x4 merged) (100-200m)
    Low,     // 256³ (8x8x8 merged) (200-400m)
    Minimal, // 512³ (16x16x16 merged) (400m+)
}

impl ChunkLOD {
    // Determina LOD basado en distancia al jugador
    pub fn from_distance(distance:f32) -> Self {
        match distance {
            d if d < LOD_DISTANCES[0] => ChunkLOD::Ultra,
            d if d < LOD_DISTANCES[1] => ChunkLOD::High,
            d if d < LOD_DISTANCES[2] => ChunkLOD::Medium,
            d if d < LOD_DISTANCES[3] => ChunkLOD::Low,
            d if d < LOD_DISTANCES[4] => ChunkLOD::Minimal,
            _ => ChunkLOD::Minimal
        }
    }

    // Factor de combinacion (cuantos chunks base se combinan)
    pub fn merge_factor(&self) -> usize {
        match self {
             ChunkLOD::Ultra => 1,   // No merging
            ChunkLOD::High => 2,    // 2x2x2 = 8 chunks
            ChunkLOD::Medium => 4,  // 4x4x4 = 64 chunks
            ChunkLOD::Low => 8,     // 8x8x8 = 512 chunks
            ChunkLOD::Minimal => 16, // 16x16x16 = 4096 chunks
        }
    }

    // Tamano efectivo del chunk combinado
    pub fn effective_size(&self) -> usize {
        BASE_CHUNK_SIZE * self.merge_factor()
    }
}

/// Sistema que actualiza LOD basado en posicion del jugador 
pub fn update_chunk_lod_system(
    player_query: Query<&Transform, With<Player>>,
    mut chunk_query: Query<(&Transform, &mut ChunkLOD)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (chunk_transform, mut chunk_lod) in chunk_query.iter_mut() {
        let distance = player_transform.translation.distance(chunk_transform.translation);
        let new_lod = ChunkLOD::from_distance(distance);

        if *chunk_lod != new_lod {
            *chunk_lod = new_lod;
            // TODO: Trigger chunk re-meshing
        }
    }
}
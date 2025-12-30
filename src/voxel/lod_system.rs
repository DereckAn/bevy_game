//! Sistema de level of detail (LOD) para chunks dinamicos
//! 

use bevy::prelude::*;
use crate::{core::constants::{BASE_CHUNK_SIZE, LOD_DISTANCES}, player::Player, voxel::BaseChunk};

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
    pub fn from_distance(distance: f32) -> Self {
        match distance {
            d if d < LOD_DISTANCES[0] => ChunkLOD::Ultra,
            d if d < LOD_DISTANCES[1] => ChunkLOD::High,
            d if d < LOD_DISTANCES[2] => ChunkLOD::Medium,
            d if d < LOD_DISTANCES[3] => ChunkLOD::Low,
            _ => ChunkLOD::Minimal
        }
    }

    // Factor de combinacion (cuantos chunks base se combinan)
    pub fn merge_factor(&self) -> usize {
        match self {
            ChunkLOD::Ultra => 1,    // No merging
            ChunkLOD::High => 2,     // 2x2x2 = 8 chunks
            ChunkLOD::Medium => 4,   // 4x4x4 = 64 chunks
            ChunkLOD::Low => 8,      // 8x8x8 = 512 chunks
            ChunkLOD::Minimal => 16, // 16x16x16 = 4096 chunks
        }
    }

    // Tamano efectivo del chunk combinado
    pub fn effective_size(&self) -> usize {
        BASE_CHUNK_SIZE * self.merge_factor()
    }

    /// Color de debug para visualizar LOD
    /// Verde (cerca) → Amarillo → Naranja → Rojo (lejos)
    pub fn debug_color(&self) -> Color {
        match self {
            ChunkLOD::Ultra => Color::srgb(0.2, 0.8, 0.2),   // Verde brillante (cerca)
            ChunkLOD::High => Color::srgb(0.6, 0.8, 0.2),    // Verde-amarillo
            ChunkLOD::Medium => Color::srgb(0.9, 0.7, 0.1),  // Amarillo-naranja
            ChunkLOD::Low => Color::srgb(0.9, 0.4, 0.1),     // Naranja
            ChunkLOD::Minimal => Color::srgb(0.9, 0.1, 0.1), // Rojo (lejos)
        }
    }
}

/// Sistema que actualiza LOD y color basado en posicion del jugador 
pub fn update_chunk_lod_system(
    player_query: Query<&Transform, With<Player>>,
    mut chunk_query: Query<(&BaseChunk, &mut ChunkLOD, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (base_chunk, mut chunk_lod, material_handle) in chunk_query.iter_mut() {
        // Calcular posición del chunk en el mundo desde su posición en la grilla
        // Cada chunk es BASE_CHUNK_SIZE * VOXEL_SIZE = 32 * 0.1 = 3.2 metros
        let chunk_world_pos = Vec3::new(
            base_chunk.position.x as f32 * BASE_CHUNK_SIZE as f32 * 0.1,  // VOXEL_SIZE = 0.1
            base_chunk.position.y as f32 * BASE_CHUNK_SIZE as f32 * 0.1,
            base_chunk.position.z as f32 * BASE_CHUNK_SIZE as f32 * 0.1,
        );
        
        let distance = player_transform.translation.distance(chunk_world_pos);
        let new_lod = ChunkLOD::from_distance(distance);

        if *chunk_lod != new_lod {
            *chunk_lod = new_lod;
            
            // Actualizar color del material para visualizar LOD
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.base_color = new_lod.debug_color();
            }
            
            info!("Chunk at {:?} LOD changed to {:?} at distance {:.1}m", base_chunk.position, new_lod, distance);
        }
    }
}
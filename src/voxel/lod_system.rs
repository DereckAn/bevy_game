//! Sistema de level of detail (LOD) para chunks dinamicos
//! Usa downsampling para reducir detalle en chunks distantes

use bevy::prelude::*;
use crate::{
    core::constants::{BASE_CHUNK_SIZE, LOD_DISTANCES}, 
    player::Player, 
    voxel::{BaseChunk, DownsampledChunk, greedy_mesh_downsampled},
};

/// Niveles de detalle para chunks
#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkLOD{
    Ultra,   // 32³ completo (0-32m)
    High,    // 32³ completo (32-64m)
    Medium,  // 16³ downsampled 2x (64-128m)
    Low,     // 8³ downsampled 4x (128-192m)
    Minimal, // 4³ downsampled 8x (192m+)
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

    // Factor de downsampling (cuánto se reduce la resolución)
    pub fn downsample_factor(&self) -> usize {
        match self {
            ChunkLOD::Ultra => 1,    // 32³ completo
            ChunkLOD::High => 1,     // 32³ completo
            ChunkLOD::Medium => 2,   // 16³ (2x downsampling)
            ChunkLOD::Low => 4,      // 8³ (4x downsampling)
            ChunkLOD::Minimal => 8,  // 4³ (8x downsampling)
        }
    }
    
    // Indica si este LOD requiere downsampling
    pub fn needs_downsampling(&self) -> bool {
        matches!(self, ChunkLOD::Medium | ChunkLOD::Low | ChunkLOD::Minimal)
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

/// Sistema que actualiza LOD, color basado en posicion del jugador 
/// (Downsampling deshabilitado temporalmente para mejor rendimiento inicial)
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
        let chunk_world_pos = Vec3::new(
            base_chunk.position.x as f32 * BASE_CHUNK_SIZE as f32 * 0.1,
            base_chunk.position.y as f32 * BASE_CHUNK_SIZE as f32 * 0.1,
            base_chunk.position.z as f32 * BASE_CHUNK_SIZE as f32 * 0.1,
        );
        
        let distance = player_transform.translation.distance(chunk_world_pos);
        let new_lod = ChunkLOD::from_distance(distance);

        if *chunk_lod != new_lod {
            *chunk_lod = new_lod;
            
            // Actualizar color del material
            if let Some(material) = materials.get_mut(&material_handle.0) {
                material.base_color = new_lod.debug_color();
            }
        }
    }
}
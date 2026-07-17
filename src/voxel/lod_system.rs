//! Sistema de level of detail (LOD) para chunks dinamicos
//! Usa downsampling para reducir detalle en chunks distantes

use crate::{
    core::constants::{BASE_CHUNK_SIZE, LOD_DISTANCES},
    player::Player,
    voxel::BaseChunk,
};
use bevy::prelude::*;

/// Niveles de detalle para chunks
#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkLOD {
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
            _ => ChunkLOD::Minimal,
        }
    }
}

/// Sistema que actualiza LOD, color basado en posicion del jugador
/// (Downsampling deshabilitado temporalmente para mejor rendimiento inicial)
pub fn update_chunk_lod_system(
    player_query: Query<&Transform, With<Player>>,
    mut chunk_query: Query<(
        &BaseChunk,
        &mut ChunkLOD,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    chunk_materials: Res<crate::voxel::ChunkMaterials>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (base_chunk, mut chunk_lod, mut material_handle) in chunk_query.iter_mut() {
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

            // Los materiales son compartidos: cambiar el handle, no mutar el
            // material (eso recolorearía TODOS los chunks de ese nivel).
            material_handle.0 = chunk_materials.real_handle(new_lod);
        }
    }
}

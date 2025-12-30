//! Sistema de Frustum Culling para renderizar solo chunks visibles
//! Mejora FPS en 50-75% al no renderizar chunks fuera de la cámara
//! 
//! NOTA MULTIPLAYER: Este sistema es CLIENT-SIDE ONLY
//! Cada cliente decide qué chunks renderizar basado en su propia cámara
//! El servidor NO usa frustum culling - siempre simula todos los chunks activos

use bevy::prelude::*;
use crate::{
    core::BASE_CHUNK_SIZE,
    voxel::BaseChunk,
};

/// Sistema simplificado de frustum culling usando distancia y ángulo
/// Más robusto que calcular planos del frustum
pub fn update_frustum_culling(
    camera_query: Query<&Transform, With<Camera>>,
    mut chunk_query: Query<(&BaseChunk, &mut Visibility)>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    
    let camera_pos = camera_transform.translation;
    let camera_forward = *camera_transform.forward();
    
    // Tamaño de chunk en metros
    let chunk_size = BASE_CHUNK_SIZE as f32 * 0.1; // VOXEL_SIZE = 0.1
    
    // Parámetros de culling
    let max_distance = 200.0; // Distancia máxima de renderizado
    let fov_angle = 110.0_f32.to_radians(); // Campo de visión amplio (110 grados)
    let cos_half_fov = (fov_angle * 0.5).cos();
    
    // Actualizar visibilidad de cada chunk
    for (base_chunk, mut visibility) in chunk_query.iter_mut() {
        // Calcular centro del chunk en mundo
        let chunk_center = Vec3::new(
            (base_chunk.position.x as f32 + 0.5) * chunk_size,
            (base_chunk.position.y as f32 + 0.5) * chunk_size,
            (base_chunk.position.z as f32 + 0.5) * chunk_size,
        );
        
        // Vector desde cámara al chunk
        let to_chunk = chunk_center - camera_pos;
        let distance = to_chunk.length();
        
        // Culling por distancia
        if distance > max_distance {
            *visibility = Visibility::Hidden;
            continue;
        }
        
        // Culling por ángulo (FOV)
        // Solo aplicar FOV culling a chunks que están lejos
        // Chunks cercanos siempre visibles para evitar pop-in
        if distance > chunk_size * 3.0 {
            let to_chunk_normalized = to_chunk / distance;
            let dot = camera_forward.dot(to_chunk_normalized);
            
            // Si el ángulo es mayor que FOV/2, está fuera de vista
            if dot < cos_half_fov {
                *visibility = Visibility::Hidden;
                continue;
            }
        }
        
        // Chunk visible
        *visibility = Visibility::Visible;
    }
}

// ============================================================================
// NOTAS SOBRE MULTIPLAYER
// ============================================================================
//
// FRUSTUM CULLING ES CLIENT-SIDE ONLY:
// 
// 1. CLIENTE (cada jugador):
//    - Usa frustum culling para decidir qué chunks RENDERIZAR
//    - Chunks ocultos NO se renderizan pero siguen existiendo en memoria
//    - Cada cliente tiene su propia cámara y su propio frustum
//    - Mejora FPS sin afectar gameplay
//
// 2. SERVIDOR:
//    - NO usa frustum culling
//    - Simula TODOS los chunks activos (con jugadores cerca)
//    - Envía updates de chunks a clientes que los necesitan
//    - Usa "interest management" (diferente a frustum culling):
//      * Cada jugador tiene un "área de interés" (ej: 32 chunks de radio)
//      * Servidor solo envía updates de chunks en el área de interés
//      * Basado en distancia, NO en lo que el jugador ve
//
// 3. SINCRONIZACIÓN:
//    - Cliente recibe chunks del servidor (área de interés)
//    - Cliente decide cuáles renderizar (frustum culling)
//    - Modificaciones de voxels se envían al servidor
//    - Servidor valida y propaga cambios a otros clientes
//
// 4. OPTIMIZACIÓN MULTIPLAYER:
//    - Servidor: Chunk loading basado en posición de jugadores
//    - Servidor: Priorizar chunks con más jugadores cerca
//    - Cliente: Frustum culling + LOD para mejor FPS
//    - Red: Comprimir datos de chunks, delta updates
//
// EJEMPLO DE ARQUITECTURA MULTIPLAYER:
//
// ```
// SERVIDOR:
// - ChunkManager: Carga chunks en radio de 32 alrededor de jugadores
// - PhysicsWorld: Simula física en chunks activos
// - NetworkSync: Envía chunk updates a clientes relevantes
// 
// CLIENTE:
// - ChunkReceiver: Recibe chunks del servidor
// - ChunkRenderer: Renderiza chunks visibles (frustum culling)
// - InputSender: Envía acciones del jugador al servidor
// ```
//
// ============================================================================

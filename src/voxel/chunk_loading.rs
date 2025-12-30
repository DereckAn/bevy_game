//! Sistema de carga dinámica de chunks
//! Genera y elimina chunks según la posición del jugador

use bevy::prelude::*;
use std::collections::HashSet;
use crate::{
    core::BASE_CHUNK_SIZE,
    player::Player,
    voxel::{BaseChunk, ChunkMap, ChunkLOD, greedy_mesh_basechunk_simple},
    physics::{RigidBody, create_terrain_collider},
};

/// Radio de carga de chunks (en chunks, no metros)
pub const CHUNK_LOAD_RADIUS: i32 = 15;

/// Radio de descarga de chunks (debe ser mayor que LOAD_RADIUS)
pub const CHUNK_UNLOAD_RADIUS: i32 = 20;

/// Máximo de chunks a generar por frame (para evitar lag)
pub const MAX_CHUNKS_PER_FRAME: usize = 4;

/// Máximo de chunks a eliminar por frame
pub const MAX_CHUNKS_TO_UNLOAD_PER_FRAME: usize = 2;

/// Recurso que rastrea qué chunks necesitan ser cargados
#[derive(Resource, Default)]
pub struct ChunkLoadQueue {
    pub to_load: Vec<IVec3>,
    pub to_unload: Vec<Entity>,
    pub last_player_chunk: IVec3,
}

/// Sistema que detecta cuando el jugador se mueve y actualiza la cola de carga
pub fn update_chunk_load_queue(
    player_query: Query<&Transform, With<Player>>,
    chunk_map: Res<ChunkMap>,
    mut load_queue: ResMut<ChunkLoadQueue>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    // Convertir posición del jugador a coordenadas de chunk
    let player_chunk = world_pos_to_chunk_pos(player_transform.translation);

    // Solo actualizar si el jugador cambió de chunk
    if player_chunk == load_queue.last_player_chunk {
        return;
    }

    load_queue.last_player_chunk = player_chunk;

    // Determinar qué chunks deberían estar cargados
    let mut chunks_needed: HashSet<IVec3> = HashSet::new();
    
    for cx in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
        for cz in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
            let chunk_pos = IVec3::new(
                player_chunk.x + cx,
                0, // Por ahora solo Y=0
                player_chunk.z + cz,
            );
            
            // Verificar si está dentro del radio (circular, no cuadrado)
            let distance_sq = cx * cx + cz * cz;
            if distance_sq <= CHUNK_LOAD_RADIUS * CHUNK_LOAD_RADIUS {
                chunks_needed.insert(chunk_pos);
            }
        }
    }

    // Encontrar chunks que necesitan ser cargados
    load_queue.to_load.clear();
    for chunk_pos in &chunks_needed {
        if !chunk_map.chunks.contains_key(chunk_pos) {
            load_queue.to_load.push(*chunk_pos);
        }
    }

    // Ordenar por distancia al jugador (cargar los más cercanos primero)
    load_queue.to_load.sort_by_key(|pos| {
        let dx = pos.x - player_chunk.x;
        let dz = pos.z - player_chunk.z;
        dx * dx + dz * dz
    });

    // Encontrar chunks que necesitan ser descargados
    load_queue.to_unload.clear();
    for (chunk_pos, &entity) in chunk_map.chunks.iter() {
        let dx = chunk_pos.x - player_chunk.x;
        let dz = chunk_pos.z - player_chunk.z;
        let distance_sq = dx * dx + dz * dz;
        
        if distance_sq > CHUNK_UNLOAD_RADIUS * CHUNK_UNLOAD_RADIUS {
            load_queue.to_unload.push(entity);
        }
    }

    if !load_queue.to_load.is_empty() {
        info!("Need to load {} chunks", load_queue.to_load.len());
    }
    if !load_queue.to_unload.is_empty() {
        info!("Need to unload {} chunks", load_queue.to_unload.len());
    }
}

/// Sistema que carga chunks de la cola
pub fn load_chunks_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunk_map: ResMut<ChunkMap>,
    mut load_queue: ResMut<ChunkLoadQueue>,
) {
    // Cargar hasta MAX_CHUNKS_PER_FRAME chunks por frame
    let chunks_to_load = load_queue.to_load.len().min(MAX_CHUNKS_PER_FRAME);
    
    for _ in 0..chunks_to_load {
        if let Some(chunk_pos) = load_queue.to_load.pop() {
            // Verificar que no se haya cargado mientras tanto
            if chunk_map.chunks.contains_key(&chunk_pos) {
                continue;
            }

            // Generar el chunk
            let base_chunk = BaseChunk::new(chunk_pos);
            let mesh = greedy_mesh_basechunk_simple(&base_chunk);

            // Crear entidad
            let chunk_entity = commands.spawn((
                Mesh3d(meshes.add(mesh.clone())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: ChunkLOD::Ultra.debug_color(),
                    cull_mode: None,
                    ..default()
                })),
                Transform::default(),
                base_chunk,
                ChunkLOD::Ultra,
                RigidBody::Fixed,
                create_terrain_collider(&mesh),
            )).id();

            chunk_map.chunks.insert(chunk_pos, chunk_entity);
            
            info!("Loaded chunk at {:?}", chunk_pos);
        }
    }
}

/// Sistema que descarga chunks lejanos
pub fn unload_chunks_system(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    chunk_query: Query<&BaseChunk>,
) {
    // Descargar hasta MAX_CHUNKS_TO_UNLOAD_PER_FRAME chunks por frame
    let chunks_to_unload = load_queue.to_unload.len().min(MAX_CHUNKS_TO_UNLOAD_PER_FRAME);
    
    for _ in 0..chunks_to_unload {
        if let Some(entity) = load_queue.to_unload.pop() {
            // Obtener posición del chunk antes de eliminarlo
            if let Ok(base_chunk) = chunk_query.get(entity) {
                let chunk_pos = base_chunk.position;
                
                // Eliminar del mapa
                chunk_map.chunks.remove(&chunk_pos);
                
                // Despawnear entidad
                commands.entity(entity).despawn();
                
                info!("Unloaded chunk at {:?}", chunk_pos);
            }
        }
    }
}

/// Convierte posición mundial a posición de chunk
fn world_pos_to_chunk_pos(world_pos: Vec3) -> IVec3 {
    let chunk_size_meters = BASE_CHUNK_SIZE as f32 * 0.1; // VOXEL_SIZE = 0.1
    
    IVec3::new(
        (world_pos.x / chunk_size_meters).floor() as i32,
        0, // Por ahora solo Y=0
        (world_pos.z / chunk_size_meters).floor() as i32,
    )
}

//! Sistema de carga dinámica de chunks LOD distantes
//! Gestiona chunks visuales sin colisión para renderizado a larga distancia

use crate::{
    core::{CHUNK_SIZE, LOD_CHUNK_RADIUS, MAX_LOD_CHUNKS_IN_MEMORY, MAX_LOD_CHUNKS_PER_FRAME},
    player::Player,
    voxel::{LodChunk, LodLevel},
};
use bevy::prelude::*;
use std::collections::HashMap;

/// Mapa de chunks LOD activos en el mundo
/// Similar a ChunkMap pero para chunks distantes
#[derive(Resource, Default)]
pub struct LodChunkMap {
    /// Chunks LOD indexados por posición
    pub chunks: HashMap<IVec3, Entity>,
}

/// Cola de chunks LOD pendientes de carga/descarga
#[derive(Resource, Default)]
pub struct LodChunkLoadQueue {
    /// Chunks LOD que necesitan ser generados
    pub to_load: Vec<IVec3>,

    /// Chunks LOD que necesitan ser eliminados
    pub to_unload: Vec<Entity>,

    /// Última posición del jugador (en coordenadas de chunk)
    pub last_player_chunk: IVec3,

    /// Total de chunks LOD cargados
    pub total_loaded: usize,
}

impl LodChunkLoadQueue {
    /// Determina qué chunks LOD deberían estar cargados según la posición del jugador
    pub fn update_needed_chunks(
        &mut self,
        player_chunk: IVec3,
        lod_chunk_map: &LodChunkMap,
        real_chunk_radius: i32,
    ) {
        // Solo actualizar si el jugador cambió de chunk
        if player_chunk == self.last_player_chunk {
            return;
        }

        self.last_player_chunk = player_chunk;

        // Determinar qué chunks LOD deberían estar cargados
        let mut chunks_needed: std::collections::HashSet<IVec3> = std::collections::HashSet::new();

        // Solo generar chunks LOD en Y=0 (superficie)
        // Esto es suficiente para el efecto Distant Horizons
        let lod_y = 0;

        // Generar chunks LOD en patrón circular
        for cx in -LOD_CHUNK_RADIUS..=LOD_CHUNK_RADIUS {
            for cz in -LOD_CHUNK_RADIUS..=LOD_CHUNK_RADIUS {
                let distance_sq = cx * cx + cz * cz;

                // Solo generar chunks LOD fuera del radio de chunks reales
                // y dentro del radio LOD
                if distance_sq > real_chunk_radius * real_chunk_radius
                    && distance_sq <= LOD_CHUNK_RADIUS * LOD_CHUNK_RADIUS
                {
                    let chunk_pos = IVec3::new(player_chunk.x + cx, lod_y, player_chunk.z + cz);
                    chunks_needed.insert(chunk_pos);
                }
            }
        }

        // Encontrar chunks que necesitan ser cargados
        self.to_load.clear();
        for chunk_pos in &chunks_needed {
            if !lod_chunk_map.chunks.contains_key(chunk_pos) {
                self.to_load.push(*chunk_pos);
            }
        }

        // Ordenar por distancia al jugador (cargar los más cercanos primero)
        let player_pos = player_chunk;
        self.to_load.sort_by_key(|pos| {
            let dx = pos.x - player_pos.x;
            let dz = pos.z - player_pos.z;
            dx * dx + dz * dz
        });

        // Encontrar chunks LOD que están demasiado lejos
        self.to_unload.clear();
        for (chunk_pos, &entity) in &lod_chunk_map.chunks {
            let dx = chunk_pos.x - player_chunk.x;
            let dz = chunk_pos.z - player_chunk.z;
            let distance_sq = dx * dx + dz * dz;

            // Descargar si está fuera del radio LOD
            if distance_sq > LOD_CHUNK_RADIUS * LOD_CHUNK_RADIUS {
                self.to_unload.push(entity);
            }
        }

        // Limitar memoria: si tenemos demasiados chunks LOD, eliminar los más lejanos
        if lod_chunk_map.chunks.len() > MAX_LOD_CHUNKS_IN_MEMORY {
            let excess = lod_chunk_map.chunks.len() - MAX_LOD_CHUNKS_IN_MEMORY;

            // Ordenar chunks por distancia y eliminar los más lejanos
            let mut chunks_by_distance: Vec<_> = lod_chunk_map.chunks.iter().collect();
            chunks_by_distance.sort_by_key(|(pos, _)| {
                let dx = pos.x - player_chunk.x;
                let dz = pos.z - player_chunk.z;
                -(dx * dx + dz * dz) // Negativo para ordenar de más lejano a más cercano
            });

            for (_, entity) in chunks_by_distance.iter().take(excess) {
                if !self.to_unload.contains(&entity) {
                    self.to_unload.push(**entity);
                }
            }
        }
    }
}

/// Convierte posición mundial a posición de chunk
pub fn world_pos_to_chunk_pos(world_pos: Vec3) -> IVec3 {
    let chunk_size_meters = 32.0 * 0.1; // BASE_CHUNK_SIZE * VOXEL_SIZE

    IVec3::new(
        (world_pos.x / chunk_size_meters).floor() as i32,
        (world_pos.y / chunk_size_meters).floor() as i32,
        (world_pos.z / chunk_size_meters).floor() as i32,
    )
}


// ============================================================================
// SISTEMAS BEVY
// ============================================================================


// Sistemas que actualiza la cola de chunk sLOD segun la posicion del jugador
pub fn update_lod_chunk_queue_system(
    player_query: Query<&Transform, With<Player>>,
    lod_chunk_map: Res<LodChunkMap>,
    mut load_queue: ResMut<LodChunkLoadQueue>
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    // convertir posicion del jugaodr a coordenadas de chunk 
    let player_chunk = world_pos_to_chunk_pos(player_transform.translation);

    // Actualizar que chunks LOD deverian estar cargados
    // Usamos  REAL_CHUNK_RAIDUS como el radio de chunks reales
    load_queue.update_needed_chunks(player_chunk, &lod_chunk_map, CHUNK_SIZE as i32);
}


/// Sistema que genera chunks LOD asíncronamente
pub fn load_lod_chunks_system(
    mut commands: Commands,
    mut lod_chunk_map: ResMut<LodChunkMap>,
    mut load_queue: ResMut<LodChunkLoadQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    use crate::voxel::{mesh_lod_chunk, TerrainGenerator};
    
    // Generar hasta MAX_LOD_CHUNKS_PER_FRAME chunks por frame
    let chunks_to_load = load_queue.to_load.len().min(MAX_LOD_CHUNKS_PER_FRAME);
    
    for _ in 0..chunks_to_load {
        if let Some(chunk_pos) = load_queue.to_load.pop() {
            // Verificar que no se haya cargado mientras tanto
            if lod_chunk_map.chunks.contains_key(&chunk_pos) {
                continue;
            }
            
            // Determinar nivel LOD basado en distancia al jugador
            let distance_chunks = (chunk_pos.x.pow(2) + chunk_pos.z.pow(2)) as f32;
            let distance_chunks = distance_chunks.sqrt() as i32;
            let lod_level = LodLevel::from_distance(distance_chunks);
            
            // Generar chunk LOD
            let mut lod_chunk = LodChunk::new(chunk_pos, lod_level);
            let mut terrain_gen = TerrainGenerator::new(12345); // Mismo seed que chunks reales
            lod_chunk.generate_surface(&mut terrain_gen);
            
            // Generar mesh
            let mesh = mesh_lod_chunk(&lod_chunk);
            
            // Solo crear entidad si el mesh tiene vértices
            if mesh.count_vertices() > 0 {
                // Color según nivel LOD (para debug)
                let color = match lod_level {
                    LodLevel::Medium => Color::srgb(0.4, 0.6, 0.8),  // Azul claro
                    LodLevel::Low => Color::srgb(0.5, 0.5, 0.7),     // Gris azulado
                    LodLevel::Minimal => Color::srgb(0.6, 0.6, 0.6), // Gris
                };
                
                let chunk_entity = commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        cull_mode: None,
                        ..default()
                    })),
                    Transform::default(),
                    lod_chunk,
                )).id();
                
                lod_chunk_map.chunks.insert(chunk_pos, chunk_entity);
                load_queue.total_loaded += 1;
            }
        }
    }
}
/// Sistema que descarga chunks LOD lejanos
pub fn unload_lod_chunks_system(
    mut commands: Commands,
    mut lod_chunk_map: ResMut<LodChunkMap>,
    mut load_queue: ResMut<LodChunkLoadQueue>,
    chunk_query: Query<&LodChunk>,
) {
    // Descargar chunks en la cola de descarga
    for entity in load_queue.to_unload.drain(..) {
        if let Ok(lod_chunk) = chunk_query.get(entity) {
            let chunk_pos = lod_chunk.position;
            lod_chunk_map.chunks.remove(&chunk_pos);
            commands.entity(entity).despawn();
        }
    }
}
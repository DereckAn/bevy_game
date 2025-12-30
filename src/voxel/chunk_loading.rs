//! Sistema de carga dinámica de chunks
//! Genera y elimina chunks según la posición del jugador
//! Usa generación asíncrona para evitar lag con grandes distancias de renderizado
//! Incluye caché persistente en disco

use bevy::{prelude::*, tasks::{AsyncComputeTaskPool, Task}};
use std::collections::HashSet;
use futures_lite::future;
use crate::{
    core::BASE_CHUNK_SIZE,
    player::Player,
    voxel::{
        BaseChunk, ChunkMap, ChunkLOD, greedy_mesh_basechunk_simple, ChunkOctree,
    },
    physics::{RigidBody, create_terrain_collider},
};

/// Radio de carga de chunks (en chunks, no metros)
/// Reducido para mejor rendimiento con chunks verticales
pub const CHUNK_LOAD_RADIUS: i32 = 16;

/// Radio de descarga de chunks (debe ser mayor que LOAD_RADIUS)
pub const CHUNK_UNLOAD_RADIUS: i32 = 20;

/// Máximo de chunks a generar por frame (reducido para mejor FPS)
pub const MAX_CHUNKS_PER_FRAME: usize = 32;

/// Máximo de chunks a eliminar por frame
pub const MAX_CHUNKS_TO_UNLOAD_PER_FRAME: usize = 16;

/// Recurso que rastrea qué chunks necesitan ser cargados
#[derive(Resource, Default)]
pub struct ChunkLoadQueue {
    pub to_load: Vec<IVec3>,
    pub to_unload: Vec<Entity>,
    pub last_player_chunk: IVec3,
    pub total_loaded: usize,
    pub last_log_time: f32,
}

/// Componente para chunks que están siendo generados asíncronamente
#[derive(Component)]
pub struct ChunkGenerationTask {
    pub task: Task<(IVec3, BaseChunk, Mesh)>,
}

/// Sistema que detecta cuando el jugador se mueve y actualiza la cola de carga
pub fn update_chunk_load_queue(
    player_query: Query<&Transform, With<Player>>,
    chunk_map: Res<ChunkMap>,
    _octree: Res<ChunkOctree>,
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

    // Determinar qué chunks deberían estar cargados (incluyendo verticales)
    let mut chunks_needed: HashSet<IVec3> = HashSet::new();
    
    // Rango vertical reducido: desde -1 hasta +3 chunks (mejor rendimiento)
    let y_min = -1;
    let y_max = 3;
    
    for cx in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
        for cz in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
            // Verificar si está dentro del radio (circular, no cuadrado)
            let distance_sq = cx * cx + cz * cz;
            if distance_sq <= CHUNK_LOAD_RADIUS * CHUNK_LOAD_RADIUS {
                // Generar chunks en múltiples niveles verticales
                for cy in y_min..=y_max {
                    let chunk_pos = IVec3::new(
                        player_chunk.x + cx,
                        cy,
                        player_chunk.z + cz,
                    );
                    chunks_needed.insert(chunk_pos);
                }
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
    let player_pos = player_chunk;
    load_queue.to_load.sort_by_key(|pos| {
        let dx = pos.x - player_pos.x;
        let dy = pos.y - player_pos.y;
        let dz = pos.z - player_pos.z;
        dx * dx + dy * dy + dz * dz
    });

    // Verificar cuáles chunks están fuera del radio de descarga
    load_queue.to_unload.clear();
    
    for chunk_pos in chunk_map.chunks.keys() {
        let dx = chunk_pos.x - player_chunk.x;
        let _dy = chunk_pos.y - player_chunk.y;
        let dz = chunk_pos.z - player_chunk.z;
        let distance_sq = dx * dx + dz * dz;
        
        // Descargar si está fuera del radio horizontal O fuera del rango vertical
        if distance_sq > CHUNK_UNLOAD_RADIUS * CHUNK_UNLOAD_RADIUS 
            || chunk_pos.y < y_min 
            || chunk_pos.y > y_max {
            if let Some(&entity) = chunk_map.chunks.get(chunk_pos) {
                load_queue.to_unload.push(entity);
            }
        }
    }
}

/// Sistema que inicia la generación asíncrona de chunks con caché
pub fn load_chunks_system(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut load_queue: ResMut<ChunkLoadQueue>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    
    // Iniciar generación de hasta MAX_CHUNKS_PER_FRAME chunks por frame
    let chunks_to_load = load_queue.to_load.len().min(MAX_CHUNKS_PER_FRAME);
    
    for _ in 0..chunks_to_load {
        if let Some(chunk_pos) = load_queue.to_load.pop() {
            // Verificar que no se haya cargado mientras tanto
            if chunk_map.chunks.contains_key(&chunk_pos) {
                continue;
            }

            // Crear entidad placeholder y marcarla como "en generación"
            let chunk_entity = commands.spawn_empty().id();
            chunk_map.chunks.insert(chunk_pos, chunk_entity);

            // Generar chunk y mesh en background thread
            // Caché deshabilitado temporalmente para mejor rendimiento
            let task = thread_pool.spawn(async move {
                let base_chunk = BaseChunk::new(chunk_pos);
                let mesh = greedy_mesh_basechunk_simple(&base_chunk);
                (chunk_pos, base_chunk, mesh)
            });

            commands.entity(chunk_entity).insert(ChunkGenerationTask { task });
        }
    }
}

/// Sistema que completa la generación de chunks cuando las tareas terminan
pub fn complete_chunk_generation_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut octree: ResMut<ChunkOctree>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut task_query: Query<(Entity, &mut ChunkGenerationTask)>,
    time: Res<Time>,
) {
    let mut completed_this_frame = 0;
    
    for (entity, mut task) in task_query.iter_mut() {
        if let Some((chunk_pos, base_chunk, mesh)) = future::block_on(future::poll_once(&mut task.task)) {
            // Verificar si el mesh tiene vértices (no está vacío)
            let has_vertices = mesh.count_vertices() > 0;
            
            // Solo agregar collider si el mesh tiene geometría
            if has_vertices {
                commands.entity(entity).insert((
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
                )).remove::<ChunkGenerationTask>();
            } else {
                // Chunk vacío (solo aire), no agregar collider
                commands.entity(entity).insert((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: ChunkLOD::Ultra.debug_color(),
                        cull_mode: None,
                        ..default()
                    })),
                    Transform::default(),
                    base_chunk,
                    ChunkLOD::Ultra,
                )).remove::<ChunkGenerationTask>();
            }

            octree.insert(chunk_pos);
            load_queue.total_loaded += 1;
            completed_this_frame += 1;
        }
    }
    
    // Log progreso cada 2 segundos
    if time.elapsed_secs() - load_queue.last_log_time > 2.0 {
        load_queue.last_log_time = time.elapsed_secs();
        let pending = task_query.iter().count() - completed_this_frame;
        let in_queue = load_queue.to_load.len();
        info!("Chunks: {} loaded, {} generating, {} in queue", 
            load_queue.total_loaded, pending, in_queue);
    }
}

/// Sistema que descarga chunks lejanos
pub fn unload_chunks_system(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut octree: ResMut<ChunkOctree>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    chunk_query: Query<Option<&BaseChunk>>,
) {
    // Descargar hasta MAX_CHUNKS_TO_UNLOAD_PER_FRAME chunks por frame
    let chunks_to_unload = load_queue.to_unload.len().min(MAX_CHUNKS_TO_UNLOAD_PER_FRAME);
    
    for _ in 0..chunks_to_unload {
        if let Some(entity) = load_queue.to_unload.pop() {
            if let Ok(maybe_chunk) = chunk_query.get(entity) {
                // Si el chunk tiene BaseChunk, eliminarlo del octree
                if let Some(base_chunk) = maybe_chunk {
                    let chunk_pos = base_chunk.position;
                    chunk_map.chunks.remove(&chunk_pos);
                    octree.remove(chunk_pos);
                }
                
                // Despawnear entidad (incluso si aún está generándose)
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Convierte posición mundial a posición de chunk
fn world_pos_to_chunk_pos(world_pos: Vec3) -> IVec3 {
    let chunk_size_meters = BASE_CHUNK_SIZE as f32 * 0.1; // VOXEL_SIZE = 0.1
    
    IVec3::new(
        (world_pos.x / chunk_size_meters).floor() as i32,
        (world_pos.y / chunk_size_meters).floor() as i32, // Ahora también calcula Y
        (world_pos.z / chunk_size_meters).floor() as i32,
    )
}

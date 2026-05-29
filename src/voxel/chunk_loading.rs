//! Sistema de carga dinámica de chunks
//! Genera y elimina chunks según la posición del jugador
//! Usa generación asíncrona para evitar lag con grandes distancias de renderizado
//! Incluye caché persistente en disco

use crate::{
    core::{BASE_CHUNK_SIZE, WORLD_CHUNK_RADIUS, WorldSeed},
    physics::{RigidBody, create_terrain_collider},
    player::Player,
    voxel::{
        self, BaseChunk, ChunkLOD, ChunkMap, LodChunk, LodLevel, SpatialHashGrid,
        TerrainGenerator, VoxelDiffs, mesh_lod_chunk,
    },
};
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use std::collections::{HashSet, VecDeque};

/// Radio de carga de chunks (en chunks, no metros)
/// Aumentado para incluir chunks LOD distantes
pub const CHUNK_LOAD_RADIUS: i32 = 64;

/// Radio de descarga de chunks (debe ser mayor que LOAD_RADIUS)
pub const CHUNK_UNLOAD_RADIUS: i32 = 70;

/// Máximo de chunks cuya GENERACIÓN async se inicia por frame.
///
/// Throttle en el origen: menos tareas iniciadas = menos remallado+collider que
/// integrar después, suavizando el frame time.
pub const MAX_CHUNKS_PER_FRAME: usize = 16;

/// Presupuesto de tiempo (ms) para integrar chunks por frame.
///
/// Acota el trabajo síncrono (remallado + collider) por wall-clock en lugar de
/// por conteo fijo. Como cada chunk cuesta distinto, un límite de tiempo evita
/// los picos de frame mucho mejor que "N chunks por frame".
pub const CHUNK_COMPLETION_BUDGET_MS: u64 = 4;

/// Máximo de tareas de generación a COMPLETAR (integrar) por frame.
///
/// Completar implica remallado con vecinos + collider Rapier en el hilo
/// principal: trabajo caro y síncrono. Acota los tirones cuando muchas tareas
/// terminan a la vez, PERO debe ser >= MAX_CHUNKS_PER_FRAME para que la
/// integración no se quede atrás de la generación (si no, se acumulan chunks
/// generados sin mesh = huecos en el terreno).
pub const MAX_CHUNK_COMPLETIONS_PER_FRAME: usize = 24;

/// Máximo de chunks a eliminar por frame
pub const MAX_CHUNKS_TO_UNLOAD_PER_FRAME: usize = 16;

/// Máximo de conversiones Real ↔ LOD por frame
pub const MAX_CHUNK_TRANSITIONS_PER_FRAME: usize = 4;

/// Distancia para convertir LOD → Real (con hysteresis)
pub const LOD_TO_REAL_DISTANCE: i32 = 30;

/// Distancia para convertir Real → LOD (con hysteresis)
pub const REAL_TO_LOD_DISTANCE: i32 = 36;

/// Tipo de chunk a generar segun distancia
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    // Chunk real con colision (0-32 chunks de distancia)
    Real,

    // Chunk LOD visiaul sin colision (32 - 200 chunks de distancia)
    Lod,
}

impl ChunkType {
    // Determina el tipo de chunk segun la distancia al jugador
    pub fn from_distance(distance_chunks: i32) -> Self {
        if distance_chunks <= 32 {
            ChunkType::Real
        } else {
            ChunkType::Lod
        }
    }
}

/// Materiales compartidos por todos los chunks.
///
/// Un puñado de handles fijos (uno por color de debug) en lugar de un
/// `StandardMaterial` nuevo por chunk: miles de materiales idénticos rompen
/// el batching del renderer y multiplican los draw calls. Además NO desactivan
/// `cull_mode`, así el GPU descarta las caras traseras (≈mitad de fragmentos).
#[derive(Resource)]
pub struct ChunkMaterials {
    /// Por nivel de `ChunkLOD` (chunks reales): Ultra, High, Medium, Low, Minimal
    real: [Handle<StandardMaterial>; 5],
    /// Por nivel de `LodLevel` (chunks LOD): Medium, Low, Minimal
    lod: [Handle<StandardMaterial>; 3],
}

impl FromWorld for ChunkMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();

        let real = [
            ChunkLOD::Ultra,
            ChunkLOD::High,
            ChunkLOD::Medium,
            ChunkLOD::Low,
            ChunkLOD::Minimal,
        ]
        .map(|lod| {
            materials.add(StandardMaterial {
                base_color: lod.debug_color(),
                ..default()
            })
        });

        // Colores de debug por nivel LOD (naranja → rojo según distancia)
        let lod = [
            Color::srgb(1.0, 0.6, 0.0), // Medium (32-64 chunks)
            Color::srgb(1.0, 0.3, 0.0), // Low (64-128)
            Color::srgb(0.8, 0.0, 0.0), // Minimal (128+)
        ]
        .map(|color| {
            materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })
        });

        Self { real, lod }
    }
}

impl ChunkMaterials {
    pub fn real_handle(&self, lod: ChunkLOD) -> Handle<StandardMaterial> {
        let idx = match lod {
            ChunkLOD::Ultra => 0,
            ChunkLOD::High => 1,
            ChunkLOD::Medium => 2,
            ChunkLOD::Low => 3,
            ChunkLOD::Minimal => 4,
        };
        self.real[idx].clone()
    }

    pub fn lod_handle(&self, level: LodLevel) -> Handle<StandardMaterial> {
        let idx = match level {
            LodLevel::Medium => 0,
            LodLevel::Low => 1,
            LodLevel::Minimal => 2,
        };
        self.lod[idx].clone()
    }
}

/// Recurso que rastrea qué chunks necesitan ser cargados
#[derive(Resource, Default)]
pub struct ChunkLoadQueue {
    // Chunks a cargar con su tipo (Real o Lod)
    pub to_load: VecDeque<(IVec3, ChunkType)>,
    pub to_unload: Vec<(IVec3, Entity)>,

    // Conversiones pendientes
    pub to_convert_to_real: Vec<Entity>, // LOD → Real
    pub to_convert_to_lod: Vec<Entity>,  // Real → LOD

    pub last_player_chunk: IVec3,
    pub total_loaded: usize,
    pub last_log_time: f32,
}

/// Componente para chunks que están siendo generados asíncronamente
#[derive(Component)]
pub struct ChunkGenerationTask {
    pub task: Task<(IVec3, BaseChunk)>,
    /// Posición del chunk, para ordenar la integración por cercanía al jugador
    /// SIN tener que pollear la tarea primero.
    pub chunk_pos: IVec3,
}

/// Destruye el mundo y reinicia los recursos de chunks.
///
/// Se ejecuta al volver al menú principal (desde InGame o Paused) para que una
/// nueva partida arranque limpia, sin chunks ni luces duplicadas.
pub fn teardown_world(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut spatial_hash: ResMut<SpatialHashGrid>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    chunks: Query<Entity, Or<(With<BaseChunk>, With<LodChunk>, With<ChunkGenerationTask>)>>,
    lights: Query<Entity, With<DirectionalLight>>,
    mut voxel_diffs: ResMut<VoxelDiffs>,
) {
    // Despawnear chunks vía queries: solo devuelven entidades vivas, así
    // evitamos intentar destruir IDs obsoletos guardados en chunk_map.
    for entity in &chunks {
        commands.entity(entity).despawn();
    }
    for entity in &lights {
        commands.entity(entity).despawn();
    }

    chunk_map.chunks.clear();
    voxel_diffs.chunks.clear();
    spatial_hash.clear();
    *load_queue = ChunkLoadQueue::default();
}

/// Sistema que detecta cuando el jugador se mueve y actualiza la cola de carga
pub fn update_chunk_load_queue(
    player_query: Query<&Transform, With<Player>>,
    chunk_map: Res<ChunkMap>,
    spatial_hash: Res<SpatialHashGrid>,
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
    // OPTIMIZACIÓN: Usar algoritmo eficiente para generar círculo
    let mut chunks_needed: HashSet<IVec3> = HashSet::new();

    // Rango vertical reducido: desde -1 hasta +3 chunks (mejor rendimiento)
    let y_min = -1;
    let y_max = 3;

    // OPTIMIZACIÓN: Generar círculo de chunks de manera eficiente
    // En lugar de iterar cuadrado completo, solo generar puntos dentro del círculo
    let radius_sq = CHUNK_LOAD_RADIUS * CHUNK_LOAD_RADIUS;

    for cy in y_min..=y_max {
        // Usar simetría del círculo para reducir cálculos
        for cx in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
            // Calcular el rango Z válido para este X (usando la ecuación del círculo)
            let x_sq = cx * cx;
            if x_sq > radius_sq {
                continue; // Este X está fuera del círculo
            }

            // Calcular el máximo Z para este X: z² <= r² - x²
            let max_z_sq = radius_sq - x_sq;
            let max_z = (max_z_sq as f32).sqrt() as i32;

            // Solo iterar en el rango válido de Z
            for cz in -max_z..=max_z {
                let chunk_pos = IVec3::new(player_chunk.x + cx, cy, player_chunk.z + cz);
                // Mapa finito: no generar nada fuera del límite del mundo
                if chunk_pos.x.abs() > WORLD_CHUNK_RADIUS || chunk_pos.z.abs() > WORLD_CHUNK_RADIUS
                {
                    continue;
                }
                chunks_needed.insert(chunk_pos);
            }
        }
    }

    // Encontrar chunks que necesitan ser cargados
    let mut to_load_vec: Vec<(IVec3, ChunkType)> = Vec::new();
    for chunk_pos in &chunks_needed {
        if !chunk_map.chunks.contains_key(chunk_pos) {
            // Calcular distancia al jugador (solo horizontal x, z)
            let dx = chunk_pos.x - player_chunk.x;
            let dz = chunk_pos.z - player_chunk.z;
            let distance_chunks = ((dx * dx + dz * dz) as f32).sqrt() as i32;

            // Determinar tipo de chunk segun distancia
            let chunk_type = ChunkType::from_distance(distance_chunks);

            // Los LOD son heightmaps con alturas ABSOLUTAS (ignoran position.y):
            // un solo chunk en y=0 representa la columna entera. Cargar los
            // demás niveles Y produciría 5 meshes idénticos apilados.
            if chunk_type == ChunkType::Lod && chunk_pos.y != 0 {
                continue;
            }

            to_load_vec.push((*chunk_pos, chunk_type));
        }
    }

    // Ordenar por distancia al jugador (cargar los más cercanos primero)
    let player_pos = player_chunk;
    to_load_vec.sort_by_key(|(pos, _chunk_type)| {
        let dx = pos.x - player_pos.x;
        let dy = pos.y - player_pos.y;
        let dz = pos.z - player_pos.z;
        dx * dx + dy * dy + dz * dz
    });

    load_queue.to_load = VecDeque::from(to_load_vec);

    // Verificar cuáles chunks están fuera del radio de descarga
    // OPTIMIZACIÓN: Usar Spatial Hash Grid con distancia HORIZONTAL (2D)
    load_queue.to_unload.clear();

    // Usar spatial hash para encontrar chunks DENTRO del radio horizontal
    let chunks_to_keep = spatial_hash.query_radius_horizontal(player_chunk, CHUNK_UNLOAD_RADIUS);

    // Filtrar por rango vertical y convertir a HashSet para búsqueda O(1)
    let keep_set: HashSet<IVec3> = chunks_to_keep
        .into_iter()
        .filter(|pos| pos.y >= y_min && pos.y <= y_max)
        .collect();

    // Descargar chunks que NO están en el set de chunks a mantener
    for (chunk_pos, &entity) in &chunk_map.chunks {
        if !keep_set.contains(chunk_pos) {
            load_queue.to_unload.push((*chunk_pos, entity));
        }
    }
}

/// Sistema que inicia la generación asíncrona de chunks con caché
pub fn load_chunks_system(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut spatial_hash: ResMut<SpatialHashGrid>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_materials: Res<ChunkMaterials>,
    world_seed: Res<WorldSeed>,
    voxel_diffs: Res<VoxelDiffs>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let seed = world_seed.0;

    // Iniciar generación de hasta MAX_CHUNKS_PER_FRAME chunks por frame
    let chunks_to_load = load_queue.to_load.len().min(MAX_CHUNKS_PER_FRAME);

    for _ in 0..chunks_to_load {
        if let Some((chunk_pos, chunk_type)) = load_queue.to_load.pop_front() {
            // Verificar que no se haya cargado mientras tanto
            if chunk_map.chunks.contains_key(&chunk_pos) {
                continue;
            }

            // Crear entidad placeholder y marcarla como "en generación"
            let chunk_entity = commands.spawn_empty().id();
            chunk_map.chunks.insert(chunk_pos, chunk_entity);

            // Agregar al spatial hash para búsquedas rápidas
            spatial_hash.insert(chunk_pos);

            // Genera chunk segun tipo
            match chunk_type {
                ChunkType::Real => {
                    // Copia los diffs de ESTE chunk antes de lanzar la tarea
                    let chunk_diffs = voxel_diffs.chunks.get(&chunk_pos).cloned();

                    let task = thread_pool.spawn(async move {
                        // El mallado se hace con vecinos en complete_chunk_generation_system
                        let mut base_chunk = BaseChunk::new(chunk_pos, seed);

                        if let Some(diffs) = &chunk_diffs {
                            base_chunk.apply_diffs(diffs);
                        }
                        (chunk_pos, base_chunk)
                    });

                    commands
                        .entity(chunk_entity)
                        .insert(ChunkGenerationTask { task, chunk_pos });
                }

                ChunkType::Lod => {
                    // chunk Lod solo superficie (generacion sincrona por ahora)
                    let delta = chunk_pos - load_queue.last_player_chunk;
                    let distance_chunks = ((delta.x.pow(2) + delta.z.pow(2)) as f32).sqrt() as i32;
                    let lod_level = LodLevel::from_distance(distance_chunks);

                    let mut lod_chunk = LodChunk::new(chunk_pos, lod_level);
                    let mut terrain_gen = TerrainGenerator::new(seed); // Mismo seed del mundo
                    lod_chunk.generate_surface(&mut terrain_gen);

                    let mesh = mesh_lod_chunk(&lod_chunk);

                    // Solo renderizar si el mesh tiene vértices
                    if mesh.count_vertices() > 0 {
                        // Insertar componentes para renderizado (SIN colisión)
                        commands.entity(chunk_entity).insert((
                            Mesh3d(meshes.add(mesh)),
                            MeshMaterial3d(chunk_materials.lod_handle(lod_level)),
                            Transform::default(),
                            lod_chunk,
                            ChunkLOD::from_distance(distance_chunks as f32),
                        ));

                        // Agregar mesh y material después
                        load_queue.total_loaded += 1;
                    } else {
                        // Chunk LOD vacío, despawnear
                        commands.entity(chunk_entity).despawn();
                        chunk_map.chunks.remove(&chunk_pos);
                    }
                }
            }
        }
    }
}

/// Sistema que completa la generación de chunks cuando las tareas terminan
pub fn complete_chunk_generation_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_materials: Res<ChunkMaterials>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut task_query: Query<(Entity, &mut ChunkGenerationTask)>,
    chunk_map: Res<ChunkMap>,
    base_chunks: Query<&BaseChunk>,
    player_query: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    use crate::voxel::greedy_mesh_basechunk;

    // Posición del jugador en chunks: para integrar primero los huecos cercanos.
    let player_chunk = player_query
        .single()
        .map(|t| world_pos_to_chunk_pos(t.translation))
        .unwrap_or(IVec3::ZERO);

    // Ordenar las tareas por cercanía HORIZONTAL al jugador. El orden de
    // iter() del ECS es arbitrario; sin esto, dentro del presupuesto de tiempo
    // se podían integrar chunks lejanos antes que los huecos junto al jugador.
    let mut pending: Vec<(Entity, IVec3)> =
        task_query.iter().map(|(e, t)| (e, t.chunk_pos)).collect();
    pending.sort_by_key(|(_, pos)| {
        let dx = pos.x - player_chunk.x;
        let dz = pos.z - player_chunk.z;
        dx * dx + dz * dz
    });

    let mut completed_this_frame = 0;
    let start = std::time::Instant::now();

    for (entity, _) in pending {
        // Cortar por presupuesto de tiempo O por conteo máximo, lo que ocurra primero.
        if completed_this_frame >= MAX_CHUNK_COMPLETIONS_PER_FRAME
            || start.elapsed() >= std::time::Duration::from_millis(CHUNK_COMPLETION_BUDGET_MS)
        {
            break;
        }

        let Ok((_, mut task)) = task_query.get_mut(entity) else {
            continue;
        };

        if let Some((chunk_pos, base_chunk)) = future::block_on(future::poll_once(&mut task.task)) {
            // Regenerar mesh CON verificación de vecinos para eliminar gaps
            let mesh = greedy_mesh_basechunk(&base_chunk, &chunk_map, &base_chunks);

            // Verificar si el mesh tiene vértices (no está vacío)
            let has_vertices = mesh.count_vertices() > 0;

            // Solo agregar collider si el mesh tiene geometría
            if has_vertices {
                // Construir el collider PRIMERO (toma &mesh prestado), luego MOVER elmesh
                // a Assets sin clonar su buffer de vértices.
                let collider = create_terrain_collider(&mesh);
                commands
                    .entity(entity)
                    .insert((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(chunk_materials.real_handle(ChunkLOD::Ultra)),
                        Transform::default(),
                        base_chunk,
                        ChunkLOD::Ultra,
                        RigidBody::Fixed,
                        collider,
                    ))
                    .remove::<ChunkGenerationTask>();
            } else {
                // Chunk vacío (solo aire), no agregar collider
                commands
                    .entity(entity)
                    .insert((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(chunk_materials.real_handle(ChunkLOD::Ultra)),
                        Transform::default(),
                        base_chunk,
                        ChunkLOD::Ultra,
                    ))
                    .remove::<ChunkGenerationTask>();
            }

            load_queue.total_loaded += 1;
            completed_this_frame += 1;
        }
    }

    // Log progreso cada 2 segundos
    if time.elapsed_secs() - load_queue.last_log_time > 2.0 {
        load_queue.last_log_time = time.elapsed_secs();
        let pending = task_query.iter().count() - completed_this_frame;
        let in_queue = load_queue.to_load.len();
        info!(
            "Chunks: {} loaded, {} generating, {} in queue",
            load_queue.total_loaded, pending, in_queue
        );
    }
}

/// Sistema que descarga chunks lejanos
pub fn unload_chunks_system(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut spatial_hash: ResMut<SpatialHashGrid>,
    mut load_queue: ResMut<ChunkLoadQueue>,
) {
    // Descargar hasta MAX_CHUNKS_TO_UNLOAD_PER_FRAME chunks por frame
    let chunks_to_unload = load_queue
        .to_unload
        .len()
        .min(MAX_CHUNKS_TO_UNLOAD_PER_FRAME);

    for _ in 0..chunks_to_unload {
        if let Some((chunk_pos, entity)) = load_queue.to_unload.pop() {
            // Limpiar SIEMPRE los registros, sea Real, LOD o aún generándose.
            // Si no, el chunk_map conserva una key fantasma y load_chunks_system
            // nunca vuelve a cargar esa posición (hueco permanente).
            chunk_map.chunks.remove(&chunk_pos);
            spatial_hash.remove(chunk_pos);
            commands.entity(entity).despawn();
        }
    }
}

/// Convierte posición mundial a posición de chunk
fn world_pos_to_chunk_pos(world_pos: Vec3) -> IVec3 {
    let chunk_size_meters = BASE_CHUNK_SIZE as f32 * 0.1; // VOXEL_SIZE = 0.1

    IVec3::new(
        (world_pos.x / chunk_size_meters).floor() as i32,
        (world_pos.y / chunk_size_meters).floor() as i32, // calcula Y
        (world_pos.z / chunk_size_meters).floor() as i32,
    )
}

/// Sistema que detecta chunks que necesitan convertirse entre Real y LOD
pub fn update_chunk_transitions_system(
    player_query: Query<&Transform, With<Player>>,
    chunk_map: Res<ChunkMap>,
    base_chunk_query: Query<&BaseChunk>,
    lod_chunk_query: Query<&LodChunk>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut last_chunk: Local<IVec3>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let player_chunk = world_pos_to_chunk_pos(player_transform.translation);

    // Las conversiones solo cambian cuando el jujgador cambia de chunk
    // Estando quieto, lsos sitemas de conversion siguien drenando las colas.
    if player_chunk == *last_chunk {
        return;
    }
    *last_chunk = player_chunk;

    // Limpiar colas de conversión
    load_queue.to_convert_to_real.clear();
    load_queue.to_convert_to_lod.clear();

    // Revisar todos los chunks cargados
    for (chunk_pos, &entity) in &chunk_map.chunks {
        // Calcular distancia horizontal al jugador (ignorar Y)
        let dx = chunk_pos.x - player_chunk.x;
        let dz = chunk_pos.z - player_chunk.z;
        let distance_sq = dx * dx + dz * dz;

        if base_chunk_query.get(entity).is_ok() {
            if distance_sq > REAL_TO_LOD_DISTANCE * REAL_TO_LOD_DISTANCE {
                load_queue.to_convert_to_lod.push(entity);
            }
        } else if lod_chunk_query.get(entity).is_ok() {
            if distance_sq < LOD_TO_REAL_DISTANCE * LOD_TO_REAL_DISTANCE {
                load_queue.to_convert_to_real.push(entity);
            }
        }
    }
}

/// Sistema que ejecuta las conversiones LOD → Real
pub fn convert_lod_to_real_system(
    mut commands: Commands,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut chunk_map: ResMut<ChunkMap>,
    lod_query: Query<&LodChunk>,
    world_seed: Res<WorldSeed>,
    voxel_diffs: Res<VoxelDiffs>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let seed = world_seed.0;

    // Procesar hasta MAX_CHUNK_TRANSITIONS_PER_FRAME conversiones
    let conversions_to_do = load_queue
        .to_convert_to_real
        .len()
        .min(MAX_CHUNK_TRANSITIONS_PER_FRAME);

    for _ in 0..conversions_to_do {
        if let Some(entity) = load_queue.to_convert_to_real.pop() {
            if let Ok(lod_chunk) = lod_query.get(entity) {
                let chunk_pos = lod_chunk.position;
                let chunk_diffs = voxel_diffs.chunks.get(&chunk_pos).cloned();

                // Generar BaseChunk asíncronamente
                let task = thread_pool.spawn(async move {
                    let mut base_chunk = BaseChunk::new(chunk_pos, seed);
                    if let Some(diffs) = chunk_diffs {
                        base_chunk.apply_diffs(&diffs);
                    }
                    (chunk_pos, base_chunk)
                });

                // Despawnear el LOD chunk y crear tarea de generación
                commands.entity(entity).despawn();

                // Crear nueva entidad con la tarea
                let new_entity = commands.spawn(ChunkGenerationTask { task, chunk_pos }).id();

                // Actualizar ChunkMap para que apunte a la nueva entidad
                chunk_map.chunks.insert(chunk_pos, new_entity);

                info!("Converting LOD → Real at {:?}", chunk_pos);
            }
        }
    }
}

/// Sistema que ejecuta las conversiones Real → LOD
pub fn convert_real_to_lod_system(
    mut commands: Commands,
    mut load_queue: ResMut<ChunkLoadQueue>,
    base_query: Query<&BaseChunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_materials: Res<ChunkMaterials>,
    mut chunk_map: ResMut<ChunkMap>,
    mut spatial_hash: ResMut<SpatialHashGrid>,
    world_seed: Res<WorldSeed>,
) {
    // Procesar hasta MAX_CHUNK_TRANSITIONS_PER_FRAME conversiones
    let conversions_to_do = load_queue
        .to_convert_to_lod
        .len()
        .min(MAX_CHUNK_TRANSITIONS_PER_FRAME);

    for _ in 0..conversions_to_do {
        if let Some(entity) = load_queue.to_convert_to_lod.pop() {
            if let Ok(base_chunk) = base_query.get(entity) {
                let chunk_pos = base_chunk.position;

                // Solo la columna y=0 se convierte en LOD (heightmap con
                // alturas absolutas que representa la columna entera); los
                // demás niveles Y simplemente se descartan.
                if chunk_pos.y != 0 {
                    commands.entity(entity).despawn();
                    chunk_map.chunks.remove(&chunk_pos);
                    spatial_hash.remove(chunk_pos);
                    continue;
                }

                // Calcular distancia al jugador para determinar nivel LOD
                let delta = chunk_pos - load_queue.last_player_chunk;
                let distance_chunks =
                    ((delta.x * delta.x + delta.z * delta.z) as f32).sqrt() as i32;
                let lod_level = LodLevel::from_distance(distance_chunks);

                // Regenerar la superficie desde el noise: el volumen del chunk
                // y=0 no contiene las montañas de los niveles superiores, así
                // que extraerla de ahí aplanaría el terreno alto.
                let mut lod_chunk = LodChunk::new(chunk_pos, lod_level);
                let mut terrain_gen = TerrainGenerator::new(world_seed.0);
                lod_chunk.generate_surface(&mut terrain_gen);
                let mesh = mesh_lod_chunk(&lod_chunk);

                // Solo crear si el mesh tiene vértices
                if mesh.count_vertices() > 0 {
                    // Despawnear el BaseChunk
                    commands.entity(entity).despawn();

                    // Crear nuevo LOD chunk
                    let new_entity = commands
                        .spawn((
                            Mesh3d(meshes.add(mesh)),
                            MeshMaterial3d(chunk_materials.lod_handle(lod_level)),
                            Transform::default(),
                            lod_chunk,
                        ))
                        .id();

                    // Actualizar ChunkMap
                    chunk_map.chunks.insert(chunk_pos, new_entity);

                    info!("Converting Real → LOD at {:?}", chunk_pos);
                } else {
                    // Mesh vacío, solo despawnear
                    commands.entity(entity).despawn();
                    chunk_map.chunks.remove(&chunk_pos);
                }
            }
        }
    }
}

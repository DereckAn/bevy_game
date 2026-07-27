//! Sistema de carga dinámica de chunks
//! Genera y elimina chunks según la posición del jugador
//! Usa generación asíncrona para evitar lag con grandes distancias de renderizado
//! Incluye caché persistente en disco

use crate::{
    core::{BASE_CHUNK_SIZE, VOXEL_SIZE, WORLD_CHUNK_RADIUS, WorldSeed, WorldKind},
    physics::{Collider, RigidBody, create_terrain_collider},
    player::Player,
    voxel::{
        BaseChunk, ChunkLOD, ChunkMap, ChunkMaterial, LodChunk, LodLevel, PaletteExtension,
        SpatialHashGrid, TerrainGenerator, VoxelDiffs, mesh_lod_chunk,
    },
};
use bevy::{
    prelude::*, tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use std::collections::{HashMap, HashSet, VecDeque};

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

/// Radio (chunks) dentro del cual una columna tiene chunks Real (voxeles).
pub const REAL_RADIUS: i32 = 32;

/// Radio (chunks) más allá del cual los chunks Real se descargan. Es > `REAL_RADIUS`
/// (hysteresis) para que la frontera Real↔LOD no parpadee al caminar sobre ella.
pub const REAL_KEEP: i32 = 36;

/// Cuánto se BAJA el LOD respecto a la superficie real (metros). Durante el
/// solape de carga el chunk Real (a la altura real) gana el test de profundidad
/// y el LOD queda oculto detrás en vez de hacer z-fighting. A 100 m+ es invisible.
pub const LOD_DROP: f32 = 0.3;

/// Tipo de chunk a generar segun distancia
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    /// Chunk real con voxeles y colisión (columna cercana).
    Real,

    /// Backdrop LOD por columna (heightmap sin colisión, lejano).
    Lod,
}

impl ChunkType {
    // Determina el tipo de chunk segun la distancia al jugador
    pub fn from_distance(distance_chunks: i32) -> Self {
        if distance_chunks <= REAL_RADIUS {
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
    real: [Handle<ChunkMaterial>; 5],
    /// Por nivel de `LodLevel` (chunks LOD): Medium, Low, Minimal
    lod: [Handle<StandardMaterial>; 3],
}

impl FromWorld for ChunkMaterials {
    fn from_world(world: &mut World) -> Self {
        // Base blanca: el color real del suelo viene de los vertex colors; la
        // extensión de paleta aplica la variación de tono por voxel. 5 handles
        // iguales para que `real_handle` siga indexando por ChunkLOD aunque
        // `update_chunk_lod_system` los intercambie.
        let real = {
            let mut materials = world.resource_mut::<Assets<ChunkMaterial>>();
            [(); 5].map(|_| {
                materials.add(ChunkMaterial {
                    base: StandardMaterial {
                        base_color: Color::WHITE,
                        ..default()
                    },
                    extension: PaletteExtension::default(),
                })
            })
        };

        // Colores de debug por nivel LOD (naranja → rojo según distancia)
        let lod = {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            [
                Color::srgb(1.0, 0.6, 0.0), // Medium (32-64 chunks)
                Color::srgb(1.0, 0.3, 0.0), // Low (64-128)
                Color::srgb(0.8, 0.0, 0.0), // Minimal (128+)
            ]
            .map(|color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    ..default()
                })
            })
        };

        Self { real, lod }
    }
}

impl ChunkMaterials {
    pub fn real_handle(&self, lod: ChunkLOD) -> Handle<ChunkMaterial> {
        let idx = match lod {
            ChunkLOD::Ultra => 0,
            ChunkLOD::High => 1,
            ChunkLOD::Medium => 2,
            ChunkLOD::Low => 3,
            ChunkLOD::Minimal => 4,
        };
        self.real[idx].clone()
    }
}

/// Recurso que rastrea qué chunks necesitan ser cargados
#[derive(Resource, Default)]
pub struct ChunkLoadQueue {
    // Chunks a cargar con su tipo (Real o Lod)
    pub to_load: VecDeque<(IVec3, ChunkType)>,
    pub to_unload: Vec<(IVec3, Entity)>,

    pub last_player_chunk: IVec3,
    pub total_loaded: usize,
    pub last_log_time: f32,
}

/// LODs indexados por COLUMNA (x,z de chunk). Viven aquí, NO en `ChunkMap`: un
/// LOD representa la columna entera (heightmap de alturas absolutas), así que no
/// compite por el slot `(x,0,z)` con un chunk Real. Se retiran cuando el terreno
/// Real cubre la columna, y se recrean antes de descargar el Real al alejarse
/// → transiciones sin huecos en ninguna dirección.
#[derive(Resource, Default)]
pub struct ColumnLods {
    pub columns: HashMap<IVec2, Entity>,
}

/// Marcador para posiciones de chunk que son enteramente aire (por encima del
/// terreno). Ocupa lugar en `ChunkMap` para no re-evaluarlas, sin almacenar
/// voxels, mesh ni collider.
#[derive(Component)]
pub struct EmptyChunk;

/// Componente para chunks que están siendo generados asíncronamente
#[derive(Component)]
pub struct ChunkGenerationTask {
    /// La tarea async devuelve el chunk Y su collider ya construido (en el hilo de
    /// fondo): el mallado de colisión + el trimesh de Rapier salen del hilo
    /// principal. `None` si el chunk no tiene geometría colisionable.
    pub task: Task<(IVec3, BaseChunk, Option<Collider>)>,
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
    mut column_lods: ResMut<ColumnLods>,
    chunks: Query<
        Entity,
        Or<(
            With<BaseChunk>,
            With<LodChunk>,
            With<ChunkGenerationTask>,
            With<EmptyChunk>,
        )>,
    >,
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
    column_lods.columns.clear(); // las entidades LOD ya se despawnearon vía `chunks`
    *load_queue = ChunkLoadQueue::default();
}

/// Sistema que detecta cuando el jugador se mueve y actualiza la cola de carga
pub fn update_chunk_load_queue(
    player_query: Query<&Transform, With<Player>>,
    chunk_map: Res<ChunkMap>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    column_lods: Res<ColumnLods>,
    world_kind: Res<WorldKind>,
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

    // Rango vertical: -1 hasta +4 chunks (mejor rendimiento). El desierto tiene
    // dunas más altas (~19.5 m), así que sube el techo para no recortarlas plano.
    let y_min = -1;
    let y_max = match *world_kind {
        WorldKind::Desert => 6,
        WorldKind::Normal => 4,
        // Montañas heladas muy altas (~60 m): sube el techo para no recortar picos.
        WorldKind::Ice => 20,
    };

    // OPTIMIZACIÓN: Generar el círculo y encolar lo que falta en UNA sola pasada.
    // El triple bucle visita cada (cx,cy,cz) exactamente una vez, así que no hay
    // duplicados que deduplicar: el viejo HashSet de ~64k entradas era puro coste.
    // Comprobamos chunk_map en el momento y empujamos solo lo que falta cargar.
    let radius_sq = CHUNK_LOAD_RADIUS * CHUNK_LOAD_RADIUS;
    let mut to_load_vec: Vec<(IVec3, ChunkType)> = Vec::new();

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

                // Distancia horizontal al jugador = (cx, cz) directamente
                let distance_chunks = ((x_sq + cz * cz) as f32).sqrt() as i32;

                match ChunkType::from_distance(distance_chunks) {
                    ChunkType::Real => {
                        // Un chunk Real por nivel Y; encola solo lo que falta.
                        if !chunk_map.chunks.contains_key(&chunk_pos) {
                            to_load_vec.push((chunk_pos, ChunkType::Real));
                        }
                    }
                    ChunkType::Lod => {
                        // Un LOD por COLUMNA (heightmap de alturas absolutas): se
                        // encola en cy=0 y se deduplica contra `ColumnLods`, no
                        // contra `chunk_map` (los LOD ya no viven ahí).
                        if cy != 0 {
                            continue;
                        }
                        let column = IVec2::new(chunk_pos.x, chunk_pos.z);
                        if !column_lods.columns.contains_key(&column) {
                            to_load_vec.push((chunk_pos, ChunkType::Lod));
                        }
                    }
                }
            }
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
    // La DESCARGA de chunks Real la hace `evict_real_to_lod_system` (asegura el
    // backdrop LOD antes de sacar el Real → sin huecos). Los LOD fuera de rango
    // los retira `retire_covered_lods_system`.
}

/// Sistema que inicia la generación asíncrona de chunks con caché
pub fn load_chunks_system(
    mut commands: Commands,
    mut chunk_map: ResMut<ChunkMap>,
    mut spatial_hash: ResMut<SpatialHashGrid>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut column_lods: ResMut<ColumnLods>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_materials: Res<ChunkMaterials>,
    world_seed: Res<WorldSeed>,
    world_kind: Res<WorldKind>,
    voxel_diffs: Res<VoxelDiffs>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let seed = world_seed.0;
    let kind = *world_kind;

    // Generador reutilizado para sondear la altura del terreno (saltar chunks de aire)
    let mut terrain_gen = TerrainGenerator::new(seed, kind);

    // Iniciar generación de hasta MAX_CHUNKS_PER_FRAME chunks por frame
    let chunks_to_load = load_queue.to_load.len().min(MAX_CHUNKS_PER_FRAME);

    for _ in 0..chunks_to_load {
        if let Some((chunk_pos, chunk_type)) = load_queue.to_load.pop_front() {
            match chunk_type {
                ChunkType::Real => {
                    // Verificar que no se haya cargado mientras tanto
                    if chunk_map.chunks.contains_key(&chunk_pos) {
                        continue;
                    }

                    // Crear entidad placeholder y marcarla como "en generación"
                    let chunk_entity = commands.spawn_empty().id();
                    chunk_map.chunks.insert(chunk_pos, chunk_entity);
                    spatial_hash.insert(chunk_pos);

                    // Saltar chunks enteramente por encima del terreno: son puro
                    // aire, sin geometría ni colisión. Se marcan con EmptyChunk
                    // (siguen en ChunkMap, así no se vuelven a evaluar). NO se
                    // saltan si el jugador los modificó (tienen diffs).
                    if !voxel_diffs.chunks.contains_key(&chunk_pos)
                        && chunk_is_above_terrain(chunk_pos, &mut terrain_gen, seed)
                    {
                        commands.entity(chunk_entity).insert(EmptyChunk);
                        continue;
                    }

                    // Saltar chunks de piedra PROFUNDA (muy por debajo de la
                    // superficie local): bajo los picos altos serían decenas de
                    // chunks sólidos mallados que el jugador casi nunca alcanza.
                    // Se deja una banda excavable bajo la superficie (ver la
                    // constante en la función). No se saltan los modificados.
                    if !voxel_diffs.chunks.contains_key(&chunk_pos)
                        && chunk_is_below_terrain_floor(chunk_pos, &mut terrain_gen)
                    {
                        commands.entity(chunk_entity).insert(EmptyChunk);
                        continue;
                    }

                    // Copia los diffs de ESTE chunk antes de lanzar la tarea
                    let chunk_diffs = voxel_diffs.chunks.get(&chunk_pos).cloned();

                    let task = thread_pool.spawn(async move {
                        // El mallado de RENDER (con vecinos) se hace en
                        // complete_chunk_generation_system. Aquí, en el hilo de fondo,
                        // construimos el COLLIDER con un mesh simple solo-colisionable
                        // (sin vecinos) → saca el trabajo caro del hilo principal.
                        let mut base_chunk = BaseChunk::new(chunk_pos, seed, kind);

                        if let Some(diffs) = &chunk_diffs {
                            base_chunk.apply_diffs(diffs);
                        }
                        let collider = build_chunk_collider(&base_chunk);
                        (chunk_pos, base_chunk, collider)
                    });

                    commands
                        .entity(chunk_entity)
                        .insert(ChunkGenerationTask { task, chunk_pos });
                }

                ChunkType::Lod => {
                    // Un LOD por columna: heightmap de superficie (síncrono, barato).
                    // Vive en `ColumnLods`, NO en `chunk_map`/`spatial_hash`.
                    let column = IVec2::new(chunk_pos.x, chunk_pos.z);
                    if column_lods.columns.contains_key(&column) {
                        continue;
                    }

                    let delta = chunk_pos - load_queue.last_player_chunk;
                    let distance_chunks = ((delta.x.pow(2) + delta.z.pow(2)) as f32).sqrt() as i32;

                    if let Some(entity) = spawn_column_lod(
                        &mut commands,
                        &mut meshes,
                        &chunk_materials,
                        chunk_pos,
                        distance_chunks,
                        seed,
                        kind,
                    ) {
                        column_lods.columns.insert(column, entity);
                        load_queue.total_loaded += 1;
                    }
                }
            }
        }
    }
}

/// Construye y spawnea el LOD de una columna (heightmap de superficie, sin
/// colisión), BAJADO `LOD_DROP` metros para que un chunk Real encima gane el
/// z-test. Devuelve la entidad, o `None` si el mesh sale vacío. Compartido por la
/// carga normal y por la evicción Real→LOD (ambas necesitan crear el mismo LOD).
fn spawn_column_lod(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    chunk_materials: &ChunkMaterials,
    chunk_pos: IVec3,
    distance_chunks: i32,
    seed: i32,
    kind: WorldKind,
) -> Option<Entity> {
    let lod_level = LodLevel::from_distance(distance_chunks);
    let mut lod_chunk = LodChunk::new(chunk_pos, lod_level);
    let mut terrain_gen = TerrainGenerator::new(seed, kind);
    lod_chunk.generate_surface(&mut terrain_gen);

    let mesh = mesh_lod_chunk(&lod_chunk, seed, kind);
    if mesh.count_vertices() == 0 {
        return None;
    }

    Some(
        commands
            .spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(chunk_materials.real_handle(ChunkLOD::Ultra)),
                Transform::from_xyz(0.0, -LOD_DROP, 0.0),
                lod_chunk,
                ChunkLOD::from_distance(distance_chunks as f32),
            ))
            .id(),
    )
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

        if let Some((_chunk_pos, base_chunk, collider)) =
            future::block_on(future::poll_once(&mut task.task))
        {
            // Solo el mesh de RENDER (con vecinos) se hace aquí; el collider ya
            // vino construido desde el hilo de fondo.
            let mesh = greedy_mesh_basechunk(&base_chunk, &chunk_map, &base_chunks);

            let mut ec = commands.entity(entity);
            ec.insert((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(chunk_materials.real_handle(ChunkLOD::Ultra)),
                Transform::default(),
                base_chunk,
                ChunkLOD::Ultra,
            ));
            if let Some(collider) = collider {
                ec.insert((RigidBody::Fixed, collider));
            }
            ec.remove::<ChunkGenerationTask>();

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

/// Construye el collider de un chunk a partir de un mesh simple SOLO-COLISIONABLE
/// (sin vecinos, ignora el follaje). Pensado para correr en el hilo de fondo.
/// `None` si el chunk no tiene geometría colisionable.
fn build_chunk_collider(chunk: &BaseChunk) -> Option<Collider> {
    let mesh = crate::voxel::greedy_mesh_basechunk_collider_simple(chunk);
    (mesh.count_vertices() > 0).then(|| create_terrain_collider(&mesh))
}

/// ¿El chunk está enteramente por ENCIMA del terreno (puro aire)?
///
/// Como la densidad es `altura(x,z) - y` (monótona en Y), un chunk es todo aire
/// si la altura MÁXIMA del terreno sobre su huella XZ queda por debajo del fondo
/// del chunk. Muestrea una rejilla 5×5 (la frecuencia del ruido hace innecesario
/// muestrear más fino sobre 3.2 m) y añade un margen para no saltar un chunk que
/// apenas roce el terreno.
fn chunk_is_above_terrain(chunk_pos: IVec3, terrain_gen: &mut TerrainGenerator, seed: i32) -> bool {
    // Y mundial del fondo del chunk (metros)
    let chunk_bottom_y = chunk_pos.y as f32 * BASE_CHUNK_SIZE as f32 * VOXEL_SIZE;

    // Margen de seguridad (~5 voxels) contra picos entre muestras
    let margin = 0.5;

    let step = BASE_CHUNK_SIZE / 4; // 0, 8, 16, 24, 32 → 5 muestras por eje
    let mut max_height = f32::MIN;
    let mut sx = 0;
    while sx <= BASE_CHUNK_SIZE {
        let mut sz = 0;
        while sz <= BASE_CHUNK_SIZE {
            let world_x = (chunk_pos.x * BASE_CHUNK_SIZE as i32 + sx as i32) as f32 * VOXEL_SIZE;
            let world_z = (chunk_pos.z * BASE_CHUNK_SIZE as i32 + sz as i32) as f32 * VOXEL_SIZE;
            let h = terrain_gen.biome_gen.generate_height(world_x, world_z);
            if h > max_height {
                max_height = h;
            }
            sz += step;
        }
        sx += step;
    }

    // Un árbol puede subir desde un chunk inferior hasta este: si alguno llega
    // aquí, NO lo tratamos como aire (si no, se cortaría el árbol).
    let chunk_bottom_voxel = chunk_pos.y * BASE_CHUNK_SIZE as i32;
    if let Some(ceiling) = crate::vegetation::trees::tree_ceiling_for_chunk(
        chunk_pos,
        &mut terrain_gen.biome_gen,
        seed,
    ) {
        if ceiling >= chunk_bottom_voxel {
            return false;
        }
    }

    max_height + margin < chunk_bottom_y
}

/// Profundidad de terreno excavable que se conserva bajo la superficie, en
/// metros. Los chunks enteramente más profundos que esto se saltan (no se
/// mallan). Subir = más profundidad para excavar pero más chunks generados.
const DIGGABLE_DEPTH_M: f32 = 9.6; // ~3 chunks (BASE_CHUNK_SIZE * VOXEL_SIZE = 3.2 m)

/// ¿Está el chunk enteramente MÁS de [`DIGGABLE_DEPTH_M`] bajo la superficie?
///
/// Bajo las montañas altas, el rango vertical fijo generaría muchos chunks de
/// piedra maciza apilados hasta la cima. Esta comprobación (espejo de
/// `chunk_is_above_terrain`) recorta esa piedra profunda: solo se malla una
/// banda alrededor de la superficie, así el conteo de chunks es ~constante sin
/// importar lo alto que sea el bioma.
///
/// Usa la superficie MÍNIMA sobre la huella del chunk (conservador: si alguna
/// esquina es baja, no se salta el chunk).
fn chunk_is_below_terrain_floor(chunk_pos: IVec3, terrain_gen: &mut TerrainGenerator) -> bool {
    // Y mundial de la CIMA del chunk (metros).
    let chunk_top_y = (chunk_pos.y + 1) as f32 * BASE_CHUNK_SIZE as f32 * VOXEL_SIZE;

    let step = BASE_CHUNK_SIZE / 4; // 5 muestras por eje
    let mut min_height = f32::MAX;
    let mut sx = 0;
    while sx <= BASE_CHUNK_SIZE {
        let mut sz = 0;
        while sz <= BASE_CHUNK_SIZE {
            let world_x = (chunk_pos.x * BASE_CHUNK_SIZE as i32 + sx as i32) as f32 * VOXEL_SIZE;
            let world_z = (chunk_pos.z * BASE_CHUNK_SIZE as i32 + sz as i32) as f32 * VOXEL_SIZE;
            let h = terrain_gen.biome_gen.generate_height(world_x, world_z);
            if h < min_height {
                min_height = h;
            }
            sz += step;
        }
        sx += step;
    }

    chunk_top_y < min_height - DIGGABLE_DEPTH_M
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

/// Mitad "LOD → Real" de la transición, sin huecos: RETIRA el backdrop LOD de una
/// columna solo cuando el terreno Real ya la cubre (su chunk de superficie existe
/// y está mallado). Hasta entonces el LOD sigue visible mientras el Real se genera
/// async, así nunca aparece un hueco. También descarga los LOD fuera de rango.
///
/// Corre cada frame (la cobertura Real puede completarse con el jugador quieto).
pub fn retire_covered_lods_system(
    mut commands: Commands,
    mut column_lods: ResMut<ColumnLods>,
    chunk_map: Res<ChunkMap>,
    base_chunks: Query<&BaseChunk>,
    player_query: Query<&Transform, With<Player>>,
    world_seed: Res<WorldSeed>,
    world_kind: Res<WorldKind>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_chunk = world_pos_to_chunk_pos(player_transform.translation);
    let mut terrain_gen = TerrainGenerator::new(world_seed.0, *world_kind);

    let chunk_m = BASE_CHUNK_SIZE as f32 * VOXEL_SIZE;
    let half = BASE_CHUNK_SIZE as i32 / 2;
    let unload_sq = CHUNK_UNLOAD_RADIUS * CHUNK_UNLOAD_RADIUS;
    let real_sq = REAL_RADIUS * REAL_RADIUS;

    let mut retired: Vec<IVec2> = Vec::new();
    for (&column, &lod_entity) in &column_lods.columns {
        let dx = column.x - player_chunk.x;
        let dz = column.y - player_chunk.z;
        let dist_sq = dx * dx + dz * dz;

        // Fuera de rango → descargar el LOD.
        if dist_sq > unload_sq {
            commands.entity(lod_entity).despawn();
            retired.push(column);
            continue;
        }

        // Solo columnas cercanas pueden estar cubiertas por chunks Real.
        if dist_sq > real_sq {
            continue;
        }

        // Chunk Real de la SUPERFICIE de la columna (centro de su huella).
        let world_x = (column.x * BASE_CHUNK_SIZE as i32 + half) as f32 * VOXEL_SIZE;
        let world_z = (column.y * BASE_CHUNK_SIZE as i32 + half) as f32 * VOXEL_SIZE;
        let surface_m = terrain_gen.biome_gen.generate_height(world_x, world_z);
        let surface_y = (surface_m / chunk_m).floor() as i32;
        let surface_pos = IVec3::new(column.x, surface_y, column.y);

        let covered = chunk_map
            .chunks
            .get(&surface_pos)
            .is_some_and(|&e| base_chunks.get(e).is_ok());
        if covered {
            commands.entity(lod_entity).despawn();
            retired.push(column);
        }
    }
    for column in retired {
        column_lods.columns.remove(&column);
    }
}

/// Mitad "Real → LOD" de la transición, sin huecos: cuando una columna sale del
/// rango Real, primero ASEGURA su backdrop LOD (lo crea si falta) y solo después
/// saca del mapa sus chunks Real y los encola para despawnear (con presupuesto en
/// `unload_chunks_system`). Como el LOD ya cubre la columna, no hay hueco.
///
/// Solo actúa al cambiar de chunk (los Real solo salen de rango al moverse).
pub fn evict_real_to_lod_system(
    mut commands: Commands,
    mut column_lods: ResMut<ColumnLods>,
    mut chunk_map: ResMut<ChunkMap>,
    mut spatial_hash: ResMut<SpatialHashGrid>,
    mut load_queue: ResMut<ChunkLoadQueue>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_materials: Res<ChunkMaterials>,
    player_query: Query<&Transform, With<Player>>,
    world_seed: Res<WorldSeed>,
    world_kind: Res<WorldKind>,
    mut last_chunk: Local<IVec3>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let player_chunk = world_pos_to_chunk_pos(player_transform.translation);
    if player_chunk == *last_chunk {
        return;
    }
    *last_chunk = player_chunk;

    let seed = world_seed.0;
    let kind = *world_kind;
    let keep_sq = REAL_KEEP * REAL_KEEP;

    // Chunks (Real / vacíos / generándose) cuya columna salió del rango Real.
    let mut to_remove: Vec<(IVec3, Entity)> = Vec::new();
    let mut columns_beyond: HashSet<IVec2> = HashSet::new();
    for (&pos, &entity) in &chunk_map.chunks {
        let dx = pos.x - player_chunk.x;
        let dz = pos.z - player_chunk.z;
        if dx * dx + dz * dz > keep_sq {
            to_remove.push((pos, entity));
            columns_beyond.insert(IVec2::new(pos.x, pos.z));
        }
    }
    if to_remove.is_empty() {
        return;
    }

    // 1) Asegurar el LOD de cada columna ANTES de descargar su Real.
    for &column in &columns_beyond {
        if column_lods.columns.contains_key(&column) {
            continue;
        }
        let dx = column.x - player_chunk.x;
        let dz = column.y - player_chunk.z;
        let distance_chunks = ((dx * dx + dz * dz) as f32).sqrt() as i32;
        if distance_chunks > CHUNK_LOAD_RADIUS {
            continue; // fuera del rango LOD: no hace falta backdrop
        }
        let chunk_pos = IVec3::new(column.x, 0, column.y);
        if let Some(entity) = spawn_column_lod(
            &mut commands,
            &mut meshes,
            &chunk_materials,
            chunk_pos,
            distance_chunks,
            seed,
            kind,
        ) {
            column_lods.columns.insert(column, entity);
        }
    }

    // 2) Sacar del mapa YA (para no re-encolar) y despawnear con presupuesto.
    for (pos, entity) in to_remove {
        chunk_map.chunks.remove(&pos);
        spatial_hash.remove(pos);
        load_queue.to_unload.push((pos, entity));
    }
}

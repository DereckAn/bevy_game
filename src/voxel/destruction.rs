//! Sistema de destruccion de voxels
//!
//! Premite al jugador romper voxels usando herramientas.

use super::{
    Chunk, VoxelType,
    tools::{Tool, ToolType},
};
use crate::player::components::Player;
use crate::{
    core::constants::{CHUNK_SIZE, VOXEL_SIZE},
    voxel::generate_mesh_with_neighbors,
};
use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use std::collections::HashMap;
use super::VoxelDrop;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Component que rastrea el progreso de destruccion de un voxel.
#[derive(Component, Debug)]
pub struct VoxelBreaking {
    // Posicion del chunk que contiene el voxel.
    pub chunk_pos: IVec2,

    // Posicion local del voxel dnetro del chunk (0-31).]
    pub local_pos: IVec3,

    // Preogreso de destruccion (0.0 = intacto - 1.0 = roto).
    pub progress: f32,

    // Tiempo total necesario para romper este voxel.
    pub break_time: f32,
}

#[derive(Resource)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec2, Entity>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Calcula el timepo necesario para romper un voxel.
///
/// # Parametros
/// voxel_type: Tipo de voxel a romper
/// tool_type: Heramienta siendo usada
///
/// # Retorna
/// Tiempo en segundos para romper el voxel.
pub fn calculate_break_time(voxel_type: VoxelType, tool_type: ToolType) -> f32 {
    // Ontener dureza del voxel
    let hardness = voxel_type.properties().hardness;

    // Obtener efectividad de la herramienta
    let effectiveness = tool_type.effectiveness_against(voxel_type);

    // Obtener multiplicador de velocidad de la herramienta
    let speed = tool_type.properties().speed_multiplier;

    // Formula: tiempo_base * hardness / (effectiveness * speed)
    // Tiempo base 1 segundo
    let base_time = 1.0;

    if effectiveness == 0.0 || speed == 0.0 {
        return 999.0; // Practicamente impoisble de romper
    }

    base_time * hardness / (effectiveness * speed)
}

/// Convierte una posicion mundial a la posicion de chunk y posicion local.
///
/// # Retorna
/// (chunk_pos, local_pos, vloxel_pos_in_chunk)
pub fn world_to_voxel(world_pos: Vec3) -> (IVec2, IVec3, IVec3) {
    // Convertir a coordenadas de voxel
    let voxel_x = (world_pos.x / VOXEL_SIZE).floor() as i32;
    let voxel_y = (world_pos.y / VOXEL_SIZE).floor() as i32;
    let voxel_z = (world_pos.z / VOXEL_SIZE).floor() as i32;

    // Calcular la posicion del chunk
    let chunk_x = voxel_x.div_euclid(CHUNK_SIZE as i32);
    let _chunk_y = voxel_y.div_euclid(CHUNK_SIZE as i32); // Unused in columnar chunks
    let chunk_z = voxel_z.div_euclid(CHUNK_SIZE as i32);

    // Calcular la posicion local del voxel dentro del chunk
    let local_x = voxel_x.rem_euclid(CHUNK_SIZE as i32);
    let local_y = voxel_y.rem_euclid(CHUNK_SIZE as i32);
    let local_z = voxel_z.rem_euclid(CHUNK_SIZE as i32);

    (
        IVec2::new(chunk_x, chunk_z),
        IVec3::new(local_x, local_y, local_z),
        IVec3::new(voxel_x, voxel_y, voxel_z),
    )
}

/// Realiza un raycast usando algoritmo DDA para detectar el voxel mas cercano.
///
/// DDA (Digital Differential Analyzer) es mucho mas eficiente que point-by-point
/// porque camina exactamente por los voxels que toca el rayo.
///
/// # Parametros
/// - origin: Punto de inicio del rayo (posicion de la camara)
/// - direction: Direccion del rayo (direccion de la camara)  
/// - max_distance: Distancia maxima del raycast (en metros)
/// - chunk_map: Mapa de chunks disponibles
/// - chunks: Query de todos los chunks en el mundo
///
/// # Retorna
/// Some((chunk_entity, chunk_pos, local_pos, voxel_type)) si encuentra un voxel solido
/// None si no encuentra nada
pub fn raycast_voxel(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_map: &ChunkMap,
    chunks: &Query<&Chunk>,
) -> Option<(Entity, IVec2, IVec3, VoxelType)> {
    let dir = direction.normalize();

    // Convertir origen a coordenadas de voxel
    let mut voxel_pos = IVec3::new(
        (origin.x / VOXEL_SIZE).floor() as i32,
        (origin.y / VOXEL_SIZE).floor() as i32,
        (origin.z / VOXEL_SIZE).floor() as i32,
    );

    // Calcular direccion del paso (1 o -1 para cada eje)
    let step = IVec3::new(
        if dir.x > 0.0 { 1 } else { -1 },
        if dir.y > 0.0 { 1 } else { -1 },
        if dir.z > 0.0 { 1 } else { -1 },
    );

    // Calcular distancia hasta el siguiente voxel en cada eje
    let mut t_max = Vec3::new(
        if dir.x != 0.0 {
            let next_boundary = if dir.x > 0.0 {
                (voxel_pos.x + 1) as f32 * VOXEL_SIZE
            } else {
                voxel_pos.x as f32 * VOXEL_SIZE
            };
            (next_boundary - origin.x) / dir.x
        } else {
            f32::INFINITY
        },
        if dir.y != 0.0 {
            let next_boundary = if dir.y > 0.0 {
                (voxel_pos.y + 1) as f32 * VOXEL_SIZE
            } else {
                voxel_pos.y as f32 * VOXEL_SIZE
            };
            (next_boundary - origin.y) / dir.y
        } else {
            f32::INFINITY
        },
        if dir.z != 0.0 {
            let next_boundary = if dir.z > 0.0 {
                (voxel_pos.z + 1) as f32 * VOXEL_SIZE
            } else {
                voxel_pos.z as f32 * VOXEL_SIZE
            };
            (next_boundary - origin.z) / dir.z
        } else {
            f32::INFINITY
        },
    );

    // Calcular incremento de distancia para cada eje
    let t_delta = Vec3::new(
        if dir.x != 0.0 {
            VOXEL_SIZE / dir.x.abs()
        } else {
            f32::INFINITY
        },
        if dir.y != 0.0 {
            VOXEL_SIZE / dir.y.abs()
        } else {
            f32::INFINITY
        },
        if dir.z != 0.0 {
            VOXEL_SIZE / dir.z.abs()
        } else {
            f32::INFINITY
        },
    );

    let max_steps = (max_distance / VOXEL_SIZE) as i32 + 1;

    // Algoritmo DDA principal
    for _ in 0..max_steps {
        // Convertir posicion de voxel a chunk y posicion local
        let (chunk_pos, local_pos, _) = world_to_voxel(Vec3::new(
            voxel_pos.x as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
            voxel_pos.y as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
            voxel_pos.z as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
        ));

        // Verificar si tenemos este chunk
        if let Some(&chunk_entity) = chunk_map.chunks.get(&chunk_pos) {
            if let Ok(chunk) = chunks.get(chunk_entity) {
                // Verificar limites del chunk
                if local_pos.x >= 0
                    && local_pos.x < CHUNK_SIZE as i32
                    && local_pos.y >= 0
                    && local_pos.y < CHUNK_SIZE as i32
                    && local_pos.z >= 0
                    && local_pos.z < CHUNK_SIZE as i32
                {
                    let voxel_type = chunk.voxel_types[local_pos.x as usize][local_pos.y as usize]
                        [local_pos.z as usize];

                    if voxel_type.is_solid() {
                        return Some((chunk_entity, chunk_pos, local_pos, voxel_type));
                    }
                }
            }
        }

        // Avanzar al siguiente voxel usando DDA
        if t_max.x < t_max.y && t_max.x < t_max.z {
            // Avanzar en X
            voxel_pos.x += step.x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            // Avanzar en Y
            voxel_pos.y += step.y;
            t_max.y += t_delta.y;
        } else {
            // Avanzar en Z
            voxel_pos.z += step.z;
            t_max.z += t_delta.z;
        }

        // Verificar si hemos excedido la distancia maxima
        let current_distance = (Vec3::new(
            voxel_pos.x as f32 * VOXEL_SIZE,
            voxel_pos.y as f32 * VOXEL_SIZE,
            voxel_pos.z as f32 * VOXEL_SIZE,
        ) - origin)
            .length();

        if current_distance > max_distance {
            break;
        }
    }

    None
}

// ============================================================================
// BEVY SYSTEMS
// ============================================================================

/// Sistema que detecta cuando el jugador intenta romper un voxel.
///
/// Solo se ejecuta cuando el jugador presiona el boton de romper.
pub fn start_voxel_breaking_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<Camera>>,
    chunk_map: Res<ChunkMap>,
    chunks: Query<&Chunk>,
    player_query: Query<&Tool, With<Player>>,
    mut commands: Commands,
    mut breaking_query: Query<(Entity, &mut VoxelBreaking)>,
) {
    // Solo ejecuta si preional el boton izquierdo
    if !mouse_input.pressed(MouseButton::Left) {
        // Si suelta el boton, cancelar destruccion en progreso
        for (entity, _) in breaking_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Obtener la camara (posicion y direccion)
    let Ok(camera_transform) = camera_query.single() else {
        return; // No hay camara.
    };

    let ray_origin = camera_transform.translation;
    let ray_direction = camera_transform.forward().as_vec3();

    // Hacer raycast para encontrar voxel
    let Some((_chunk_entity, chunk_pos, local_pos, voxel_type)) = raycast_voxel(
        ray_origin,
        ray_direction,
        5.0, // Maximo 5 metros de distancia
        &chunk_map,
        &chunks,
    ) else {
        // No encontro nada, cnacelar destruccion
        for (entity, _) in breaking_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    // Obtener herramienta del jugador
    let tool_type = player_query
        .single()
        .map(|tool| tool.tool_type)
        .unwrap_or(ToolType::None);

    // Calcular tiempo de destruccion
    let break_time = calculate_break_time(voxel_type, tool_type);

    // Verificar si ya estamos rompiendo este voxel
    let mut found_existing = false;
    for (entity, breaking) in breaking_query.iter_mut() {
        if breaking.chunk_pos == chunk_pos && breaking.local_pos == local_pos {
            // Ya estamos rompiendo este voxel, no hacer nada
            found_existing = true;
            break;
        } else {
            // Estamos mirandop otro voxel, cancelar el anterior
            commands.entity(entity).despawn();
        }
    }

    // Si no existe crear nuevo componente de destruccion
    if !found_existing {
        commands.spawn(VoxelBreaking {
            chunk_pos,
            local_pos,
            progress: 0.0,
            break_time,
        });
    }
}

/// Sistema que actualiza el progreso de destruccion de voxels.
///
/// Se ejecuta cada frame para actualizar el progreso.
pub fn update_voxel_breaking_system(
    time: Res<Time>,
    mut breaking_query: Query<(Entity, &mut VoxelBreaking)>,
    mut chunk_queries: ParamSet<(Query<&mut Chunk>, Query<&Chunk>)>,
    chunk_map: Res<ChunkMap>,
    mut commands: Commands,
    mut player_query: Query<&mut Tool, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_query: Query<&mut Mesh3d>,
) {
    for (entity, mut breaking) in breaking_query.iter_mut() {
        // Actualizar preogreso basado en tiempo
        breaking.progress += time.delta_secs() / breaking.break_time;

        // Si llego a 100%, romper el voxel
        if breaking.progress >= 1.0 {
            // Obtener el chunk
            if let Some(&chunk_entity) = chunk_map.chunks.get(&breaking.chunk_pos) {
                // Primero modificar el chunk
                let broken_voxel_type = if let Ok(mut chunk) =
                    chunk_queries.p0().get_mut(chunk_entity)
                {
                    // Obtener herramienta para el patron de destruccion
                    let tool_type = player_query
                        .single()
                        .map(|tool| tool.tool_type)
                        .unwrap_or(ToolType::None);
                    let destruction_pattern = tool_type.get_destruction_pattern();

                    // Destruir multiples voxels segun el patron
                    let mut total_drops = 0;
                    for offset in destruction_pattern {
                        let target_x = (breaking.local_pos.x + offset.x) as usize;
                        let target_y = (breaking.local_pos.y + offset.y) as usize;
                        let target_z = (breaking.local_pos.z + offset.z) as usize;

                        // Verificar limites del chunk
                        if target_x < CHUNK_SIZE && target_y < CHUNK_SIZE && target_z < CHUNK_SIZE {
                            let voxel_type = chunk.voxel_types[target_x][target_y][target_z];

                            // Solo destruir si es sólido
                            if voxel_type.is_solid() {
                                // Convertir a aire
                                chunk.voxel_types[target_x][target_y][target_z] = VoxelType::Air;
                                chunk.densities[target_x][target_y][target_z] = -1.0;

                                // Calcular drops para este voxel
                                let drops = tool_type.calculate_drops(voxel_type);
                                total_drops += drops;

                                // Spawnar drops fisicos
                                if drops > 0 {
                                    spawn_voxel_drop(
                                        &mut commands,
                                        &mut meshes,
                                        &mut materials,
                                        voxel_type,
                                        drops, 
                                        Vec3::new(
                                            (breaking.chunk_pos.x * CHUNK_SIZE as i32 + target_x as i32) as f32 * VOXEL_SIZE,
                                            target_y as f32 * VOXEL_SIZE, // Y is absolute within the world column
                                            (breaking.chunk_pos.y * CHUNK_SIZE as i32 + target_z as i32) as f32 * VOXEL_SIZE,
                                        ),
                                        time.elapsed_secs(),
                                    );
                                }
                            }
                        }
                    }
                    info!("Destruido cráter con {} drops totales", total_drops);
                    Some(VoxelType::Air) // Retorna algo para que compile
                } else {
                    None
                };

                // Luego regenerar el mesh (después de liberar el borrow mutable)
                if let Some(_) = broken_voxel_type {
                    // Usar el query inmutable para generar el mesh
                    let chunks_read = chunk_queries.p1();
                    if let Ok(chunk) = chunks_read.get(chunk_entity) {
                        // Generar nuevo mesh con neighbors
                        let new_mesh =
                            generate_mesh_with_neighbors(&chunk, &chunk_map, &chunks_read);

                        if let Ok(mut mesh3d) = mesh_query.get_mut(chunk_entity) {
                            *mesh3d = Mesh3d(meshes.add(new_mesh));
                        }
                    }

                    // Danar herramienta del jugador
                    if let Ok(mut tool) = player_query.single_mut() {
                        let broke = tool.damage(1); // 1 punto de durabilidad
                        if broke {
                            info!("Herramienta rota");
                            // TODO: Cambiar a manos (ToolType::None) Tambien hacer que desaparesca la heramienta
                        }
                    }

                    if let Some(voxel_type) = broken_voxel_type {
                        info!("voxel roto {:?} en {:?}", voxel_type, breaking.local_pos);
                    }
                }
            }

            // Eliminar el componente de destruccion
            commands.entity(entity).despawn();
        }
    }
}

/// Spawna un drop fisico en el mundo
fn spawn_voxel_drop (
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    voxel_type: VoxelType,
    quantity: u32,
    world_position: Vec3,
    current_time: f32,
) {
    // Crear mesh de cubo pequeno
    let cube_mesh = meshes.add(Cuboid::new(0.2, 0.2, 0.2));

    // Color basado en el tipo de voxel
    let color = voxel_type.properties().color;
    let material = materials.add(StandardMaterial {
        base_color: color,
        metallic: 0.1,
        perceptual_roughness: 0.0,
        ..default()
    });

    commands.spawn((
        VoxelDrop::new(voxel_type, quantity, current_time),
        Mesh3d(cube_mesh),
        MeshMaterial3d(material),
        Transform::from_translation(world_position + Vec3::new(0.0, 1.0, 0.0))
        .with_scale(Vec3::splat(0.8)),
        GlobalTransform::default(),
        Visibility::default()
    ));
}
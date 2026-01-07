//! Sistema de destrucción de voxels con chunks dinámicos 3D
//!
//! Permite al jugador romper voxels usando herramientas, con detección
//! de suelo mejorada inspirada en "Lay of the Land" usando PhysX-style collision.

use super::{
    BaseChunk, VoxelType,
    tools::{Tool, ToolType},
    greedy_meshing::greedy_mesh_basechunk,
};
use crate::{physics::spawn_rapier_voxel_drop, player::components::Player};
use crate::{
    core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE},
    core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE},
};
use bevy::prelude::*;
use std::collections::HashMap;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Component que rastrea el progreso de destrucción de un voxel.
/// 
/// Actualizado para usar chunks 3D (IVec3) en lugar de columnares (IVec2).
#[derive(Component, Debug)]
pub struct VoxelBreaking {
    // Posición del chunk 3D que contiene el voxel (X, Y, Z)
    pub chunk_pos: IVec3,

    // Posición local del voxel dentro del chunk (0-31 en cada eje)
    pub local_pos: IVec3,

    // Progreso de destrucción (0.0 = intacto - 1.0 = roto)
    pub progress: f32,

    // Tiempo total necesario para romper este voxel
    pub break_time: f32,
}

/// Mapa de chunks 3D para el sistema dinámico
#[derive(Resource)]
pub struct ChunkMap3D {
    pub chunks: HashMap<IVec3, Entity>,
}

/// Sistema de detección de suelo mejorado (inspirado en "Lay of the Land")
/// 
/// Usa raycast hacia abajo para encontrar la superficie real del terreno,
/// evitando que los drops traspasen el piso o queden flotando.
#[derive(Component)]
pub struct GroundDetection {
    pub ground_height: f32,
    pub is_valid: bool,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Calcula el tiempo necesario para romper un voxel.
pub fn calculate_break_time(voxel_type: VoxelType, tool_type: ToolType) -> f32 {
    let hardness = voxel_type.properties().hardness;
    let effectiveness = tool_type.effectiveness_against(voxel_type);
    let speed = tool_type.properties().speed_multiplier;

    let base_time = 1.0;

    if effectiveness == 0.0 || speed == 0.0 {
        return 999.0;
    }

    base_time * hardness / (effectiveness * speed)
}

/// Convierte una posicion mundial a la posicion de chunk y posicion local.
///
/// # Retorna
/// (chunk_pos, local_pos, voxel_pos_in_chunk)
pub fn world_to_voxel(world_pos: Vec3) -> (IVec3, IVec3, IVec3) {
    // Convertir a coordenadas de voxel
    let voxel_x = (world_pos.x / VOXEL_SIZE).floor() as i32;
    let voxel_y = (world_pos.y / VOXEL_SIZE).floor() as i32;
    let voxel_z = (world_pos.z / VOXEL_SIZE).floor() as i32;

    // Calcular la posicion del chunk
    let chunk_x = voxel_x.div_euclid(BASE_CHUNK_SIZE as i32);
    let chunk_y = voxel_y.div_euclid(BASE_CHUNK_SIZE as i32);
    let chunk_z = voxel_z.div_euclid(BASE_CHUNK_SIZE as i32);

    // Calcular la posicion local del voxel dentro del chunk
    let local_x = voxel_x.rem_euclid(BASE_CHUNK_SIZE as i32);
    let local_y = voxel_y.rem_euclid(BASE_CHUNK_SIZE as i32);
    let local_z = voxel_z.rem_euclid(BASE_CHUNK_SIZE as i32);

    (
        IVec3::new(chunk_x, chunk_y, chunk_z),
        IVec3::new(local_x, local_y, local_z),
        IVec3::new(voxel_x, voxel_y, voxel_z),
    )
}

/// Detección de suelo mejorada usando raycast hacia abajo
/// 
/// Inspirado en "Lay of the Land" - encuentra la superficie real del terreno
/// para evitar que los drops traspasen o queden flotando.
pub fn find_ground_height(
    position: Vec3,
    chunk_system: &DynamicChunkSystem,
    max_distance: f32,
) -> Option<f32> {
    let ray_origin = position;
    let ray_direction = Vec3::NEG_Y; // Hacia abajo
    
    // Raycast hacia abajo para encontrar superficie sólida
    if let Some(hit_pos) = raycast_ground(ray_origin, ray_direction, max_distance, chunk_system) {
        Some(hit_pos.y + VOXEL_SIZE * 0.5) // Superficie + medio voxel
    } else {
        None
    }
}

/// Raycast especializado para detección de suelo
fn raycast_ground(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_system: &DynamicChunkSystem,
) -> Option<Vec3> {
    let dir = direction.normalize();
    let mut current_pos = origin;
    let step_size = VOXEL_SIZE * 0.5; // Pasos más pequeños para mayor precisión
    let max_steps = (max_distance / step_size) as i32;

    for _ in 0..max_steps {
        let (chunk_pos, local_pos, _) = world_to_voxel_3d(current_pos);
        
        // Verificar si tenemos este chunk
        if let Some(chunk) = chunk_system.base_chunks.get(&chunk_pos) {
            // Verificar límites del chunk
            if local_pos.x >= 0 && local_pos.x < BASE_CHUNK_SIZE as i32 &&
               local_pos.y >= 0 && local_pos.y < BASE_CHUNK_SIZE as i32 &&
               local_pos.z >= 0 && local_pos.z < BASE_CHUNK_SIZE as i32 {
                
                let voxel_type = chunk.get_voxel_type(
                    local_pos.x as usize,
                    local_pos.y as usize,
                    local_pos.z as usize
                );

                if voxel_type.is_solid() {
                    return Some(current_pos);
                }
            }
        }

        current_pos += dir * step_size;
    }

    None
}

/// Raycast DDA actualizado para chunks 3D dinámicos
pub fn raycast_voxel_3d(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_map: &ChunkMap,
    chunks: &Query<&BaseChunk>,
) -> Option<(Entity, IVec3, IVec3, VoxelType)> {
    let dir = direction.normalize();

    let mut voxel_pos = IVec3::new(
        (origin.x / VOXEL_SIZE).floor() as i32,
        (origin.y / VOXEL_SIZE).floor() as i32,
        (origin.z / VOXEL_SIZE).floor() as i32,
    );

    let step = IVec3::new(
        if dir.x > 0.0 { 1 } else { -1 },
        if dir.y > 0.0 { 1 } else { -1 },
        if dir.z > 0.0 { 1 } else { -1 },
    );

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

    let t_delta = Vec3::new(
        if dir.x != 0.0 { VOXEL_SIZE / dir.x.abs() } else { f32::INFINITY },
        if dir.y != 0.0 { VOXEL_SIZE / dir.y.abs() } else { f32::INFINITY },
        if dir.z != 0.0 { VOXEL_SIZE / dir.z.abs() } else { f32::INFINITY },
    );

    let max_steps = (max_distance / VOXEL_SIZE) as i32 + 1;

    for _ in 0..max_steps {
        let (chunk_pos, local_pos, _) = world_to_voxel_3d(Vec3::new(
            voxel_pos.x as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
            voxel_pos.y as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
            voxel_pos.z as f32 * VOXEL_SIZE + VOXEL_SIZE * 0.5,
        ));

        // Verificar si tenemos este chunk
        if let Some(&chunk_entity) = chunk_map.chunks.get(&chunk_pos) {
            if let Ok(chunk) = chunks.get(chunk_entity) {
                // Verificar limites del chunk
                if local_pos.x >= 0
                    && local_pos.x < BASE_CHUNK_SIZE as i32
                    && local_pos.y >= 0
                    && local_pos.y < BASE_CHUNK_SIZE as i32
                    && local_pos.z >= 0
                    && local_pos.z < BASE_CHUNK_SIZE as i32
                {
                    let voxel_type = chunk.voxel_types[local_pos.x as usize][local_pos.y as usize]
                        [local_pos.z as usize];

                if voxel_type.is_solid() {
                    return Some((chunk_pos, local_pos, voxel_type));
                }
            }
        }

        // Avanzar al siguiente voxel usando DDA
        if t_max.x < t_max.y && t_max.x < t_max.z {
            voxel_pos.x += step.x;
            t_max.x += t_delta.x;
        } else if t_max.y < t_max.z {
            voxel_pos.y += step.y;
            t_max.y += t_delta.y;
        } else {
            voxel_pos.z += step.z;
            t_max.z += t_delta.z;
        }

        let current_distance = (Vec3::new(
            voxel_pos.x as f32 * VOXEL_SIZE,
            voxel_pos.y as f32 * VOXEL_SIZE,
            voxel_pos.z as f32 * VOXEL_SIZE,
        ) - origin).length();

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
/// Actualizado para usar el sistema de chunks dinámicos 3D.
pub fn start_voxel_breaking_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<Camera>>,
    chunk_map: Res<ChunkMap>,
    chunks: Query<&BaseChunk>,
    player_query: Query<&Tool, With<Player>>,
    mut commands: Commands,
    mut breaking_query: Query<(Entity, &mut VoxelBreaking)>,
) {
    if !mouse_input.pressed(MouseButton::Left) {
        // Si suelta el botón, cancelar destrucción en progreso
        for (entity, _) in breaking_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    let ray_origin = camera_transform.translation;
    let ray_direction = camera_transform.forward().as_vec3();

    // Hacer raycast para encontrar voxel usando el nuevo sistema
    let Some((chunk_pos, local_pos, voxel_type)) = raycast_voxel_3d(
        ray_origin,
        ray_direction,
        5.0, // Máximo 5 metros de distancia
        &chunk_system,
    ) else {
        // No encontró nada, cancelar destrucción
        for (entity, _) in breaking_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let tool_type = player_query
        .single()
        .map(|tool| tool.tool_type)
        .unwrap_or(ToolType::None);

    let break_time = calculate_break_time(voxel_type, tool_type);

    // Verificar si ya estamos rompiendo este voxel
    let mut found_existing = false;
    for (entity, breaking) in breaking_query.iter_mut() {
        if breaking.chunk_pos == chunk_pos && breaking.local_pos == local_pos {
            found_existing = true;
            break;
        } else {
            // Estamos mirando otro voxel, cancelar el anterior
            commands.entity(entity).despawn();
        }
    }

    if !found_existing {
        commands.spawn(VoxelBreaking {
            chunk_pos,
            local_pos,
            progress: 0.0,
            break_time,
        });
    }
}

/// Sistema que actualiza el progreso de destrucción de voxels.
/// 
/// Actualizado para usar chunks dinámicos y detección de suelo mejorada.
pub fn update_voxel_breaking_system(
    time: Res<Time>,
    mut breaking_query: Query<(Entity, &mut VoxelBreaking)>,
    mut chunk_queries: ParamSet<(Query<&mut BaseChunk>, Query<&BaseChunk>)>,
    chunk_map: Res<ChunkMap>,
    mut commands: Commands,
    mut player_query: Query<&mut Tool, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut breaking) in breaking_query.iter_mut() {
        breaking.progress += time.delta_secs() / breaking.break_time;

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
                        if target_x < BASE_CHUNK_SIZE && target_y < BASE_CHUNK_SIZE && target_z < BASE_CHUNK_SIZE {
                            let voxel_type = chunk.voxel_types[target_x][target_y][target_z];

                            // Solo destruir si es sólido
                            if voxel_type.is_solid() {
                                // Convertir a aire
                                chunk.voxel_types[target_x][target_y][target_z] = VoxelType::Air;
                                chunk.densities[target_x][target_y][target_z] = -1.0;

                            // Calcular drops
                            let drops = tool_type.calculate_drops(voxel_type);
                            total_drops += drops;

                                // Spawnar drops fisicos usando Rapier
                                if drops > 0 {
                                    spawn_rapier_voxel_drop(
                                        &mut commands,
                                        &mut meshes,
                                        &mut materials,
                                        voxel_type,
                                        drops, 
                                        Vec3::new(
                                            (breaking.chunk_pos.x * BASE_CHUNK_SIZE as i32 + target_x as i32) as f32 * VOXEL_SIZE,
                                            (breaking.chunk_pos.y * BASE_CHUNK_SIZE as i32 + target_y as i32) as f32 * VOXEL_SIZE,
                                            (breaking.chunk_pos.z * BASE_CHUNK_SIZE as i32 + target_z as i32) as f32 * VOXEL_SIZE,
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
                    // Usar el query inmutable para generar el mesh con greedy meshing
                    let chunks_read = chunk_queries.p1();
                    if let Ok(chunk) = chunks_read.get(chunk_entity) {
                        // Generar nuevo mesh con greedy meshing y neighbors
                        let new_mesh = greedy_mesh_basechunk(chunk, &chunk_map, &chunks_read);

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

            // Eliminar el componente de destrucción
            commands.entity(entity).despawn();
        }
    }
}

//! Sistema de destruccion de voxels
//!
//! Premite al jugador romper voxels usando herramientas.

use super::{
    Chunk, VoxelType,
    tools::{Tool, ToolType},
};
use crate::{core::constants::{CHUNK_SIZE, VOXEL_SIZE}, voxel::generate_mesh};
use crate::player::components::Player;
use bevy::prelude::*;
use std::collections::HashMap;

// ============================================================================
// COMPONENTS
// ============================================================================

/// Component que rastrea el progreso de destruccion de un voxel.
#[derive(Component, Debug)]
pub struct VoxelBreaking {
    // Posicion del chunk que contiene el voxel.
    pub chunk_pos: IVec3,

    // Posicion local del voxel dnetro del chunk (0-31).]
    pub local_pos: IVec3,

    // Preogreso de destruccion (0.0 = intacto - 1.0 = roto).
    pub progress: f32,

    // Tiempo total necesario para romper este voxel.
    pub break_time: f32,
}

#[derive(Resource)]
pub struct ChunkMap {
    pub chunks: HashMap<IVec3, Entity>,
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
pub fn world_to_voxel(world_pos: Vec3) -> (IVec3, IVec3, IVec3) {
    // Convertir a coordenadas de voxel
    let voxel_x = (world_pos.x / VOXEL_SIZE).floor() as i32;
    let voxel_y = (world_pos.y / VOXEL_SIZE).floor() as i32;
    let voxel_z = (world_pos.z / VOXEL_SIZE).floor() as i32;

    // Calcular la posicion del chunk
    let chunk_x = voxel_x.div_euclid(CHUNK_SIZE as i32);
    let chunk_y = voxel_y.div_euclid(CHUNK_SIZE as i32);
    let chunk_z = voxel_z.div_euclid(CHUNK_SIZE as i32);

    // Calcular la posicion local del voxel dentro del chunk
    let local_x = voxel_x.rem_euclid(CHUNK_SIZE as i32);
    let local_y = voxel_y.rem_euclid(CHUNK_SIZE as i32);
    let local_z = voxel_z.rem_euclid(CHUNK_SIZE as i32);

    (
        IVec3::new(chunk_x, chunk_y, chunk_z),
        IVec3::new(local_x, local_y, local_z),
        IVec3::new(voxel_x, voxel_y, voxel_z),
    )
}

/// Realiza un raycast para detectar el voxel mas cercano en la linea de vision.
///
/// # Parametros
/// origin: Punto de inicio del rayo (posicion de la camara)
/// direction: Direccion del rayo (direccion de la camara)
/// max distance: Distancia maxima del raycast (en metros)
/// chunks: queary de todos los chunks en el mundo
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
) -> Option<(Entity, IVec3, IVec3, VoxelType)> {
    // Normalizar direccion
    let dir = direction.normalize();

    // Paso del raycast (mas pequeno = mas preciso pero mas costoso)
    let step_size = VOXEL_SIZE * 0.5;
    let max_steps = (max_distance / step_size) as i32;

    // Iterar a lo largo del rayo
    for i in 0..max_steps {
        let distance = i as f32 * step_size;
        let point = origin + dir * distance;

        // Convertir punto a coordenadas de voxel
        let (chunk_pos, local_pos, _) = world_to_voxel(point);

        if let Some(&chunk_entity) = chunk_map.chunks.get(&chunk_pos) {
            let chunk = chunks.get(chunk_entity).unwrap();

            // Verificar que la posicion local este dentro del chunk
            if local_pos.x < 0
                || local_pos.x >= CHUNK_SIZE as i32
                || local_pos.y < 0
                || local_pos.y >= CHUNK_SIZE as i32
                || local_pos.z < 0
                || local_pos.z >= CHUNK_SIZE as i32
            {
                continue;
            }

            // Obtener el tipo de voxel
            let voxel_type =
                chunk.voxel_types[local_pos.x as usize][local_pos.y as usize][local_pos.z as usize];

            // Si es solido, lo encontramos
            if voxel_type.is_solid() {
                return Some((chunk_entity, chunk_pos, local_pos, voxel_type));
            }
        }
    }
    None // No se encontro ningun voxel solido
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
    let Some((chunk_entity, chunk_pos, local_pos, voxel_type)) = raycast_voxel(
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
    for (entity, mut breaking) in breaking_query.iter_mut() {
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
    mut chunks: Query<&mut Chunk>,
    chunk_map: Res<ChunkMap>,
    mut commands: Commands,
    mut player_query: Query<&mut Tool, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_query: Query<&mut Mesh3d>
) {
    for (entity, mut breaking) in breaking_query.iter_mut() {
        // Actualizar preogreso basado en tiempo
        breaking.progress += time.delta_secs() / breaking.break_time;

        // Si llego a 100%, romper el voxel
        if breaking.progress >= 1.0 {
            // Obtener el chunk
            if let Some(&chunk_entity) = chunk_map.chunks.get(&breaking.chunk_pos) {
                if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
                    // Romper el voxel (convertir a aire)
                    let x = breaking.local_pos.x as usize;
                    let y = breaking.local_pos.y as usize;
                    let z = breaking.local_pos.z as usize;

                    // guarda el tipo antes de romperlo (para drops)
                    let broken_voxel_type = chunk.voxel_types[x][y][z];

                    // Convertir a aire
                    chunk.voxel_types[x][y][z] = VoxelType::Air;

                    // Tambien actualzar densidad para que el meshing funcione
                    chunk.densities[x][y][z] = -1.0;

                    // Todo: Generar drops aqui
                    // TODO: Re-mesh del chunk
                    let new_mesh = generate_mesh(&chunk);

                    if let Ok(mut mesh3d) = mesh_query.get_mut(chunk_entity){
                        *mesh3d = Mesh3d(meshes.add(new_mesh));
                    }

                    // Danar herramienta del jugador
                    if let Ok(mut tool) = player_query.single_mut() {
                        let broke = tool.damage(1); // 1 punto de durabilidad 
                        if broke {
                            info!("Herramienta rota");
                            // TODO: Cambiar a manos (ToolType::None) Tambien hacer que desaparesca la heramienta
                        }
                    }
                    info!(
                        "voxel roto {:?} en {:?}",
                        broken_voxel_type, breaking.local_pos
                    );
                }
            }

            // Eliminar el componente de destruccion
            commands.entity(entity).despawn();
        }
    }
}

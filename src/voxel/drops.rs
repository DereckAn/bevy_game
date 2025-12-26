//! Sistema de drops de voxels
//!
//! Maneja los items fisicos que aparecen cuando se destruyen voxels.

use crate::{player::components::Player, voxel::{Chunk, ChunkMap, world_to_voxel}};

use super::VoxelType;
use bevy::prelude::*;
use crate::core::constants::{CHUNK_SIZE, WORLD_HEIGHT};

/// Componente que representa un drop fisico en el mundo.
///
/// los drops son entidades que:
/// - Tienen fisica (gravedad, colisiones)
/// - se auto-recolectan  cuando el jugador se acerca
/// - Se despawnean despues de 60 segundos

#[derive(Component, Debug)]
pub struct VoxelDrop {
    // Tipo de voxel que representa este drop
    pub voxel_type: VoxelType,

    // Cantidad de items en este drop
    pub quantity: u32,

    // Tiempo cuando fue creado(para despawn automatico)
    pub spawn_time: f32,

    // Velocidad inicial del drop
    pub velocity: Vec3, 

    // Si puede ser recolectado (despues de 1 segundo)
    pub can_collect: bool
}

impl VoxelDrop {
    // Crea un nuevo drop
    pub fn new(voxel_type: VoxelType, quantity: u32, current_time: f32) -> Self {
        // Velocidad aleatoria hacia arriba y lados
        let velocity = Vec3::new(
            (rand::random::<f32>() - 0.5) * 4.0, 
            rand::random::<f32>() * 3.0 + 2.0, 
        (rand::random::<f32>() - 0.5) * 4.0,
    );

        Self {
            voxel_type,
            quantity,
            spawn_time: current_time,
            velocity,
            can_collect: false, // No se puede recolectar inmediatamente
        }
    }

    // Verifica si este drop debe se despawneado (mas de 60 segundos)
    pub fn should_despawn(&self, current_time: f32) -> bool {
        current_time - self.spawn_time > 60.0
    }
}

/// Encuentra la altura real del suelo usando raycast hacia abajo 
fn find_ground_height(
    position: Vec3,
    chunk_map: &ChunkMap,
    chunks: &Query<&Chunk>,
) -> f32 {
    let mut test_y = position.y;
    let step = 0.1; // paso de busqueda

    // Log inicial para debug
    println!("Buscando suelo para posición: {:?}", position);

    // Raycast hacia abajo hasta encontrar el suelo solido
    while test_y > -20.0 {
        let test_pos = Vec3::new(position.x, test_y, position.z);
        let (chunk_pos, local_pos, _) = world_to_voxel(test_pos);

        // Log de coordenadas
        if test_y == position.y { // Solo log en la primera iteración
            println!("Chunk pos: {:?}, Local pos: {:?}", chunk_pos, local_pos);
        }

        // Verificar si hay chunk valido
        if let Some(&chunk_entity) = chunk_map.chunks.get(&chunk_pos){
            if let Ok(chunk) = chunks.get(chunk_entity) {
                // Verificar limites del chunk
                if local_pos.x >= 0 && local_pos.x < CHUNK_SIZE as i32 &&
local_pos.y >= 0 && local_pos.y < WORLD_HEIGHT as i32 && 
local_pos.z >= 0 && local_pos.z < CHUNK_SIZE as i32 {

                    // Verificar si es voxel solido
                    let density = chunk.get_density(
                        local_pos.x as usize,
                        local_pos.y as usize,
                        local_pos.z as usize
                    );
                    
                    // Log de densidad para debug
                    if test_y == position.y { // Solo log en la primera iteración
                        println!("Densidad en Y={}: {}", test_y, density);
                    }
                    
                    if density > 0.0 {
                        println!("Suelo encontrado en Y={}", test_y + 0.5);
                        return test_y + 0.5; // Superficie del voxel
                    }
                }
            } else {
                if test_y == position.y {
                    println!("Chunk entity no encontrado");
                }
            }
        } else {
            if test_y == position.y {
                println!("Chunk no existe en ChunkMap");
            }
        }
        test_y -= step; // Continuar hacia abajo
    }
    
    println!("No se encontró suelo, usando fallback Y=0.5");
    0.5 // Fallback si no encuentra suelo
}


/// Sistema que maneja la fisica de los drops (gravedad y velocidad)
pub fn update_drops_system(
    time: Res<Time>,
    mut drop_query: Query<(&mut Transform, &mut VoxelDrop)>,
    chunk_map: Res<ChunkMap>,
    chunks: Query<&Chunk>
) {
   for (mut transform, mut drop) in drop_query.iter_mut() {
    // Aplicar velocidad
    transform.translation += drop.velocity * time.delta_secs();

    // Aplicar gravedad a la velocidad
    drop.velocity.y -= 9.8 * time.delta_secs();

    // Friccion en x y z (se van frenando)
    drop.velocity.x *= 0.98;
    drop.velocity.z *= 0.98;

    // Deteccion real del suelo 
    let ground_height = find_ground_height(transform.translation, &chunk_map, &chunks);

    // Rebote en el suelo
    if transform.translation.y <= ground_height {
        transform.translation.y = ground_height;
        drop.velocity.y = drop.velocity.y.abs() * 0.3; // Rebote con perdida de energia.
    }

    // Despues de 1 segundo, permitir recoleccion
    let current_time = time.elapsed_secs();
    if current_time - drop.spawn_time > 1.0 {
        drop.can_collect = true;
    }
   }
}

/// Sistema que recolecta drops cuando el jugador se acerca
pub fn collect_drop_system(
    mut commands: Commands,
    player_query: Query<&Transform, (With<Player>, Without<VoxelDrop>)>,
    drop_query: Query<(Entity, &Transform, &VoxelDrop), Without<Player>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (entity, drop_transform, drop) in drop_query.iter() {
        // Solo recoletar si ya puede ser recolectado
        if !drop.can_collect { continue; }

        let distance = player_transform
            .translation
            .distance(drop_transform.translation);

        // Auto-recolectar si esta dentro de 2 metros
        if distance <= 2.0 {
            info!("recolectado {:?} x{}", drop.voxel_type, drop.quantity);
            commands.entity(entity).despawn();
            // TODO: Agregar al inventario del jugador
        }
    }
}

/// Sistema que despawnea drops despues de 60 segundos
pub fn clean_old_drops_system(
    mut commands: Commands,
    time: Res<Time>,
    drop_query: Query<(Entity, &VoxelDrop)>,
) {
    let current_time = time.elapsed_secs();

    for (entity, drop) in drop_query.iter() {
        if drop.should_despawn(current_time) {
            info!("Drop despawneado por tiempo {:?}", drop.voxel_type);
            commands.entity(entity).despawn();
        }
    }
}

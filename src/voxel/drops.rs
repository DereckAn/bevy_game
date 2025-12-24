//! Sistema de drops de voxels
//!
//! Maneja los items fisicos que aparecen cuando se destruyen voxels.

use crate::player::components::Player;

use super::VoxelType;
use bevy::prelude::*;

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
}

impl VoxelDrop {
    // Crea un nuevo drop
    pub fn new(voxel_type: VoxelType, quantity: u32, current_time: f32) -> Self {
        Self {
            voxel_type,
            quantity,
            spawn_time: current_time,
        }
    }

    // Verifica si este drop debe se despawneado (mas de 60 segundos)
    pub fn should_despawn(&self, current_time: f32) -> bool {
        current_time - self.spawn_time > 60.0
    }
}

/// Sistema que maneja la fisica de los drops (gravedad)
pub fn update_drops_system(
    time: Res<Time>,
    mut drop_query: Query<&mut Transform, With<VoxelDrop>>,
) {
    for mut transform in drop_query.iter_mut() {
        // Aplicar gravedad simple
        transform.translation.y -= 9.8 * time.delta_secs();

        // Evita que caigan por debajo del suelo (y = 0)
        if transform.translation.y < 0.5 {
            transform.translation.y = 0.5;
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
//! Módulo de física 
//! 
//! Sistema de física usando Rapier para colisiones realistas de terreno y drops de voxels.
//! Optimizado para multijugador con física determinística.

pub mod components;
pub mod rapier_integration;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

// Re-exportar componentes de Rapier que usamos
pub use bevy_rapier3d::prelude::{RigidBody, Collider, Velocity, Restitution, Friction};

// Re-exportar nuestras funciones personalizadas
pub use rapier_integration::{
    RapierVoxelDrop, 
    spawn_rapier_voxel_drop, 
    collect_rapier_drops_system,
    update_rapier_drops_system,
    create_chunk_collider
};

/// Plugin de física que configura Rapier para el juego de voxels
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            // Agregar Rapier plugin con configuración básica
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
            // .add_plugins(RapierDebugRenderPlugin::default()) // Para debug visual
            
            // Agregar sistemas de drops
            .add_systems(Update, (
                update_rapier_drops_system,
                collect_rapier_drops_system,
            ));
    }
}

/// Función helper para crear collider de terreno (mantener compatibilidad)
pub fn create_terrain_collider(mesh: &Mesh) -> Collider {
    create_chunk_collider(mesh)
}

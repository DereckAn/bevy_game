use crate::voxel::{VoxelType, VOXEL_TYPE_COUNT};
use bevy::prelude::*;

/// Conteo de voxels recolectados por el jugador, indexado por `VoxelType as usize`.
#[derive(Resource, Default)]
pub struct Inventory(pub [u32; VOXEL_TYPE_COUNT]);

impl Inventory {
    /// Suma `qty` voxels del tipo dado al inventario.
    pub fn add(&mut self, voxel_type: VoxelType, qty: u32) {
        self.0[voxel_type as usize] += qty;
    }
}

// Recurso de configuración insertado al arranque; los sistemas de cámara/movimiento
// aún no leen estos campos (pendiente de conectar el menú de ajustes).
#[allow(dead_code)]
#[derive(Resource, Default)]
pub struct GameSettings {
    pub mouse_sensitivity: f32,
    pub movement_speed: f32,
    pub fov: f32,
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            mouse_sensitivity: 0.002,
            movement_speed: 5.0,
            fov: 90.0,
        }
    }
}

/// Semilla del mundo actual.
///
/// Se aleatoriza en cada arranque para generar un mapa distinto cada vez.
#[derive(Resource, Clone, Copy)]
pub struct WorldSeed(pub i32);

impl WorldSeed {
    /// Genera una semilla aleatoria nueva.
    pub fn random() -> Self {
        Self(rand::random::<i32>())
    }
}

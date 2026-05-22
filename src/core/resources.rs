use bevy::prelude::*;

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

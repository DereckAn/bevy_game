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

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
    Menu,
}
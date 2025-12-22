use bevy::prelude::*;

#[derive(Component)]
pub struct Player {
    pub speed: f32,
    pub sensitivity: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.002,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

#[derive(Component)]
pub struct PlayerPhysics {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub can_jump: bool,
}

impl Default for PlayerPhysics {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            is_grounded: false,
            can_jump: true,
        }
    }
}

#[derive(Component)]
pub struct PlayerController;

/// Crea la entidad del jugador con cámara 3D y física.
pub fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player::default(),
        PlayerPhysics::default(),
        PlayerController,
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 0.0), // Spawn más alto para probar la caída
    ));
}
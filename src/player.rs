//! # Sistema del jugador 
//! 
//! Maneja la camara en primera persona con controles estils FPS:
//! - WASD: Movimiento horizontal
//! Space/Shift: Subir/Bajar
//! Mouse: Mirar alrededor 
//! Click izquierdo: Captirar cursor
//! Escape: Liberar cursor


use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow, CursorOptions};

// ============================================================================
// PLUGIN
// ============================================================================

// plugin que encapsula toda la funcionalidad del jugador. 
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (player_look, player_move, cursor_grab));
    }
}


// ============================================================================
// COMPONENTES
// ============================================================================

// Componente que define las propiedades del jugador.
#[derive(Component)]
pub struct Player {
    // Velocidad de movimiento en unidades por segundo.
    pub speed: f32,
    // Sensibilidad del mouse (radianes por pixel)
    pub sensitivity: f32,
    // Rotacion vertical actual (radianes, limitado a +- 1.5)
    pub pitch: f32,
    // Rotacion horizontal actual (radianes)
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

// ============================================================================
// SISTEMAS
// ============================================================================

/// Crea la entidad del jugador con cámara 3D.
fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player::default(),
        Camera3d::default(),
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
}


/// Procesa el movimiento del mouse para rotar la camara.
/// 
/// Usa rotacion Euler YXZ para evitar gimbal lock en movimientos tipicos de fps.
fn player_look(
    mut motion: EventReader<MouseMotion>,
    mut query: Query<(&mut Player, &mut Transform)>,
) {
    let Ok((mut player, mut transform)) = query.single_mut() else {
        return;
    };

    for ev in motion.read() {
        player.yaw -= ev.delta.x * player.sensitivity;
        player.pitch -= ev.delta.y * player.sensitivity;
        player.pitch = player.pitch.clamp(-1.5, 1.5);
    }

    transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
}

/// Procesa input de teclado para mover al jugador.
/// 
/// El movimiento es relativo a la orientacion de la camara (forward/right)
/// pero el movimiento vertical (Space/Shift) es absoluto en el eje Y.

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &mut Transform)>,
) {
    let Ok((player, mut transform)) = query.single_mut() else {
        return;
    };

    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir.x += 1.0;
    }
    if keys.pressed(KeyCode::Space) {
        dir.y += 1.0;
    }
    if keys.pressed(KeyCode::ShiftLeft) {
        dir.y -= 1.0;
    }

    if dir != Vec3::ZERO {
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        let move_dir = (forward * -dir.z + right * dir.x).normalize_or_zero();
        let move_dir = Vec3::new(move_dir.x, dir.y, move_dir.z).normalize_or_zero();
        transform.translation += move_dir * player.speed * time.delta_secs();
    }
}

/// Captura/libera el cursos del mouse.
/// 
/// - Click izuierdo: Bloquea y oculta el cursor
/// - Escapte: Libera y muestra el cursor
fn cursor_grab(
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut cursor) = cursor.single_mut() else { return };

    if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

// ============================================================================
// TESTS
// ============================================================================


#[cfg(test)]
mod tests {
    use super::*;

    #[test]


    fn test_player_default_values() {
        let player = Player::default();
        assert_eq!(player.speed, 5.0);
        assert_eq!(player.sensitivity, 0.002);
        assert_eq!(player.pitch, 0.0);
        assert_eq!(player.yaw, 0.0);
    }

    #[test]
    fn test_pitch_clamping() {
        let mut player = Player::default();
        
        player.pitch = 2.0;
        player.pitch = player.pitch.clamp(-1.5, 1.5);
        assert_eq!(player.pitch, 1.5);
        
        player.pitch = -2.0;
        player.pitch = player.pitch.clamp(-1.5, 1.5);
        assert_eq!(player.pitch, -1.5);
    }
}
use bevy::prelude::*;
use crate::core::{GRAVITY, JUMP_FORCE};
use super::components::{Player, PlayerPhysics, PlayerController};

/// Sistema de movimiento del jugador con física de salto.
pub fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Player, &mut PlayerPhysics, &mut Transform), With<PlayerController>>,
) {
    let Ok((player, mut physics, mut transform)) = query.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // Input horizontal (WASD)
    let mut input_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input_dir.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input_dir.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input_dir.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input_dir.x += 1.0;
    }

    // Salto
    if keys.just_pressed(KeyCode::Space) && physics.is_grounded && physics.can_jump {
        physics.velocity.y = JUMP_FORCE;
        physics.is_grounded = false;
    }

    // Aplicar gravedad
    if !physics.is_grounded {
        physics.velocity.y += GRAVITY * dt;
    }

    // Movimiento horizontal relativo a la cámara
    if input_dir != Vec3::ZERO {
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        let move_dir = (forward * -input_dir.z + right * input_dir.x).normalize_or_zero();
        
        // Solo movimiento horizontal, la velocidad Y se maneja por separado
        physics.velocity.x = move_dir.x * player.speed;
        physics.velocity.z = move_dir.z * player.speed;
    } else {
        // Fricción horizontal cuando no hay input
        physics.velocity.x *= 0.8;
        physics.velocity.z *= 0.8;
    }

    // Aplicar velocidad a la posición
    let new_position = transform.translation + physics.velocity * dt;
    
    // Ground check simple (por ahora, hasta que tengamos colisiones con el terreno)
    if new_position.y <= 0.0 {
        transform.translation.x = new_position.x;
        transform.translation.z = new_position.z;
        transform.translation.y = 0.0;
        physics.velocity.y = 0.0;
        physics.is_grounded = true;
    } else {
        transform.translation = new_position;
        physics.is_grounded = false;
    }
}
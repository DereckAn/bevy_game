use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use super::components::{Player, PlayerController};

/// Procesa el movimiento del mouse para rotar la cámara.
/// 
/// Usa rotación Euler YXZ para evitar gimbal lock en movimientos típicos de fps.
pub fn player_look(
    mut motion: MessageReader<MouseMotion>,
    mut query: Query<(&mut Player, &mut Transform), With<PlayerController>>,
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
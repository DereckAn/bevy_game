// ============================================================================
// IMPORTS - TRAER CÓDIGO DE OTRAS LIBRERÍAS
// ============================================================================

use super::components::{Player, PlayerController};
use bevy::input::mouse::MouseMotion; // Evento de movimiento del mouse
use bevy::prelude::*; // Tipos básicos de Bevy // Nuestros componentes

// ============================================================================
// SISTEMA DE CÁMARA DEL JUGADOR
// ============================================================================

/// Procesa el movimiento del mouse para rotar la cámara.
///
/// Usa rotación Euler YXZ para evitar gimbal lock en movimientos típicos de fps.
/// El gimbal lock es un problema donde se pierden grados de libertad en ciertas rotaciones.
pub fn player_look(
    // ========================================================================
    // PARÁMETROS DEL SISTEMA
    // ========================================================================
    mut motion: MessageReader<MouseMotion>, // Lector de eventos de movimiento del mouse (mutable)
    mut body_query: Query<
        // Query mutable para buscar entidades del jugador
        (&mut Player, &mut Transform), // Componentes que necesitamos:
        //   - Player: para acceder a yaw, pitch, sensitivity (mutable)
        //   - Transform: para modificar la rotación (mutable)
        With<PlayerController>, // Filtro: solo entidades con PlayerController
    >,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<PlayerController>)>, // Query para la cámara 3D (sin PlayerController)
) {
    let Ok((mut player, mut body)) = body_query.single_mut() else {
        return;
    };
    let Ok(mut camera) = camera_query.single_mut() else {
        return;
    };

    for ev in motion.read() {
        player.yaw -= ev.delta.x * player.sensitivity;
        player.pitch -= ev.delta.y * player.sensitivity;
        player.pitch = player.pitch.clamp(-1.5, 1.5);
    }

    // Separar yaw/pitch evita que el collider se incline al mirar arriba/abajo
    body.rotation = Quat::from_rotation_y(player.yaw);
    camera.rotation = Quat::from_rotation_x(player.pitch);
}

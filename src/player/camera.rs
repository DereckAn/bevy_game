// ============================================================================
// IMPORTS - TRAER CÓDIGO DE OTRAS LIBRERÍAS
// ============================================================================

use bevy::input::mouse::MouseMotion;               // Evento de movimiento del mouse
use bevy::prelude::*;                              // Tipos básicos de Bevy
use super::components::{Player, PlayerController}; // Nuestros componentes

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
    mut motion: MessageReader<MouseMotion>,        // Lector de eventos de movimiento del mouse (mutable)
    mut query: Query<                              // Query mutable para buscar entidades del jugador
        (&mut Player, &mut Transform),             // Componentes que necesitamos:
                                                   //   - Player: para acceder a yaw, pitch, sensitivity (mutable)
                                                   //   - Transform: para modificar la rotación (mutable)
        With<PlayerController>                     // Filtro: solo entidades con PlayerController
    >,
) {
    // ========================================================================
    // OBTENER LA ENTIDAD DEL JUGADOR
    // ========================================================================
    
    let Ok((mut player, mut transform)) = query.single_mut() else {
        return;                                    // Si no hay jugador, salir
    };

    // ========================================================================
    // PROCESAR EVENTOS DE MOVIMIENTO DEL MOUSE
    // ========================================================================
    
    // Iterar sobre todos los eventos de movimiento del mouse de este frame
    for ev in motion.read() {                      // motion.read() devuelve un iterador de eventos MouseMotion
        
        // Actualizar rotación horizontal (yaw)
        player.yaw -= ev.delta.x * player.sensitivity;
        // Explicación:
        // - ev.delta.x: movimiento horizontal del mouse en pixels
        // - player.sensitivity: convierte pixels a radianes (ej: 0.002 rad/pixel)
        // - Signo negativo: mouse derecha = rotar izquierda (convención FPS)
        
        // Actualizar rotación vertical (pitch)  
        player.pitch -= ev.delta.y * player.sensitivity;
        // Explicación:
        // - ev.delta.y: movimiento vertical del mouse en pixels
        // - Signo negativo: mouse arriba = mirar arriba (convención FPS)
        
        // Limitar rotación vertical para evitar dar vueltas completas
        player.pitch = player.pitch.clamp(-1.5, 1.5);
        // Explicación:
        // - clamp(min, max): limita el valor entre min y max
        // - -1.5 a 1.5 radianes ≈ -86° a +86° (casi vertical pero no completamente)
        // - Esto evita que el jugador pueda mirar "detrás de su cabeza"
    }

    // ========================================================================
    // APLICAR ROTACIÓN AL TRANSFORM
    // ========================================================================
    
    // Convertir yaw y pitch a una rotación 3D y aplicarla al transform
    transform.rotation = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    // Explicación:
    // - Quat: quaternion, representación matemática de rotación 3D
    // - from_euler: crea quaternion desde ángulos de Euler
    // - EulerRot::YXZ: orden de rotación (Y primero, luego X, luego Z)
    //   * Y (yaw): rotación horizontal alrededor del eje Y (arriba/abajo del mundo)
    //   * X (pitch): rotación vertical alrededor del eje X (izquierda/derecha)  
    //   * Z (roll): rotación de inclinación alrededor del eje Z (siempre 0.0 para FPS)
    // - Este orden YXZ evita el gimbal lock en cámaras FPS típicas
}
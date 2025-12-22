// ============================================================================
// IMPORTS - TRAER CÓDIGO DE OTRAS LIBRERÍAS
// ============================================================================

use bevy::prelude::*;                              // Tipos básicos de Bevy (Vec3, Query, Res, etc.)
use bevy_rapier3d::prelude::*;                     // Tipos de física (Velocity)
use super::components::{Player, PlayerController}; // Nuestros componentes desde el módulo padre

// ============================================================================
// SISTEMA DE MOVIMIENTO DEL JUGADOR
// ============================================================================

/// Sistema de movimiento del jugador con física de Rapier.
/// 
/// Este sistema se ejecuta cada frame y procesa el input del teclado
/// para mover al jugador usando el motor de física Rapier.
pub fn player_movement(
    // ========================================================================
    // PARÁMETROS DEL SISTEMA
    // ========================================================================
    keys: Res<ButtonInput<KeyCode>>,               // Recurso de solo lectura para detectar teclas presionadas
    mut query: Query<                              // Query mutable para buscar entidades específicas
        (&Player, &mut Velocity, &Transform),     // Tupla de componentes que necesitamos:
                                                   //   - Player: propiedades del jugador (solo lectura)
                                                   //   - Velocity: velocidad física (mutable)
                                                   //   - Transform: posición y rotación (solo lectura)
        With<PlayerController>                     // Filtro: solo entidades que tengan PlayerController
    >,
) {
    // ========================================================================
    // OBTENER LA ENTIDAD DEL JUGADOR
    // ========================================================================
    
    // Intenta obtener la única entidad que coincida con el query
    let Ok((player, mut velocity, transform)) = query.single_mut() else {
        return;                                    // Si no hay jugador o hay más de uno, salir
    };
    // Explicación de la sintaxis:
    // - query.single_mut() retorna Result<(componentes), QuerySingleError>
    // - let Ok(...) = ... else { return; } es pattern matching
    // - Si es Ok, extrae los componentes; si es Err, ejecuta el else

    // ========================================================================
    // PROCESAR INPUT HORIZONTAL (WASD)
    // ========================================================================
    
    // Input horizontal (WASD)
    let mut input_dir = Vec3::ZERO;                // Vector de dirección inicial (0, 0, 0)
    
    if keys.pressed(KeyCode::KeyW) {               // Si W está presionada
        input_dir.z -= 1.0;                       // Mover hacia adelante (Z negativo en Bevy)
    }
    if keys.pressed(KeyCode::KeyS) {               // Si S está presionada  
        input_dir.z += 1.0;                       // Mover hacia atrás (Z positivo)
    }
    if keys.pressed(KeyCode::KeyA) {               // Si A está presionada
        input_dir.x -= 1.0;                       // Mover hacia la izquierda (X negativo)
    }
    if keys.pressed(KeyCode::KeyD) {               // Si D está presionada
        input_dir.x += 1.0;                       // Mover hacia la derecha (X positivo)
    }
    
    // Nota: input_dir ahora contiene la dirección deseada en coordenadas locales
    // Por ejemplo: W+D = (-1, 0, 1) = diagonal adelante-derecha

    // ========================================================================
    // APLICAR MOVIMIENTO RELATIVO A LA CÁMARA
    // ========================================================================
    
    // Movimiento horizontal relativo a la cámara
    if input_dir != Vec3::ZERO {                   // Si hay algún input de movimiento
        
        // Obtener vectores de dirección de la cámara
        let forward = transform.forward().as_vec3(); // Vector "adelante" de la cámara
        let right = transform.right().as_vec3();     // Vector "derecha" de la cámara
        
        // Calcular dirección de movimiento en el mundo
        let move_dir = (forward * -input_dir.z + right * input_dir.x) // Combinar adelante/atrás + izquierda/derecha
            .normalize_or_zero();                    // Normalizar a longitud 1 (o 0 si es vector nulo)
        
        // Explicación de la fórmula:
        // - forward * -input_dir.z: si input_dir.z = -1 (W), entonces forward * 1 = adelante
        // - right * input_dir.x: si input_dir.x = 1 (D), entonces right * 1 = derecha
        // - La suma da la dirección diagonal correcta
        
        // Aplicar velocidad horizontal
        velocity.linvel.x = move_dir.x * player.speed; // Velocidad X = dirección X * velocidad del jugador
        velocity.linvel.z = move_dir.z * player.speed; // Velocidad Z = dirección Z * velocidad del jugador
        
    } else {
        // Fricción horizontal cuando no hay input
        velocity.linvel.x *= 0.8;                  // Reducir velocidad X al 80% (fricción)
        velocity.linvel.z *= 0.8;                  // Reducir velocidad Z al 80% (fricción)
        // Esto hace que el jugador se detenga gradualmente cuando no presiona teclas
    }

    // ========================================================================
    // PROCESAR SALTO
    // ========================================================================
    
    // Salto simple por ahora
    if keys.just_pressed(KeyCode::Space) {         // Si Space fue presionada este frame (no mantenida)
        velocity.linvel.y = player.jump_force;     // Aplicar velocidad vertical hacia arriba
    }
    // Nota: La gravedad se encarga automáticamente por Rapier, no necesitamos manejarla aquí
}
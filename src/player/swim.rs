//! Física de nado del jugador en el bioma manglar.
//!
//! No hay motor de fluidos: se modula la `GravityScale` de Rapier según cuánto
//! cuerpo esté sumergido y se deja que Rapier integre. Esa es la diferencia clave
//! frente a escribir `linvel.y` a mano — la gravedad efectiva cruza cero a media
//! inmersión, así que la flotación tiene un EQUILIBRIO real en la superficie en
//! vez de rebotar dentro/fuera del agua.

use super::components::PlayerController;
use crate::voxel::{world_to_voxel, BaseChunk, ChunkMap, VoxelType};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// Velocidad de nado vertical (m/s) al pulsar Space (subir) o Shift (bajar).
const SWIM_SPEED: f32 = 4.0;

/// Fracción del cuerpo sumergida en la que la gravedad efectiva es CERO: el
/// jugador flota aquí por sí solo. 0.5 = medio cuerpo fuera del agua.
const NEUTRAL_BUOYANCY: f32 = 0.5;

/// Fracción de la velocidad vertical que sobrevive a un segundo bajo el agua.
/// Sin esto la gravedad efectiva alterna de signo alrededor de la superficie y el
/// jugador oscila; con esto converge. Elevado a `dt` → independiente de los FPS.
const VERTICAL_DAMPING_PER_SEC: f32 = 0.01;

/// Cuánto frena el agua el avance horizontal. Se aplica a la velocidad que
/// `player_movement` acaba de FIJAR desde el input, así que con teclas pulsadas el
/// resultado es exactamente este factor, sin depender de los FPS.
const WATER_SPEED_FACTOR: f32 = 0.5;

/// Altura de la cápsula del jugador (m). Espejo de `spawn_player`: media altura
/// 0.9 → 1.8 m en total, con el `Transform` en el CENTRO de la cápsula.
const PLAYER_HEIGHT: f32 = 1.8;

/// Marca al jugador mientras alguna parte de su cuerpo está en el agua.
/// `player_movement` la consulta para no saltar mientras se nada: en el agua la
/// vertical la controla solo `water_physics`.
#[derive(Component)]
pub struct Swimming;

/// ¿Está la posición mundial dentro de un voxel de agua cargado?
fn is_water_at(world_pos: Vec3, chunk_map: &ChunkMap, chunks: &Query<&BaseChunk>) -> bool {
    let (chunk_pos, local, _) = world_to_voxel(world_pos);
    let Some(&entity) = chunk_map.chunks.get(&chunk_pos) else {
        return false;
    };
    let Ok(chunk) = chunks.get(entity) else {
        return false;
    };
    chunk.voxel_types[local.x as usize][local.y as usize][local.z as usize] == VoxelType::Water
}

/// Fracción del cuerpo sumergida (0.0 a 1.0), muestreando 5 alturas de la cápsula
/// desde los pies hasta la cabeza.
///
/// Muestrear un solo punto (el centro) daba un control todo-o-nada: en la
/// superficie el centro entraba y salía del agua cada pocos frames y la
/// flotabilidad se encendía y apagaba con él. Varias muestras dan una rampa, y una
/// rampa sí puede equilibrarse.
fn submerged_fraction(center: Vec3, chunk_map: &ChunkMap, chunks: &Query<&BaseChunk>) -> f32 {
    const SAMPLES: usize = 5;
    let bottom = center.y - PLAYER_HEIGHT * 0.5;
    let step = PLAYER_HEIGHT / (SAMPLES - 1) as f32;

    let wet = (0..SAMPLES)
        .filter(|i| {
            let pos = Vec3::new(center.x, bottom + step * *i as f32, center.z);
            is_water_at(pos, chunk_map, chunks)
        })
        .count();

    wet as f32 / SAMPLES as f32
}

/// `GravityScale` para una fracción sumergida: +1 en seco (caída normal), 0 en
/// `NEUTRAL_BUOYANCY` (el jugador se queda flotando) y negativa más abajo (empuje
/// neto hacia arriba). Que cruce cero es lo que da un equilibrio en la superficie.
fn buoyant_gravity_scale(submerged: f32) -> f32 {
    (NEUTRAL_BUOYANCY - submerged) / NEUTRAL_BUOYANCY
}

/// Aplica flotabilidad y nado según la inmersión. Corre DESPUÉS de
/// `player_movement`: frena la velocidad horizontal que aquel fijó y toma el
/// control de la vertical.
pub fn water_physics(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    chunk_map: Res<ChunkMap>,
    chunks: Query<&BaseChunk>,
    mut query: Query<
        (Entity, &Transform, &mut Velocity, &mut GravityScale),
        With<PlayerController>,
    >,
) {
    let Ok((entity, transform, mut velocity, mut gravity)) = query.single_mut() else {
        return;
    };

    let submerged = submerged_fraction(transform.translation, &chunk_map, &chunks);

    if submerged == 0.0 {
        // Fuera del agua: gravedad normal. Se comprueba antes de escribir para no
        // disparar la detección de cambios de Rapier en cada frame en seco.
        if gravity.0 != 1.0 {
            gravity.0 = 1.0;
        }
        commands.entity(entity).remove::<Swimming>();
        return;
    }

    commands.entity(entity).insert(Swimming);

    // Rapier integra la gravedad efectiva; aquí no se toca `linvel.y` salvo para
    // nadar a mano.
    gravity.0 = buoyant_gravity_scale(submerged);

    if keys.pressed(KeyCode::Space) {
        velocity.linvel.y = SWIM_SPEED;
    } else if keys.pressed(KeyCode::ShiftLeft) {
        velocity.linvel.y = -SWIM_SPEED;
    } else {
        velocity.linvel.y *= VERTICAL_DAMPING_PER_SEC.powf(time.delta_secs());
    }

    velocity.linvel.x *= WATER_SPEED_FACTOR;
    velocity.linvel.z *= WATER_SPEED_FACTOR;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La versión anterior empujaba hacia arriba con fuerza constante mientras el
    /// centro de la cápsula tocaba agua, así que el jugador salía disparado, caía y
    /// volvía a entrar: no existía ninguna inmersión estable.
    #[test]
    fn gravity_cancels_out_at_the_floating_line() {
        assert_eq!(buoyant_gravity_scale(NEUTRAL_BUOYANCY), 0.0);
    }

    #[test]
    fn fully_submerged_pushes_the_player_up() {
        assert!(buoyant_gravity_scale(1.0) < 0.0);
    }

    #[test]
    fn barely_submerged_still_pulls_the_player_down() {
        assert!(buoyant_gravity_scale(0.2) > 0.0);
    }
}

//! Física de nado del jugador en el bioma manglar.
//!
//! Cuando la cápsula del jugador queda dentro de un voxel de agua, se reduce la
//! gravedad (flotabilidad), se amortigua el movimiento (el agua frena) y se nada
//! con Space (subir) / Shift (bajar). Reutiliza la `Velocity` de Rapier: no añade
//! un motor de fluidos, solo modula la velocidad existente.

use super::components::PlayerController;
use crate::voxel::{world_to_voxel, BaseChunk, ChunkMap, VoxelType};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// Velocidad de nado vertical (m/s) al pulsar Space (subir) o Shift (bajar).
const SWIM_SPEED: f32 = 4.0;
/// Factor de frenado horizontal bajo el agua (por frame, estilo de la fricción
/// existente en `player_movement`).
const WATER_DRAG: f32 = 0.6;
/// Escala de gravedad bajo el agua: hundimiento lento en vez de caída plena.
const SUBMERGED_GRAVITY: f32 = 0.25;

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

/// Aplica flotabilidad y nado cuando el jugador está sumergido. Corre DESPUÉS de
/// `player_movement`: amortigua la velocidad horizontal que aquel fijó y controla
/// la vertical (empuje hacia la superficie + nado manual).
pub fn water_physics(
    keys: Res<ButtonInput<KeyCode>>,
    chunk_map: Res<ChunkMap>,
    chunks: Query<&BaseChunk>,
    mut query: Query<(&Transform, &mut Velocity, &mut GravityScale), With<PlayerController>>,
) {
    let Ok((transform, mut velocity, mut gravity)) = query.single_mut() else {
        return;
    };

    if !is_water_at(transform.translation, &chunk_map, &chunks) {
        gravity.0 = 1.0; // Fuera del agua: gravedad normal.
        return;
    }

    gravity.0 = SUBMERGED_GRAVITY;

    // Vertical: nado manual con Space/Shift; en reposo, leve empuje hacia arriba
    // (flota hasta la superficie).
    if keys.pressed(KeyCode::Space) {
        velocity.linvel.y = SWIM_SPEED;
    } else if keys.pressed(KeyCode::ShiftLeft) {
        velocity.linvel.y = -SWIM_SPEED;
    } else {
        velocity.linvel.y = (velocity.linvel.y * 0.8 + 0.5).clamp(-SWIM_SPEED, SWIM_SPEED);
    }

    // Horizontal: el agua frena el nado.
    velocity.linvel.x *= WATER_DRAG;
    velocity.linvel.z *= WATER_DRAG;
}

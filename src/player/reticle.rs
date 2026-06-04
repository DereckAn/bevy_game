//! Mira (crosshair) y resaltado del voxel apuntado.
//!
//! El crosshair es un "+" centrado en pantalla (UI). El resaltado se dibuja con
//! Gizmos (modo inmediato): cada frame se lanza un raycast desde la cámara y, si
//! golpea un voxel sólido, se dibuja un recuadro wireframe a su alrededor. Si
//! apunta al aire, no se dibuja nada — sin entidades que gestionar.

use crate::{
    core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE},
    voxel::{raycast_voxel, BaseChunk, ChunkMap},
};
use bevy::prelude::*;

/// Marcador de la entidad raíz del crosshair (para poder despawnearlo).
#[derive(Component)]
pub struct Crosshair;

/// Spawnea el "+" centrado en la pantalla.
pub fn spawn_crosshair(mut commands: Commands) {
    commands
        .spawn((
            Crosshair,
            // Contenedor a pantalla completa que centra a su hijo
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("+"),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Despawnea el crosshair (al pausar o volver al menú).
pub fn despawn_crosshair(mut commands: Commands, crosshairs: Query<Entity, With<Crosshair>>) {
    for entity in &crosshairs {
        commands.entity(entity).despawn();
    }
}

/// Dibuja un recuadro wireframe alrededor del voxel que el jugador apunta.
pub fn highlight_aimed_voxel(
    camera_query: Query<&Transform, With<Camera>>,
    chunk_map: Res<ChunkMap>,
    chunks: Query<&BaseChunk>,
    mut gizmos: Gizmos,
) {
    let Ok(camera) = camera_query.single() else {
        return;
    };

    let origin = camera.translation;
    let direction = camera.forward().as_vec3();

    // Mismo alcance que la destrucción (5 m): solo resaltamos lo que se puede romper
    if let Some((_entity, chunk_pos, local_pos, _voxel_type)) =
        raycast_voxel(origin, direction, 5.0, &chunk_map, &chunks)
    {
        // Índice global del voxel → centro en coordenadas de mundo
        let voxel_index = chunk_pos * BASE_CHUNK_SIZE as i32 + local_pos;
        let center = (voxel_index.as_vec3() + Vec3::splat(0.5)) * VOXEL_SIZE;

        // Cubo wireframe del tamaño de un voxel (la escala del Transform = lado del cubo)
        gizmos.cuboid(
            Transform::from_translation(center).with_scale(Vec3::splat(VOXEL_SIZE)),
            Color::BLACK,
        );
    }
}

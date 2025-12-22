//! # Punto de entrada principal del juego voxels
//! 
//! Este módulo inicializa la aplicación Bevy, configura los plugins necesarios 
//! y genera la escena inicial con chunks de terreno e iluminación.

mod core;
mod voxel;
mod player;

use bevy::prelude::*;
use core::GameSettings;
use voxel::{Chunk, generate_mesh};
use player::PlayerPlugin;

// Punto de entrada de la aplicación
// Configura bevy con plugins por defecto, el plugin del jugador y el sistema de setup
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .insert_resource(GameSettings::new())
        .add_systems(Startup, setup)
        .run();
}

/// Sistema de inicialización que genera la escena. 
/// 
/// Crea una grilla de 3x3 chunks centrada en el origen y añade iluminación
/// 
/// # Parámetros
/// - `commands`: Comandos para crear entidades y recursos en el mundo.
/// - `meshes`: Recursos para almacenar y gestionar las mallas 3D.
/// - `materials`: Recursos para almacenar y gestionar los materiales estándar.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Generar varios chunks
    for cx in -5..=5 {
        for cz in -5..=5 {
            let chunk = Chunk::new(IVec3::new(cx, 0, cz));
            let mesh = generate_mesh(&chunk);

            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.5, 0.3),
                    cull_mode: None,
                    ..default()
                })),
                Transform::default(),
                chunk,
            ));
        }
    }

    // Luz direccional (simula el sol)
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

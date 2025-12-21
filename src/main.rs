mod voxel;
mod player;

use bevy::prelude::*;
use voxel::{Chunk, generate_mesh};
use player::PlayerPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Generar varios chunks
    for cx in -1..=1 {
        for cz in -1..=1 {
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

    // Luz
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

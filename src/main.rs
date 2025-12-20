mod voxel;

use voxel::{Chunk, generate_mesh};
use bevy::prelude::*;
use bevy::prelude::{Camera3d, DirectionalLight, Mesh3d, MeshMaterial3d, StandardMaterial};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    // Chunk
    let chunk =  Chunk::new(IVec3::ZERO);
    let mesh = generate_mesh(&chunk);

    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color:Color::srgb(0.3, 0.5, 0.3),
            ..default()
        })),
        Transform::default(),
        chunk,
        ));

    // Luz
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));

    // Camara
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
}
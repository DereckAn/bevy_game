//! Rapier Physics Integration for Voxel Drops

use crate::{core::constants::VOXEL_SIZE, voxel::VoxelType};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// Assets compartidos por todos los voxel drops.
///
/// El cubo es idéntico para cada drop y el material solo depende del tipo de
/// voxel (7 tipos). Crearlos una vez y clonar los handles evita asignar un
/// `Mesh` + `StandardMaterial` nuevos por cada drop spawneado.
#[derive(Resource)]
pub struct DropAssets {
    mesh: Handle<Mesh>,
    /// Indexado por `VoxelType as usize` (Air, Dirt, Stone, Wood, Metal, Grass, Sand, Leaves, Foliage)
    materials: [Handle<StandardMaterial>; 9],
}

impl FromWorld for DropAssets {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Cuboid::new(
            VOXEL_SIZE * 0.8,
            VOXEL_SIZE * 0.8,
            VOXEL_SIZE * 0.8,
        ));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mut make = |vt: VoxelType| {
            materials.add(StandardMaterial {
                base_color: vt.properties().color,
                metallic: 0.1,
                perceptual_roughness: 0.8,
                ..default()
            })
        };

        // Orden = valores del enum (repr u8): Air=0 .. Sand=6
        let materials = [
            make(VoxelType::Air),
            make(VoxelType::Dirt),
            make(VoxelType::Stone),
            make(VoxelType::Wood),
            make(VoxelType::Metal),
            make(VoxelType::Grass),
            make(VoxelType::Sand),
            make(VoxelType::Leaves),
            make(VoxelType::Foliage),
        ];

        Self { mesh, materials }
    }
}

impl DropAssets {
    fn material(&self, voxel_type: VoxelType) -> Handle<StandardMaterial> {
        self.materials[voxel_type as usize].clone()
    }
}

/// Component for voxel drops with Rapier physics
#[derive(Component, Debug)]
pub struct RapierVoxelDrop {
    pub voxel_type: VoxelType,
    pub quantity: u32,
    pub spawn_time: f32,
    pub can_collect: bool,
}

impl RapierVoxelDrop {
    pub fn new(voxel_type: VoxelType, quantity: u32, spawn_time: f32) -> Self {
        Self {
            voxel_type,
            quantity,
            spawn_time,
            can_collect: false,
        }
    }

    pub fn should_despawn(&self, current_time: f32) -> bool {
        current_time - self.spawn_time > 60.0
    }
}

/// Spawns a voxel drop with real Rapier physics
pub fn spawn_rapier_voxel_drop(
    commands: &mut Commands,
    drop_assets: &DropAssets,
    voxel_type: VoxelType,
    quantity: u32,
    world_position: Vec3,
    current_time: f32,
) {
    let properties = voxel_type.properties();

    // Handles compartidos (sin asignar mesh/material nuevos por drop)
    let cube_mesh = drop_assets.mesh.clone();
    let material = drop_assets.material(voxel_type);

    // ARREGLO: Spawnar drops siempre arriba del voxel destruido
    let spawn_position = Vec3::new(
        world_position.x,
        world_position.y + VOXEL_SIZE * 2.0, // Spawnar 2 voxels arriba
        world_position.z,
    );

    // ARREGLO: Velocidad inicial siempre hacia arriba con componentes horizontales menores
    let initial_velocity = Vec3::new(
        (rand::random::<f32>() - 0.5) * 2.0, // Reducir velocidad horizontal
        rand::random::<f32>() * 2.0 + 3.0,   // Velocidad hacia arriba más consistente (3-5 m/s)
        (rand::random::<f32>() - 0.5) * 2.0, // Reducir velocidad horizontal
    );

    commands.spawn((
        // Visual components
        Mesh3d(cube_mesh),
        MeshMaterial3d(material),
        Transform::from_translation(spawn_position), // Usar posición corregida
        GlobalTransform::default(),
        Visibility::default(),
        // Game logic component
        RapierVoxelDrop::new(voxel_type, quantity, current_time),
        // Rapier physics components
        RigidBody::Dynamic,
        Collider::cuboid(VOXEL_SIZE * 0.4, VOXEL_SIZE * 0.4, VOXEL_SIZE * 0.4),
        // Physics properties based on voxel type
        AdditionalMassProperties::Mass(properties.density),
        Restitution::coefficient(0.3), // Bounciness
        Friction::coefficient(0.7),    // Surface friction
        // Initial velocity
        Velocity {
            linvel: initial_velocity,
            angvel: Vec3::new(
                (rand::random::<f32>() - 0.5) * 1.0, // Reducir rotación angular
                (rand::random::<f32>() - 0.5) * 1.0,
                (rand::random::<f32>() - 0.5) * 1.0,
            ),
        },
        // Collision groups for optimization
        CollisionGroups::new(Group::GROUP_1, Group::ALL),
    ));
}

/// System to collect drops when player approaches
pub fn collect_rapier_drops_system(
    mut commands: Commands,
    player_query: Query<
        &Transform,
        (
            With<crate::player::components::Player>,
            Without<RapierVoxelDrop>,
        ),
    >,
    drop_query: Query<
        (Entity, &Transform, &RapierVoxelDrop),
        Without<crate::player::components::Player>,
    >,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    for (entity, drop_transform, drop) in drop_query.iter() {
        if !drop.can_collect {
            continue;
        }

        let distance = player_transform
            .translation
            .distance(drop_transform.translation);

        if distance <= 2.0 {
            info!("Collected {:?} x{}", drop.voxel_type, drop.quantity);
            commands.entity(entity).despawn();
            // TODO: Add to player inventory
        }
    }
}

/// System to enable collection after 1 second and clean up old drops
pub fn update_rapier_drops_system(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_query: Query<(Entity, &mut RapierVoxelDrop)>,
) {
    let current_time = time.elapsed_secs();

    for (entity, mut drop) in drop_query.iter_mut() {
        // Enable collection after 1 second
        if current_time - drop.spawn_time > 1.0 {
            drop.can_collect = true;
        }

        // Despawn after 60 seconds
        if drop.should_despawn(current_time) {
            info!("Drop despawned by timeout: {:?}", drop.voxel_type);
            commands.entity(entity).despawn();
        }
    }
}

/// Create terrain collider for a chunk using Rapier
pub fn create_chunk_collider(mesh: &Mesh) -> Collider {
    // Use the correct API for creating mesh colliders
    Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh(Default::default()))
        .unwrap_or_else(|| {
            warn!("Failed to create mesh collider, using default box");
            Collider::cuboid(16.0, 8.0, 16.0) // Fallback box collider
        })
}

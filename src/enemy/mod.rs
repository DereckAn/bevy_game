//! Enemigos: cajas moradas que persiguen al jugador.

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::core::{GameState, WorldKind, WorldSeed};
use crate::player::PlayerController;
use crate::voxel::TerrainGenerator;

const ENEMY_COUNT: usize = 5;
// Dentro del área que `setup` genera de forma síncrona (radio 2 chunks ≈ 6.4 m):
// más lejos, el enemigo podría caer por un chunk que aún no se ha cargado.
const SPAWN_RADIUS: f32 = 5.0;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnTransition {
                exited: GameState::MainMenu,
                entered: GameState::InGame,
            },
            spawn_enemies,
        )
        .add_systems(OnEnter(GameState::MainMenu), despawn_enemies)
        .add_systems(Update, chase_player.run_if(in_state(GameState::InGame)));
    }
}

#[derive(Component)]
pub struct Enemy {
    pub speed: f32,
}

pub fn spawn_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world_seed: Res<WorldSeed>,
    world_kind: Res<WorldKind>,
) {
    let mut terrain_gen = TerrainGenerator::new(world_seed.0, *world_kind);

    // Mesh y material compartidos: con miles de enemigos, uno por entidad
    // multiplicaría la memoria y rompería el batching del render.
    let mesh = meshes.add(Cuboid::new(0.6, 1.8, 0.6));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.15, 0.75),
        ..default()
    });

    for i in 0..ENEMY_COUNT {
        let angle = i as f32 / ENEMY_COUNT as f32 * TAU;
        let (x, z) = (angle.cos() * SPAWN_RADIUS, angle.sin() * SPAWN_RADIUS);
        let ground_y = terrain_gen.biome_gen.generate_height(x, z);

        commands.spawn((
            Enemy { speed: 3.0 },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, ground_y + 2.0, z),
            RigidBody::Dynamic,
            Collider::capsule_y(0.6, 0.3), // 0.6*2 + 0.3*2 = 1.8 m, igual que el Cuboid
            Velocity::zero(),
            LockedAxes::ROTATION_LOCKED,
        ));
    }
}

pub fn despawn_enemies(mut commands: Commands, enemies: Query<Entity, With<Enemy>>) {
    for entity in &enemies {
        commands.entity(entity).despawn();
    }
}

pub fn chase_player(
    player: Query<&Transform, With<PlayerController>>,
    mut enemies: Query<(&Enemy, &Transform, &mut Velocity)>,
) {
    let Ok(player) = player.single() else {
        return;
    };

    for (enemy, transform, mut velocity) in &mut enemies {
        let horizontal = chase_velocity(transform.translation, player.translation, enemy.speed);
        // La Y la deja Rapier (gravedad): así el enemigo sigue el suelo.
        velocity.linvel.x = horizontal.x;
        velocity.linvel.z = horizontal.z;
    }
}

/// Velocidad horizontal hacia `to`. Sin radio de parada a propósito: pegado al
/// jugador sigue empujando y es la colisión la que lo frena, así nunca se queda
/// quieto esperando a que el jugador se aleje.
fn chase_velocity(from: Vec3, to: Vec3, speed: f32) -> Vec3 {
    let mut dir = to - from;
    dir.y = 0.0;
    dir.normalize_or_zero() * speed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_chasing_when_almost_touching_the_player() {
        let velocity = chase_velocity(Vec3::ZERO, Vec3::new(0.01, 0.0, 0.0), 3.0);
        assert!(velocity.abs_diff_eq(Vec3::new(3.0, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn ignores_height_difference() {
        let velocity = chase_velocity(Vec3::ZERO, Vec3::new(0.0, 10.0, 4.0), 3.0);
        assert!(velocity.abs_diff_eq(Vec3::new(0.0, 0.0, 3.0), 1e-5));
    }
}

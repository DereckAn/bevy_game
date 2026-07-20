//! Cielo dinámico: atmósfera física (Bevy 0.17), ciclo día/noche, niebla de
//! distancia y una capa de nubes FBM. Por ahora solo el "look" del cielo; la
//! precipitación (lluvia/nieve) se añadirá más adelante enganchada a un estado
//! de clima.

mod clouds;
mod time_of_day;

use bevy::prelude::*;

pub use clouds::CloudMaterial;
pub use time_of_day::TimeOfDay;

use clouds::{despawn_cloud_dome, spawn_cloud_dome, update_cloud_material};
use time_of_day::{setup_sky_camera, spawn_sun, update_day_night};

use crate::core::GameState;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CloudMaterial>::default())
            .init_resource::<TimeOfDay>()
            .add_systems(
                OnTransition {
                    exited: GameState::MainMenu,
                    entered: GameState::InGame,
                },
                (spawn_sun, spawn_cloud_dome),
            )
            .add_systems(OnEnter(GameState::MainMenu), despawn_cloud_dome)
            .add_systems(
                Update,
                (setup_sky_camera, update_day_night, update_cloud_material)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

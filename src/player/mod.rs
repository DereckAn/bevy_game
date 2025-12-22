pub mod components;
pub mod movement;
pub mod camera;
pub mod input;

use bevy::prelude::*;
pub use components::*;
use movement::*;
use camera::*;
use input::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (
                player_look,
                player_movement,
                cursor_grab,
            ));
    }
}
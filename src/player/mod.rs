pub mod camera;
pub mod components;
pub mod input;
pub mod movement;
pub mod reticle;

use bevy::prelude::*;
use camera::*;
pub use components::*;
use input::*;
use movement::*;
use reticle::*;

use crate::core::GameState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            // El jugador se crea SOLO al empezar partida (MainMenu → InGame), no al
            // arrancar ni al reanudar, para que no caiga por gravedad durante el menú
            // ni se reinicie su posición al despausar.
            .add_systems(
                OnTransition {
                    exited: GameState::MainMenu,
                    entered: GameState::InGame,
                },
                spawn_player,
            )
            // El jugador se elimina al llegar al menú principal desde cualquier
            // estado (InGame o Paused), evitando cámaras/jugadores duplicados.
            .add_systems(OnEnter(GameState::MainMenu), despawn_player)
            // Cursor: bloqueado siempre que estemos en juego, libre en los menús
            .add_systems(OnEnter(GameState::InGame), grab_cursor)
            .add_systems(OnEnter(GameState::Paused), release_cursor)
            .add_systems(OnEnter(GameState::MainMenu), release_cursor)
            // Crosshair: visible solo en juego (mismo ciclo de vida que el cursor)
            .add_systems(OnEnter(GameState::InGame), spawn_crosshair)
            .add_systems(OnEnter(GameState::Paused), despawn_crosshair)
            .add_systems(OnEnter(GameState::MainMenu), despawn_crosshair)
            // Movimiento, cámara y resaltado de voxel solo activos durante el juego
            .add_systems(
                Update,
                (
                    player_look,
                    player_movement,
                    cursor_grab_on_click,
                    highlight_aimed_voxel,
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

//! Modulo de interfaz de usuario (ui)
//!
//! Maneja todos los menus, botones y elementos visiales de la ui

pub mod menu;

pub use menu::*;

use crate::core::GameState::*;
use bevy::prelude::*;

// Plugin que maneja toda la UI del jugador
pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        // Por ahora esta
        app
            // REgistrar el sustema de estados
            .init_state::<crate::core::GameState>()
            // Sistema que corren al entrar al menu principal
            .add_systems(OnEnter(MainMenu), setup_main_menu)
            // Sistema que corren MIENTRAS estamos en el menu
            .add_systems(Update, menu_button_system.run_if(in_state(MainMenu)))
            // Sistemas que corren AL SALIR del menu
            .add_systems(OnExit(MainMenu), cleanup_main_menu);
    }
}

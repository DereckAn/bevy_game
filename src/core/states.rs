//! File donde se manejan los estados del juego 
//! 

use bevy::prelude::*;

/// Estados principales del juego 
/// 
/// este enum define las diferentes "Pantallas" o modos del juego.
/// Bevy esa esto para saber que sistemas ejecutar en cada momento.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    InGame, 
    Paused,
}
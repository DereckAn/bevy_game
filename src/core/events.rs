use bevy::prelude::*;

#[derive(Event)]
pub struct PlayerJumpEvent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct PlayerLandEvent {
    pub entity: Entity,
}
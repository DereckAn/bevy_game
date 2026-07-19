//! HUD del juego
//!
//! Muestra la herramienta equipada y el conteo de voxels recolectados.
//! Visible solo en `GameState::InGame` (se crea al entrar y se elimina al salir).

use crate::core::Inventory;
use crate::player::components::Player;
use crate::voxel::{Tool, VoxelType, VOXEL_TYPE_COUNT};
use bevy::prelude::*;

/// Marcador del contenedor raíz del HUD (para limpiarlo al salir del juego).
#[derive(Component)]
pub struct HudUI;

/// Marcador del texto del HUD que se reescribe cada frame.
#[derive(Component)]
pub struct HudText;

/// Crea el HUD al entrar a `InGame`.
pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            HudUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                HudText,
            ));
        });
}

/// Reescribe el texto del HUD con la herramienta y los conteos actuales.
pub fn update_hud(
    inventory: Res<Inventory>,
    tool_query: Query<&Tool, With<Player>>,
    mut text_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let tool_name = tool_query
        .single()
        .map(|tool| tool.tool_type.properties().name)
        .unwrap_or("None");

    let mut contents = format!("Tool: {}\n", tool_name);
    for id in 0..VOXEL_TYPE_COUNT {
        let count = inventory.0[id];
        if count == 0 {
            continue;
        }
        let name = VoxelType::from_u8(id as u8).properties().name;
        contents.push_str(&format!("{}: {}\n", name, count));
    }

    **text = contents;
}

/// Elimina el HUD al salir de `InGame` (menú/pausa).
pub fn cleanup_hud(mut commands: Commands, hud_query: Query<Entity, With<HudUI>>) {
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
}

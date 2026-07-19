use crate::player::components::Player;
use crate::voxel::{Tool, ToolType};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

/// Bloquea y oculta el cursor (modo juego).
pub fn grab_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Libera y muestra el cursor (menús: principal y pausa).
pub fn release_cursor(mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = cursor.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Re-bloquea el cursor al hacer click dentro del juego.
///
/// Solo corre en InGame; recupera el bloqueo si el SO liberó el cursor.
pub fn cursor_grab_on_click(
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Cambia la herramienta equipada con las teclas 1–4.
///
/// Reemplaza el `Tool` (durabilidad reiniciada al máximo); solo hay una
/// herramienta equipada a la vez.
pub fn switch_tool(
    keys: Res<ButtonInput<KeyCode>>,
    mut tool_query: Query<&mut Tool, With<Player>>,
) {
    let Ok(mut tool) = tool_query.single_mut() else {
        return;
    };

    let new_type = if keys.just_pressed(KeyCode::Digit1) {
        ToolType::Pickaxe
    } else if keys.just_pressed(KeyCode::Digit2) {
        ToolType::Axe
    } else if keys.just_pressed(KeyCode::Digit3) {
        ToolType::Shovel
    } else if keys.just_pressed(KeyCode::Digit4) {
        ToolType::Hoe
    } else {
        return;
    };

    if tool.tool_type != new_type {
        *tool = Tool::new(new_type);
        info!("Herramienta equipada: {:?}", new_type);
    }
}

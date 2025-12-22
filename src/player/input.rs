use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow, CursorOptions};

/// Captura/libera el cursor del mouse.
/// 
/// - Click izquierdo: Bloquea y oculta el cursor
/// - Escape: Libera y muestra el cursor
pub fn cursor_grab(
    mut cursor: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut cursor) = cursor.single_mut() else { return };

    if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}
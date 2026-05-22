//! Menú de pausa (ESC durante el juego)
//!
//! Permite reanudar y, a futuro, acceder a configuración, sonido y salir.

use crate::core::GameState;
use bevy::prelude::*;

/// Marcador para entidades del menú de pausa (para limpieza).
#[derive(Component)]
pub struct PauseMenuUI;

/// Acciones de los botones del menú de pausa.
#[derive(Component, Clone, Copy, Debug)]
pub enum PauseAction {
    Resume,
    Settings,
    Sound,
    QuitToMenu,
}

/// Alterna entre InGame y Paused al presionar ESC.
///
/// Corre en cualquier estado; ignora MainMenu.
pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    match state.get() {
        GameState::InGame => next_state.set(GameState::Paused),
        GameState::Paused => next_state.set(GameState::InGame),
        GameState::MainMenu => {}
    }
}

/// Construye la UI del menú de pausa al entrar en Paused.
pub fn setup_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            // Fondo semitransparente sobre el juego
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            PauseMenuUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 0.8)),
            ));

            create_pause_button(parent, "RESUME", PauseAction::Resume);
            create_pause_button(parent, "SETTINGS", PauseAction::Settings);
            create_pause_button(parent, "SOUND", PauseAction::Sound);
            create_pause_button(parent, "QUIT TO MENU", PauseAction::QuitToMenu);
        });
}

/// Elimina la UI del menú de pausa al salir de Paused.
pub fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenuUI>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn create_pause_button(parent: &mut ChildSpawnerCommands<'_>, text: &str, action: PauseAction) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(320.0),
                height: Val::Px(64.0),
                border: UiRect::all(Val::Px(3.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.8, 0.8, 0.8)),
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
            action,
            // Sin PauseMenuUI: es hijo del contenedor raíz y se limpia en cascada
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Maneja clicks y hover de los botones del menú de pausa.
pub fn pause_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &PauseAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, action) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => match action {
                PauseAction::Resume => next_state.set(GameState::InGame),
                PauseAction::QuitToMenu => next_state.set(GameState::MainMenu),
                PauseAction::Settings => {
                    info!("Configuración (no implementado aún)");
                }
                PauseAction::Sound => {
                    info!("Sonido (no implementado aún)");
                }
            },
            Interaction::Hovered => {
                *color = Color::srgb(0.3, 0.3, 0.3).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.2, 0.2, 0.2).into();
            }
        }
    }
}

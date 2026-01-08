//! Sistema de menu pricipal

use crate::core::GameState;
use bevy::prelude::*;

/// Componente marcador para entidades del menu pricipal
///
/// Usamos esto para identificar que entidades pertenecen al menu
/// Y poder eliminarlas cuando salgamos del menu
#[derive(Component)]
pub struct MainMenuUI;

/// Actions que pueden realizar los botones del menu
#[derive(Component, Clone, Copy, Debug)]
pub enum MenuAction {
    Play,
    Settings,
    Credits,
    Quit,
}

/// Sistema que se ejecuta al ENTRAR al estado MainMenu
///
/// Crea todo los botones y elementos visuales del menu
pub fn setup_main_menu(mut commands: Commands) {
    info!("Creando menuy principal ...");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),              // 100% del ancho de pantalla
                height: Val::Percent(100.0),             // 100% del alto de pantalla
                align_items: AlignItems::Center,         // Centrar verticalmente
                justify_content: JustifyContent::Center, // Centrar horizontalmente
                flex_direction: FlexDirection::Column,   // Botones en la columna
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)), // Dondo gris oscuro
            MainMenuUI, // Marcamos esta entidad para poder eliminarla despues
        ))
        .with_children(|parent| {
            // Boton PLAY
            create_menu_button(parent, "PLAY", MenuAction::Play);

            // Boton SETTINGS
            create_menu_button(parent, "SETTINGS", MenuAction::Settings);

            // Boton CREDITS
            create_menu_button(parent, "CREDITS", MenuAction::Credits);

            // Bototn QUIT
            create_menu_button(parent, "QUIT", MenuAction::Quit);
        });
}

/// Sistema que se ejecuta al SALIR del estado MainMenu
///
/// Elimina todos las entidades marcadas con MainMenuUI
pub fn cleanup_main_menu(mut commands: Commands, menu_query: Query<Entity, With<MainMenuUI>>) {
    info!("Limpiando menu principal ...");
    for entity in &menu_query {
        commands.entity(entity).despawn();
    }
}

/// Funcion helper para crear un boton del menu
///
/// Parametros:
/// - parent : El contenedor padre donde se anadiria el boton
/// - parent: El texto que mostrara el boton
/// - action: La accion asociada al boton
fn create_menu_button(parent: &mut ChildSpawnerCommands<'_>, text: &str, action: MenuAction) {
    parent
        .spawn((
            Button, // Componente que hace que sea clickable
            Node {
                width: Val::Px(300.0),             // Ancho de boton
                height: Val::Px(80.0),             // Alto del boton
                border: UiRect::all(Val::Px(4.0)), // Borde de 4 pixeles
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(Color::srgb(0.8, 0.8, 0.8)), // Color del borde (gris claro)
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),  // Fondo del boton (gris color)
            action,                                       // Guardamos la accion del boton
            MainMenuUI,                                   // marcamos para limpieza
        ))
        .with_children(|parent| {
            // Texto dentro del boton
            parent.spawn((
                Text::new(text),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}


/// Sistema que detecta interacciones con los btones del menu
/// 
/// Cambia colores cuando el mouse esta encima y ejecuta acciones al hacer ckick
pub fn menu_button_system(
    mut interaction_query: Query<
    (&Interaction, &mut BackgroundColor, &MenuAction),
    (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>
) {
    for (interaction, mut color, menu_action) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Usuario hizo CLICK en el boton
                info!("Botón presionado: {:?}", menu_action);

                match menu_action {
                    MenuAction::Play => {
                        info!("Iniciando juego ... ");
                        next_state.set(GameState::InGame);
                    }
                    MenuAction::Settings => {
                        info!("Abriendo configuracion .. (no implementado aun)");
                    }
                    MenuAction::Credits => {
                        info!("Mostrando creditos ... (No implementado aun)");
                    }
                    MenuAction::Quit => {
                        info!("SAliendo del juego");
                        // TODO: Cerrar el juego
                    }
                }
            }
            Interaction::Hovered => {
                // MOuse ENCIMA del boton
                *color = Color::srgb(0.3, 0.3, 0.3).into(); // Gris mas claro
            }
            Interaction::None => {
                // EStado NORMAL
                *color = Color::srgb(0.2, 0.2, 0.2).into();
            }
        }
    }
}
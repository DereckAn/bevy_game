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
    info!("Creando menú principal...");

    // Contenedor principal (toda la pantalla)
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row, // Horizontal
                justify_content: JustifyContent::FlexEnd, // Alinea a la DERECHA
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            MainMenuUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(30.0),
                         border: UiRect::all(Val::Px(4.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.0, 1.0, 0.8)),
                ))
                .with_children(|parent| {
                    create_menu_button(parent, "⚙️", MenuAction::Settings);
                });

            // ========================================
            // CONTENEDOR DERECHO (30% del ancho)
            // ========================================
            parent
                .spawn((
                    Node {
                        width: Val::Percent(40.0),               // 30% del ancho de pantalla
                        height: Val::Percent(100.0),             // 100% de altura
                        flex_direction: FlexDirection::Column,   // Botones en columna
                        justify_content: JustifyContent::Center, // Centrar verticalmente
                        align_items: AlignItems::Center,         // Centrar horizontalmente
                        border: UiRect::all(Val::Px(4.0)),
                        ..default()
                    },
                    BorderColor::all(Color::srgb(0.0, 1.0, 0.8)),
                ))
                .with_children(|parent| {
                    create_menu_button(parent, "PLAY", MenuAction::Play);
                    create_menu_button(parent, "SETTINGS", MenuAction::Settings);
                });
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
                width: Val::Percent(100.0),        // Ancho de boton
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
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
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

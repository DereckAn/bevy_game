//! HUD del juego
//!
//! - Barra de herramientas fija abajo a la izquierda (4 slots: pico, hacha,
//!   pala, azada). El slot equipado se resalta.
//! - Inventario en rejilla que aparece mientras se mantiene `Tab`: un slot por
//!   tipo de voxel recolectado, con su icono y la cantidad.
//!
//! Todo visible solo en `GameState::InGame` (se limpia al salir a pausa/menú).
//! Los iconos viven en `assets/icons/<nombre>.png`; los tipos sin icono propio
//! usan `default.png`.

use crate::core::Inventory;
use crate::player::components::Player;
use crate::voxel::{Tool, ToolType, VoxelType, VOXEL_TYPE_COUNT};
use bevy::prelude::*;

/// Herramientas mostradas en la barra, en el orden de las teclas 1–4.
const TOOLBAR: [ToolType; 4] = [
    ToolType::Pickaxe,
    ToolType::Axe,
    ToolType::Shovel,
    ToolType::Hoe,
];

const SLOT_PX: f32 = 64.0;
const HIGHLIGHT: Color = Color::srgb(1.0, 0.9, 0.2);
const SLOT_BORDER: Color = Color::srgb(0.4, 0.4, 0.4);

/// Marcador del contenedor de la barra de herramientas.
#[derive(Component)]
pub struct ToolbarUI;

/// Slot de la barra; guarda qué herramienta representa para resaltarlo.
#[derive(Component)]
pub struct ToolSlot(pub ToolType);

/// Marcador del contenedor del inventario (rejilla temporal con `Tab`).
#[derive(Component)]
pub struct InventoryUI;

/// Ruta del icono en `assets/`. `stem` vacío / desconocido cae en `default`.
fn icon_path(stem: &str) -> String {
    format!("icons/{}.png", stem)
}

/// Icono de cada herramienta.
fn tool_icon(tool: ToolType) -> &'static str {
    match tool {
        ToolType::Pickaxe => "pickaxe",
        ToolType::Axe => "axe",
        ToolType::Shovel => "shovel",
        ToolType::Hoe => "hoe",
        ToolType::None => "default",
    }
}

/// Icono de cada tipo de voxel; los que no tienen icono propio usan `default`.
fn voxel_icon(voxel: VoxelType) -> &'static str {
    match voxel {
        VoxelType::Dirt => "dirt",
        VoxelType::Grass => "grass",
        VoxelType::Stone => "stone",
        VoxelType::Metal => "metal",
        VoxelType::Sand => "sand",
        VoxelType::Wood | VoxelType::PineWood => "wood",
        VoxelType::Leaves | VoxelType::PineNeedles | VoxelType::SmallLeaves => "leaves",
        _ => "default",
    }
}

/// Crea la barra de herramientas al entrar a `InGame`.
pub fn setup_toolbar(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ToolbarUI,
        ))
        .with_children(|parent| {
            for tool in TOOLBAR {
                parent
                    .spawn((
                        Node {
                            width: Val::Px(SLOT_PX),
                            height: Val::Px(SLOT_PX),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor::all(SLOT_BORDER),
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        ToolSlot(tool),
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            ImageNode::new(asset_server.load(icon_path(tool_icon(tool)))),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                        ));
                    });
            }
        });
}

/// Resalta el slot de la herramienta equipada.
pub fn update_toolbar_highlight(
    tool_query: Query<&Tool, With<Player>>,
    mut slots: Query<(&ToolSlot, &mut BorderColor)>,
) {
    let Ok(tool) = tool_query.single() else {
        return;
    };
    for (slot, mut border) in &mut slots {
        *border = if slot.0 == tool.tool_type {
            BorderColor::all(HIGHLIGHT)
        } else {
            BorderColor::all(SLOT_BORDER)
        };
    }
}

/// Muestra/oculta el inventario según se mantenga `Tab`.
pub fn toggle_inventory(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    inventory: Res<Inventory>,
    open: Query<Entity, With<InventoryUI>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        spawn_inventory(&mut commands, &asset_server, &inventory);
    }
    if keys.just_released(KeyCode::Tab) {
        for entity in &open {
            commands.entity(entity).despawn();
        }
    }
}

/// Construye la rejilla del inventario (un slot por tipo con cantidad > 0).
fn spawn_inventory(commands: &mut Commands, asset_server: &AssetServer, inventory: &Inventory) {
    commands
        .spawn((
            // Capa a pantalla completa que centra el panel.
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            InventoryUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![RepeatedGridTrack::px(6, SLOT_PX)],
                        row_gap: Val::Px(6.0),
                        column_gap: Val::Px(6.0),
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                ))
                .with_children(|grid| {
                    for id in 0..VOXEL_TYPE_COUNT {
                        let count = inventory.0[id];
                        if count == 0 {
                            continue;
                        }
                        spawn_inventory_slot(
                            grid,
                            asset_server,
                            VoxelType::from_u8(id as u8),
                            count,
                        );
                    }
                });
        });
}

/// Un slot: icono del voxel con la cantidad en la esquina inferior derecha.
fn spawn_inventory_slot(
    grid: &mut ChildSpawnerCommands<'_>,
    asset_server: &AssetServer,
    voxel: VoxelType,
    count: u32,
) {
    grid.spawn((
        Node {
            width: Val::Px(SLOT_PX),
            height: Val::Px(SLOT_PX),
            ..default()
        },
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
    ))
    .with_children(|slot| {
        slot.spawn((
            ImageNode::new(asset_server.load(icon_path(voxel_icon(voxel)))),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ));
        slot.spawn((
            Text::new(count.to_string()),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(2.0),
                right: Val::Px(4.0),
                ..default()
            },
        ));
    });
}

/// Elimina barra e inventario al salir de `InGame`.
pub fn cleanup_hud(
    mut commands: Commands,
    hud_query: Query<Entity, Or<(With<ToolbarUI>, With<InventoryUI>)>>,
) {
    for entity in &hud_query {
        commands.entity(entity).despawn();
    }
}

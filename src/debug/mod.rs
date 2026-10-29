// ============================================================================
// DEBUG Y PROFILING TOOLS
// ============================================================================

use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::pbr::Atmosphere;
use bevy::post_process::bloom::Bloom;

use crate::sky::CloudDome;
use crate::voxel::{BaseChunk, LodChunk};

/// Plugin que configura todas las herramientas de debug y profiling
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app
            // Plugin para medir FPS y frame time
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            
            // Sistema para mostrar FPS en pantalla
            .add_systems(Startup, setup_fps_display)
            .add_systems(Update, update_fps_display)
            // Interruptores para aislar qué gasta GPU: comparar FPS mirando al mismo sitio
            .init_resource::<GpuToggles>()
            .add_systems(Update, (toggle_gpu_features, hide_new_lod_tiles));

        // Al cambiar a la ventana de Tracy, Bevy limitaría el juego a 60 Hz y
        // falsearía la traza
        #[cfg(feature = "tracy")]
        app.insert_resource(bevy::winit::WinitSettings::continuous());
    }
}

/// Componente para identificar el texto de FPS
#[derive(Component)]
struct FpsText;

/// Configura el display de FPS en pantalla
fn setup_fps_display(mut commands: Commands) {
    // Crear UI para mostrar FPS
    commands.spawn((
        Text::new("FPS: --"),                      // Texto inicial
        Node {                                     // Configuración de posición
            position_type: PositionType::Absolute, // Posición absoluta
            top: Val::Px(10.0),                    // 10 pixels desde arriba
            left: Val::Px(10.0),                   // 10 pixels desde la izquierda
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),    // Color verde
        FpsText,                                   // Marcador para identificar este texto
    ));
}

/// Actualiza el display de FPS cada frame
fn update_fps_display(
    diagnostics: Res<DiagnosticsStore>,           // Acceso a las métricas del sistema
    mut query: Query<&mut Text, With<FpsText>>,   // Query para encontrar el texto de FPS
    real_chunks: Query<(), With<BaseChunk>>,
    lod_chunks: Query<(), With<LodChunk>>,
    meshes: Query<&ViewVisibility, With<Mesh3d>>,
) {
    // Intentar obtener el texto de FPS
    if let Ok(mut text) = query.single_mut() {
        
        // Obtener FPS actual desde las métricas
        if let Some(fps) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)    // Obtener métrica de FPS
            .and_then(|fps| fps.smoothed())           // Obtener valor suavizado
        {
            // Actualizar el texto con FPS formateado
            **text = format!("FPS: {:.1}", fps);
        }
        
        // También mostrar frame time si está disponible
        if let Some(frame_time) = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) // Obtener métrica de frame time
            .and_then(|ft| ft.smoothed())                 // Obtener valor suavizado
        {
            // Agregar frame time al texto
            **text = format!("{}\nFrame Time: {:.2}ms", **text, frame_time * 1000.0);
        }

        // Cuántas mallas hay y cuántas pasan el culling: aproxima los draw calls
        let visible = meshes.iter().filter(|v| v.get()).count();
        **text = format!(
            "{}\nChunks: {} real, {} LOD\nMeshes: {} visible / {} total",
            **text,
            real_chunks.count(),
            lod_chunks.count(),
            visible,
            meshes.iter().count(),
        );
    }
}
/// Estado de los interruptores de diagnóstico de GPU (F3/F4/F5).
#[derive(Resource)]
struct GpuToggles {
    shadows: bool,
    lod_tiles: bool,
    sky_effects: bool,
}

impl Default for GpuToggles {
    fn default() -> Self {
        Self { shadows: true, lod_tiles: true, sky_effects: true }
    }
}

/// F3: sombras del sol. F4: teselas LOD. F5: atmósfera, bloom y nubes.
fn toggle_gpu_features(
    keys: Res<ButtonInput<KeyCode>>,
    mut toggles: ResMut<GpuToggles>,
    mut commands: Commands,
    mut lights: Query<&mut DirectionalLight>,
    mut lod_tiles: Query<&mut Visibility, (With<LodChunk>, Without<CloudDome>)>,
    mut clouds: Query<&mut Visibility, (With<CloudDome>, Without<LodChunk>)>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    if keys.just_pressed(KeyCode::F3) {
        toggles.shadows = !toggles.shadows;
        for mut light in &mut lights {
            light.shadows_enabled = toggles.shadows;
        }
        info!("Sombras: {}", toggles.shadows);
    }
    if keys.just_pressed(KeyCode::F4) {
        toggles.lod_tiles = !toggles.lod_tiles;
        for mut visibility in &mut lod_tiles {
            *visibility = visibility_for(toggles.lod_tiles);
        }
        info!("Teselas LOD: {}", toggles.lod_tiles);
    }
    if keys.just_pressed(KeyCode::F5) {
        toggles.sky_effects = !toggles.sky_effects;
        for mut visibility in &mut clouds {
            *visibility = visibility_for(toggles.sky_effects);
        }
        for camera in &cameras {
            if toggles.sky_effects {
                commands.entity(camera).insert((Atmosphere::EARTH, Bloom::NATURAL));
            } else {
                commands.entity(camera).remove::<(Atmosphere, Bloom)>();
            }
        }
        info!("Atmósfera/bloom/nubes: {}", toggles.sky_effects);
    }
}

/// Las teselas LOD que se crean con F4 activo nacen visibles; se ocultan aquí.
fn hide_new_lod_tiles(toggles: Res<GpuToggles>, mut new_tiles: Query<&mut Visibility, Added<LodChunk>>) {
    if !toggles.lod_tiles {
        for mut visibility in &mut new_tiles {
            *visibility = Visibility::Hidden;
        }
    }
}

fn visibility_for(is_on: bool) -> Visibility {
    if is_on { Visibility::Inherited } else { Visibility::Hidden }
}

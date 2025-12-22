// ============================================================================
// DEBUG Y PROFILING TOOLS
// ============================================================================

use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

/// Plugin que configura todas las herramientas de debug y profiling
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app
            // Plugin para medir FPS y frame time
            .add_plugins(FrameTimeDiagnosticsPlugin::default())
            
            // Sistema para mostrar FPS en pantalla
            .add_systems(Startup, setup_fps_display)
            .add_systems(Update, update_fps_display);
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
    }
}
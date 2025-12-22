// ============================================================================
// DECLARACIÓN DE MÓDULOS
// ============================================================================

pub mod components;                                // Declara el submódulo components (src/physics/components.rs)

// ============================================================================
// IMPORTS - TRAER CÓDIGO DE OTRAS LIBRERÍAS
// ============================================================================

use bevy::prelude::*;                              // Tipos básicos de Bevy (App, Plugin)
use bevy_rapier3d::prelude::*;                     // Plugins de física de Rapier
pub use components::*;                             // Re-exporta todo desde components para facilitar el uso

// ============================================================================
// PLUGIN DE FÍSICA
// ============================================================================

/// Plugin que configura el sistema de física del juego usando Rapier3D
pub struct PhysicsPlugin;                          // Estructura vacía que implementa Plugin

/// Implementación del trait Plugin para PhysicsPlugin
impl Plugin for PhysicsPlugin {                    // Plugin es un trait de Bevy que define cómo configurar sistemas
    fn build(&self, app: &mut App) {               // Función requerida que configura la aplicación
        app                                        // Encadena configuraciones en la aplicación
            .add_plugins(RapierPhysicsPlugin::<NoUserData>::default()) // Añade el plugin principal de física Rapier
            // Explicación de RapierPhysicsPlugin:
            // - RapierPhysicsPlugin: plugin que integra Rapier con Bevy
            // - <NoUserData>: tipo genérico que indica que no usamos datos personalizados en colisiones
            // - ::default(): usa configuración por defecto (gravedad, timestep, etc.)
            
            .add_plugins(RapierDebugRenderPlugin::default()); // Añade plugin de debug visual
            // Explicación de RapierDebugRenderPlugin:
            // - Dibuja wireframes de colisionadores para debug
            // - Muestra líneas verdes/rojas alrededor de objetos físicos
            // - Útil para desarrollo, se puede quitar en producción
    }
}
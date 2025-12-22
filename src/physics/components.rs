// ============================================================================
// IMPORTS - TRAER CÓDIGO DE OTRAS LIBRERÍAS
// ============================================================================

use bevy::prelude::*;                              // Tipos básicos de Bevy (Mesh)
use bevy_rapier3d::prelude::*;                     // Tipos de física de Rapier

// ============================================================================
// RE-EXPORTACIÓN DE COMPONENTES
// ============================================================================

// Re-exportamos los componentes de Rapier para facilitar el uso desde otros módulos
pub use bevy_rapier3d::prelude::{                 // pub use = re-exportar públicamente
    RigidBody,                                     // Tipo de cuerpo físico (Dynamic, Fixed, Kinematic)
    Collider,                                      // Forma de colisión (capsule, box, mesh, etc.)
    Velocity,                                      // Velocidad lineal y angular
    LockedAxes,                                    // Ejes bloqueados (evitar rotación/movimiento en ciertos ejes)
    Friction,                                      // Coeficiente de fricción
    Restitution,                                   // Coeficiente de rebote/elasticidad
    AdditionalMassProperties                       // Propiedades de masa adicionales
};
// Esto permite usar estos tipos como physics::RigidBody en lugar de bevy_rapier3d::prelude::RigidBody

// ============================================================================
// FUNCIONES HELPER
// ============================================================================

/// Función helper para crear colisiones de terreno a partir de una malla 3D
/// 
/// Convierte una malla de Bevy en un colisionador de Rapier para física de terreno.
/// Usa TriMesh que es preciso pero más costoso computacionalmente.
pub fn create_terrain_collider(mesh: &Mesh) -> Collider {
    Collider::from_bevy_mesh(                      // Crea colisionador desde malla de Bevy
        mesh,                                      // Referencia a la malla 3D
        &ComputedColliderShape::TriMesh(           // Tipo de colisionador: malla de triángulos
            TriMeshFlags::default()                // Flags por defecto para la malla de triángulos
        )
    ).unwrap()                                     // unwrap() = "confío en que no falle, si falla crashea el programa"
    
    // Explicación de los tipos:
    // - Collider::from_bevy_mesh: función que convierte Mesh de Bevy a Collider de Rapier
    // - ComputedColliderShape::TriMesh: tipo de colisionador basado en triángulos
    //   * Más preciso que formas primitivas (box, sphere)
    //   * Más costoso computacionalmente
    //   * Ideal para terreno complejo
    // - TriMeshFlags: configuración adicional para la malla (por defecto está bien)
    // - unwrap(): manejo de errores agresivo, crashea si falla la conversión
}
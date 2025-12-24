// ============================================================================
// IMPORTS - TRAER CÓDIGO DE OTRAS LIBRERÍAS
// ============================================================================

use bevy::prelude::*;          // Importa tipos básicos de Bevy (Component, Commands, Transform, etc.)
use bevy_rapier3d::prelude::*;
use crate::voxel::{Tool, ToolType}; // Importa tipos de física de Rapier (RigidBody, Collider, Velocity, etc.)

// ============================================================================
// DEFINICIÓN DE COMPONENTES
// ============================================================================

/// Componente que representa al jugador con sus propiedades de movimiento y cámara
#[derive(Component)]           // Macro que hace que Player sea un componente de Bevy ECS
pub struct Player {            // Estructura pública que define las propiedades del jugador
    pub speed: f32,            // Velocidad de movimiento en unidades por segundo (público = accesible desde otros módulos)
    pub sensitivity: f32,      // Sensibilidad del mouse en radianes por pixel
    pub pitch: f32,            // Rotación vertical actual en radianes (mirar arriba/abajo)
    pub yaw: f32,              // Rotación horizontal actual en radianes (mirar izquierda/derecha)
    pub jump_force: f32,       // Fuerza del salto en unidades por segundo
}

/// Implementación del trait Default para Player
impl Default for Player {     // Define valores por defecto para cuando se crea un Player::default()
    fn default() -> Self {    // Función que retorna una instancia de Player con valores iniciales
        Self {                // Self se refiere a Player en este contexto
            speed: 5.0,       // 5 unidades por segundo de velocidad
            sensitivity: 0.002, // Sensibilidad baja del mouse (0.002 radianes por pixel)
            pitch: 0.0,       // Mirando al horizonte (sin inclinación vertical)
            yaw: 0.0,         // Mirando hacia el frente (sin rotación horizontal)
            jump_force: 5.0,  // Fuerza de salto moderada
        }
    }
}

/// Componente marcador para identificar entidades que son controladores de jugador
#[derive(Component)]          // Macro que hace que PlayerController sea un componente
pub struct PlayerController;  // Estructura vacía usada solo como "etiqueta" o "marcador"

// ============================================================================
// FUNCIÓN DE CREACIÓN DEL JUGADOR
// ============================================================================

/// Crea la entidad del jugador con cámara 3D y física.
/// 
/// Esta función se ejecuta al inicio del juego y crea una entidad completa
/// del jugador con todos los componentes necesarios para movimiento, cámara y física.
pub fn spawn_player(mut commands: Commands) { // Recibe Commands mutable para crear entidades
    commands.spawn((           // Crea una nueva entidad con los siguientes componentes:
        
        // ====================================================================
        // COMPONENTES PERSONALIZADOS
        // ====================================================================
        Player::default(),     // Nuestro componente Player con valores por defecto
        PlayerController,      // Marcador para identificar esta entidad como jugador
        Tool::new(ToolType::Shovel), // Agregar tool al jugador
        
        // ====================================================================
        // COMPONENTES DE BEVY
        // ====================================================================
        Camera3d::default(),   // Cámara 3D con configuración por defecto
        Transform::from_xyz(0.0, 10.0, 0.0), // Posición inicial: X=0, Y=10 (alto), Z=0
        
        // ====================================================================
        // COMPONENTES DE FÍSICA (RAPIER)
        // ====================================================================
        RigidBody::Dynamic,    // Cuerpo rígido dinámico (afectado por fuerzas y gravedad)
        Collider::capsule_y(0.9, 0.3), // Colisionador en forma de cápsula:
                              //   - 0.9 = mitad de altura (total 1.8m)
                              //   - 0.3 = radio (0.6m de diámetro)
        Velocity::zero(),     // Velocidad inicial en cero (parado)
        LockedAxes::ROTATION_LOCKED, // Bloquea rotación por física (evita que el jugador ruede)
        Friction::coefficient(0.7),  // Coeficiente de fricción 0.7 (realista para caminar)
        Restitution::coefficient(0.0), // Sin rebote (coeficiente 0.0 = no elástico)
        AdditionalMassProperties::Mass(70.0), // Masa de 70 kilogramos (peso humano promedio)
    ));
}
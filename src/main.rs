//! # Punto de entrada principal del juego voxels
//!
//! Este módulo inicializa la aplicación Bevy, configura los plugins necesarios
//! y genera la escena inicial con chunks de terreno e iluminación.

// ============================================================================
// DECLARACIÓN DE MÓDULOS
// ============================================================================

mod core; // Declara el módulo 'core' (busca src/core/mod.rs)
mod debug;
mod physics; // Declara el módulo 'physics' (busca src/physics/mod.rs)
mod player; // Declara el módulo 'player' (busca src/player/mod.rs)
mod voxel; // Declara el módulo 'voxel' (busca src/voxel/mod.rs) // Declara el módulo 'debug' (busca src/debug/mod.rs)

// ============================================================================
// IMPORTS (TRAER CÓDIGO DE OTROS MÓDULOS)
// ============================================================================
use std::collections::HashMap;

use bevy::{prelude::*, window::{CursorGrabMode, CursorOptions}}; // Importa todo lo común de Bevy (App, Commands, etc.)
use core::GameSettings; // Importa GameSettings desde nuestro módulo core
use debug::DebugPlugin;
use physics::{PhysicsPlugin, RigidBody, create_terrain_collider}; // Importa componentes de física
use player::PlayerPlugin; // Importa PlayerPlugin desde nuestro módulo player
use voxel::{
    Chunk, ChunkMap, generate_simple_mesh, start_voxel_breaking_system, update_voxel_breaking_system,update_drops_system, collect_drop_system, clean_old_drops_system
}; // Importa Chunk y generate_simple_mesh desde nuestro módulo voxel // Importa DebugPlugin para métricas de rendimiento

// ============================================================================
// FUNCIÓN PRINCIPAL
// ============================================================================

// Punto de entrada de la aplicación
// Configura bevy con plugins por defecto, el plugin del jugador y el sistema de setup
fn main() {
    // Función principal que ejecuta Rust al iniciar
    App::new() // Crea una nueva aplicación de Bevy
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Voxel game".to_string(),
                ..default()
            }),
            // Configuración del cursor como componente separado
            primary_cursor_options: Some(CursorOptions {
                visible: false,
                grab_mode: CursorGrabMode::Locked,
                ..default()
            }),
            ..default()
        })) // Añade plugins básicos (ventana, input, render, etc.)
        .add_plugins(PhysicsPlugin) // Añade nuestro plugin de física (Rapier)
        .add_plugins(PlayerPlugin) // Añade nuestro plugin del jugador (movimiento, cámara)
        .add_plugins(DebugPlugin) // Añade herramientas de debug y profiling
        .insert_resource(GameSettings::new()) // Inserta recurso global GameSettings en el mundo
        .insert_resource(ChunkMap {
            chunks: HashMap::new(),
        })
        .add_systems(Startup, setup) // Registra la función 'setup' para ejecutar al inicio
        .add_systems(Update, (
            start_voxel_breaking_system,
            update_voxel_breaking_system,
            update_drops_system,
            collect_drop_system,
            clean_old_drops_system,
        ).chain())
        .run(); // Inicia el loop principal del juego
}

// ============================================================================
// SISTEMA DE INICIALIZACIÓN
// ============================================================================

/// Sistema de inicialización que genera la escena.
///
/// Crea una grilla de 11x11 chunks centrada en el origen y añade iluminación
///
/// # Parámetros
/// - `commands`: Comandos para crear entidades y recursos en el mundo.
/// - `meshes`: Recursos para almacenar y gestionar las mallas 3D.
/// - `materials`: Recursos para almacenar y gestionar los materiales estándar.
fn setup(
    mut commands: Commands, // Sistema de comandos para crear/modificar entidades
    mut meshes: ResMut<Assets<Mesh>>, // Recurso mutable para gestionar mallas 3D
    mut materials: ResMut<Assets<StandardMaterial>>, // Recurso mutable para gestionar materiales
    mut chunk_map: ResMut<ChunkMap>,
) {
    // ========================================================================
    // GENERACIÓN DE TERRENO
    // ========================================================================

    // Generar varios chunks en una grilla de 11x11 (de -5 a +5 en X y Z)
    for cx in -5..=5 {
        // Loop de X: -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5
        for cz in -5..=5 {
            // Loop de Z: -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5
            let chunk = Chunk::new(IVec2::new(cx, cz)); // Crea un nuevo chunk en posición (cx, 0, cz)
            
            // Para la inicialización, usamos la función simple sin neighbors
            // porque aún no tenemos todos los chunks creados
            let mesh = generate_simple_mesh(&chunk); // Genera la malla 3D del chunk

            // Guarda la posicion antes de moverla.
            let chunk_position = chunk.position;

            // Crear entidad del chunk con todos sus componentes
            let chunk_entity = commands.spawn((
                // Crea una nueva entidad con los siguientes componentes:
                Mesh3d(meshes.add(mesh.clone())), // Componente de malla 3D (clona porque también lo usa física)
                MeshMaterial3d(materials.add(StandardMaterial {
                    // Componente de material 3D
                    base_color: Color::srgb(0.3, 0.5, 0.3), // Color verde (R=0.3, G=0.5, B=0.3)
                    cull_mode: None, // No descartar caras (mostrar ambos lados)
                    ..default()      // Usar valores por defecto para el resto
                })),
                Transform::default(), // Componente de transformación (posición, rotación, escala)
                chunk,                // Nuestro componente Chunk personalizado
                // Física del terreno
                RigidBody::Fixed,               // Cuerpo rígido fijo (no se mueve)
                create_terrain_collider(&mesh), // Colisionador generado desde la malla
            )).id();

            // Agrega al chunkMap
            chunk_map.chunks.insert(chunk_position, chunk_entity);
        }
    }

    // ========================================================================
    // ILUMINACIÓN
    // ========================================================================

    // Luz direccional (simula el sol)
    commands.spawn((
        // Crea entidad de luz
        DirectionalLight {
            // Componente de luz direccional
            illuminance: 15000.0,  // Intensidad de la luz en lux
            shadows_enabled: true, // Habilitar sombras
            ..default()            // Valores por defecto para el resto
        },
        Transform::from_xyz(4.0, 10.0, 4.0) // Posición de la luz en (4, 10, 4)
            .looking_at(Vec3::ZERO, Vec3::Y), // Apunta hacia el origen (0,0,0), con Y como "arriba"
    ));
}

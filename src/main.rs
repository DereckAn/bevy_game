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
    BaseChunk, ChunkMap3D, DynamicChunkSystem, generate_mesh, start_voxel_breaking_system, update_voxel_breaking_system, update_drops_system, collect_drop_system, clean_old_drops_system, update_drop_ground_detection_system
}; // Importa tipos del nuevo sistema de chunks dinámicos

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
        .insert_resource(DynamicChunkSystem::new()) // Sistema de chunks dinámicos
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
    mut chunk_system: ResMut<DynamicChunkSystem>,
) {
    // ========================================================================
    // GENERACIÓN DE TERRENO 3D
    // ========================================================================

    println!("Generando chunks 3D dinámicos...");

    // ========================================================================
    // GENERACIÓN DE TERRENO 3D CON RUIDO
    // ========================================================================

    println!("Generando terreno procedural con ruido Perlin...");

    // Generar chunks en una grilla similar al sistema anterior
    // Pero ahora con chunks 3D de 32³ en lugar de columnares
    for cx in -3..=3 {  // 7x7 chunks horizontales (como antes era 11x11 pero más pequeño)
        for cy in 0..=3 {   // 4 capas verticales (32*4 = 128 voxels de altura)
            for cz in -3..=3 {
                let chunk_pos = IVec3::new(cx, cy, cz);
                let chunk = chunk_system.get_or_create_chunk(chunk_pos);
                
                // Generar mesh para el chunk
                let mesh = generate_mesh(chunk);
                
                // Solo crear entidad si el mesh tiene geometría
                let vertex_count = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    .map(|attr| attr.len())
                    .unwrap_or(0);
                
                if vertex_count > 0 {
                    println!("Chunk {:?} generado con {} vértices", chunk_pos, vertex_count);
                    
                    // Crear entidad del chunk con mesh visible
                    commands.spawn((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.4, 0.7, 0.3), // Verde pasto
                            metallic: 0.0,
                            perceptual_roughness: 0.8,
                            ..default()
                        })),
                        Transform::default(),
                        // TODO: Agregar física cuando sea necesario
                        // RigidBody::Fixed,
                        // Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::TriMesh).unwrap(),
                    ));
                } else {
                    // Es normal que chunks altos estén vacíos (solo aire)
                    if cy <= 1 {
                        println!("Chunk {:?} está vacío (puede ser normal si está sobre el terreno)", chunk_pos);
                    }
                }
            }
        }
    }

    // ========================================================================
    // ILUMINACIÓN Y CÁMARA
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

    // Cámara principal - posicionada para ver el terreno procedural
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(8.0, 6.0, 8.0) // Posición elevada para ver el terreno
            .looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y), // Mirar hacia el centro del terreno
    ));

    println!("Escena inicializada - Cámara posicionada en (8, 6, 8) mirando hacia (0, 1, 0)");
}

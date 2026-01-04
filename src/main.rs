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

use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};
use core::GameSettings; // Importa GameSettings desde nuestro módulo core
use debug::DebugPlugin;
use physics::{PhysicsPlugin, RigidBody, create_terrain_collider}; // Importa componentes de física
use player::PlayerPlugin; // Importa PlayerPlugin desde nuestro módulo player
use voxel::{
    BaseChunk, BoundingBox, ChunkLOD, ChunkLoadQueue, ChunkMap, ChunkOctree,
    complete_chunk_generation_system, convert_lod_to_real_system, convert_real_to_lod_system,
    greedy_mesh_basechunk_simple, load_chunks_system, start_voxel_breaking_system,
    unload_chunks_system, update_chunk_load_queue, update_chunk_lod_system,
    update_chunk_transitions_system, update_voxel_breaking_system,
};

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
        .insert_resource(ChunkLoadQueue::default())
        .insert_resource(ChunkOctree::new(BoundingBox::new(
            IVec3::new(-200, -10, -200),
            IVec3::new(200, 10, 200),
        )))
        .add_systems(Startup, setup) // Registra la función 'setup' para ejecutar al inicio
        .add_systems(
            Update,
            (
                start_voxel_breaking_system,
                update_voxel_breaking_system,
                update_chunk_lod_system,
                // Sistemas de carga dinámica de chunks (async)
                update_chunk_load_queue,
                load_chunks_system,
                complete_chunk_generation_system,
                unload_chunks_system,
                // Sistemas de transiciones Real ↔ LOD
                update_chunk_transitions_system,
                convert_lod_to_real_system,
                convert_real_to_lod_system,
            )
                .chain(),
        )
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
    mut octree: ResMut<ChunkOctree>,
) {
    // ========================================================================
    // INICIALIZAR CACHÉ DE CHUNKS (DESHABILITADO TEMPORALMENTE)
    // ========================================================================

    // Caché deshabilitado para mejor rendimiento inicial
    // if let Err(e) = init_cache_dir() {
    //     warn!("Failed to initialize cache directory: {}", e);
    // } else {
    //     match get_cache_stats() {
    //         Ok(stats) => {
    //             info!("Cache initialized: {} chunks cached ({:.2} MB)",
    //                 stats.chunk_count, stats.total_size_mb());
    //         }
    //         Err(e) => warn!("Failed to get cache stats: {}", e),
    //     }
    // }

    // ========================================================================
    // GENERACIÓN DE TERRENO INICIAL
    // ========================================================================

    // Generar solo chunks iniciales alrededor del spawn (radio de 5 chunks)
    // El sistema de carga dinámica generará el resto
    let initial_radius = 5;
    let y_min = -1; // Chunks bajo tierra
    let y_max = 3; // Chunks en el aire (para montañas)

    let mut temp_chunks: HashMap<IVec3, BaseChunk> = HashMap::new();

    for cx in -initial_radius..=initial_radius {
        for cz in -initial_radius..=initial_radius {
            // Solo generar en un círculo, no un cuadrado
            if cx * cx + cz * cz <= initial_radius * initial_radius {
                // Generar chunks en múltiples niveles verticales
                for cy in y_min..=y_max {
                    let base_chunk = BaseChunk::new(IVec3::new(cx, cy, cz));
                    temp_chunks.insert(base_chunk.position, base_chunk);
                }
            }
        }
    }

    info!("Generating {} initial chunks...", temp_chunks.len());

    // Crear entidades con meshes
    for (chunk_pos, base_chunk) in temp_chunks.into_iter() {
        let mesh = greedy_mesh_basechunk_simple(&base_chunk);

        // Solo crear entidad si el mesh tiene vértices
        if mesh.count_vertices() > 0 {
            let chunk_entity = commands
                .spawn((
                    Mesh3d(meshes.add(mesh.clone())),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: ChunkLOD::Ultra.debug_color(),
                        cull_mode: None,
                        ..default()
                    })),
                    Transform::default(),
                    base_chunk,
                    ChunkLOD::Ultra,
                    RigidBody::Fixed,
                    create_terrain_collider(&mesh),
                ))
                .id();

            chunk_map.chunks.insert(chunk_pos, chunk_entity);
            octree.insert(chunk_pos);
        } else {
            // Chunk vacío, crear sin collider
            let chunk_entity = commands
                .spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: ChunkLOD::Ultra.debug_color(),
                        cull_mode: None,
                        ..default()
                    })),
                    Transform::default(),
                    base_chunk,
                    ChunkLOD::Ultra,
                ))
                .id();

            chunk_map.chunks.insert(chunk_pos, chunk_entity);
            octree.insert(chunk_pos);
        }
    }

    let stats = octree.stats();
    info!("Initial chunks generated! Octree stats: {:?}", stats);

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

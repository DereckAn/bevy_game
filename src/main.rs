//! # Punto de entrada principal del juego voxels
//!
//! Este módulo inicializa la aplicación Bevy, configura los plugins necesarios
//! y genera la escena inicial con chunks de terreno e iluminación.

// ============================================================================
// DECLARACIÓN DE MÓDULOS
// ============================================================================

mod core; // Declara el módulo 'core' (busca src/core/mod.rs)
mod debug;
mod ui;
mod physics; // Declara el módulo 'physics' (busca src/physics/mod.rs)
mod player; // Declara el módulo 'player' (busca src/player/mod.rs)
mod voxel; // Declara el módulo 'voxel' (busca src/voxel/mod.rs) // Declara el módulo 'debug' (busca src/debug/mod.rs)
mod vegetation; // Declara el módulo 'vegetation' (busca src/vegetation/mod.rs)

// ============================================================================
// IMPORTS (TRAER CÓDIGO DE OTROS MÓDULOS)
// ============================================================================
use std::collections::HashMap;
use ui::UIPlugin;
use bevy::prelude::*;
use core::{GameSettings, WorldSeed}; // Importa recursos globales desde nuestro módulo core
use debug::DebugPlugin;
use physics::{PhysicsPlugin, RigidBody, create_terrain_collider}; // Importa componentes de física
use player::PlayerPlugin; // Importa PlayerPlugin desde nuestro módulo player
use voxel::{
    BaseChunk, ChunkLOD, ChunkLoadQueue, ChunkMap, ChunkMaterials, SpatialHashGrid,
    complete_chunk_generation_system, convert_lod_to_real_system, convert_real_to_lod_system,
    greedy_mesh_basechunk_simple, load_chunks_system, remesh_dirty_chunks_system,
    start_voxel_breaking_system, teardown_world, unload_chunks_system, update_chunk_load_queue,
    update_chunk_lod_system, update_chunk_transitions_system, update_frustum_culling,
    update_voxel_breaking_system, TerrainGenerator, VoxelDiffs,
};

use crate::core::GameState;

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
            // El cursor arranca visible (estamos en MainMenu); el estado
            // del juego lo bloquea/libera en player/input.rs
            ..default()
        })) // Añade plugins básicos (ventana, input, render, etc.)
        .add_plugins(PhysicsPlugin) // Añade nuestro plugin de física (Rapier)
        .add_plugins(UIPlugin) // Anade el plugin de ui 
        .add_plugins(PlayerPlugin) // Añade nuestro plugin del jugador (movimiento, cámara)
        .add_plugins(DebugPlugin) // Añade herramientas de debug y profiling
        .insert_resource(GameSettings::new()) // Inserta recurso global GameSettings en el mundo
        .insert_resource(WorldSeed::random()) // Semilla aleatoria: mapa distinto cada arranque
        .insert_resource(ChunkMap {
            chunks: HashMap::new(),
        })
        .insert_resource(ChunkLoadQueue::default())
        .insert_resource(SpatialHashGrid::default())
        .init_resource::<VoxelDiffs>()
        .init_resource::<ChunkMaterials>()
        // El terreno se genera solo al empezar partida, no al reanudar desde pausa
        .add_systems(
            OnTransition {
                exited: GameState::MainMenu,
                entered: GameState::InGame,
            },
            setup,
        )
        // Al volver al menú se destruye el mundo para que el próximo Play arranque limpio
        .add_systems(OnEnter(GameState::MainMenu), teardown_world)
        .add_systems(
            Update,
            (
                start_voxel_breaking_system,
                update_voxel_breaking_system,
                remesh_dirty_chunks_system,
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
                // Optimización: Frustum culling
                update_frustum_culling,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
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
    mut materials: ResMut<Assets<StandardMaterial>>, // Para el material de la caja de referencia
    chunk_materials: Res<ChunkMaterials>, // Materiales compartidos de chunks
    mut chunk_map: ResMut<ChunkMap>,
    world_seed: Res<WorldSeed>,
) {
    // ========================================================================
    // GENERACIÓN DE TERRENO INICIAL
    // ========================================================================

    // Generar solo el área mínima bajo el spawn (radio de 2 chunks) para que el
    // jugador tenga suelo al caer; el loader async rellena el resto sin congelar
    // el arranque. (#10: antes radio 5 ≈ 390 chunks síncronos al pulsar Play.)
    let initial_radius = 2;
    let y_min = -1; // Chunks bajo tierra
    let y_max = 3; // Chunks en el aire (para montañas)

    let mut temp_chunks: HashMap<IVec3, BaseChunk> = HashMap::new();

    for cx in -initial_radius..=initial_radius {
        for cz in -initial_radius..=initial_radius {
            // Solo generar en un círculo, no un cuadrado
            if cx * cx + cz * cz <= initial_radius * initial_radius {
                // Generar chunks en múltiples niveles verticales
                for cy in y_min..=y_max {
                    let base_chunk = BaseChunk::new(IVec3::new(cx, cy, cz), world_seed.0);
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
                    MeshMaterial3d(chunk_materials.real_handle(ChunkLOD::Ultra)),
                    Transform::default(),
                    base_chunk,
                    ChunkLOD::Ultra,
                    RigidBody::Fixed,
                    create_terrain_collider(&mesh),
                ))
                .id();

            chunk_map.chunks.insert(chunk_pos, chunk_entity);
        } else {
            // Chunk vacío, crear sin collider
            let chunk_entity = commands
                .spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(chunk_materials.real_handle(ChunkLOD::Ultra)),
                    Transform::default(),
                    base_chunk,
                    ChunkLOD::Ultra,
                ))
                .id();

            chunk_map.chunks.insert(chunk_pos, chunk_entity);
        }
    }

    info!("Initial chunks generated!");

    // ========================================================================
    // CAJA DE REFERENCIA DE ESCALA (~1.9 m, tamaño de una persona)
    // ========================================================================
    // Marcador visual junto al spawn para tener una referencia del tamaño del
    // jugador. Solo visual (sin colisión): se puede atravesar.
    {
        let (ref_x, ref_z) = (1.0_f32, 1.0_f32); // ~1.4 m del origen
        let height = 1.9; // 190 cm

        // Apoyar la base de la caja sobre el terreno: centro = suelo + media altura
        let mut terrain_gen = TerrainGenerator::new(world_seed.0);
        let ground_y = terrain_gen.biome_gen.generate_height(ref_x, ref_z);

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.5, height, 0.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.15, 0.15), // rojo, para destacar
                ..default()
            })),
            Transform::from_xyz(ref_x, ground_y + height / 2.0, ref_z),
        ));
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

//! Sistema de chunks dinamicos como minecraft o lay of the land
//! Chunks base de 32³ que se combinan segun LOD

use bevy::prelude::*;
use std::time::Instant;
use std::collections::HashMap;
use crate::core::BASE_CHUNK_SIZE;
use crate::voxel::{ChunkLOD, VoxelType};


/// Chunk base de 32³ (use heap para evitar stack overflow)
#[derive(Component)]
pub struct BaseChunk {
    pub densities: Box<[[[f32; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]>,
    pub voxel_types: Box<[[[VoxelType; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]>,
    pub position: IVec3,
    pub last_accessed: Instant
}

/// Chunk combinado (multiples basechunks unidos)
#[derive(Component)]
pub struct MergedChunk {
    pub base_chunks: Vec<IVec3>, // Posiciones de chunks base incluidos
    pub effective_size: usize, // 64, 128, 256, 512
    pub position: IVec3, // Posicion del chunk combinado
    pub last_updated: Instant
}

/// Sistema principal de chunks dinamicos
#[derive(Resource)]
pub struct DynamicChunkSystem {
    pub base_chunks: HashMap<IVec3, Entity>,
    pub merged_chunks: HashMap<IVec3, Entity>,
    pub player_position: Vec3,
    pub merge_scheduler: ChunkMergeScheduler,
}

/// Programador de operaciones de merge/split
#[derive(Default)]
pub struct ChunkMergeScheduler {
    pub merge_queue: Vec<MergeTask>,
    pub split_queue: Vec<SplitTask>,
}

/// Tarea para combinar chunks
pub struct MergeTask {
    pub chunks_to_merge: Vec<IVec3>,
    pub target_lod: ChunkLOD,
    pub priority: f32
}

/// Tarea para dividir chunks
pub struct SplitTask {
    pub chunk_to_split: IVec3,
    pub target_lod: IVec3,
    pub priority: f32
}

impl BaseChunk {
    /// Crea un nuevo chunk base con heap allocation
    pub fn new(position: IVec3) -> Self {
        Self {
            densities: Box::new([[[0.0; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]),
            voxel_types: Box::new([[[VoxelType::Air; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]),
            position,
            last_accessed: Instant::now(),
        }
    }

    /// Obtiene densidad en posicion local
    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[x][y][z]
    }
}

impl DynamicChunkSystem {
    pub fn new() -> Self {
        Self {
            base_chunks: HashMap::new(),
            merged_chunks: HashMap::new(),
            player_position: Vec3::ZERO,
            merge_scheduler: ChunkMergeScheduler::default()
        }
    }

    /// Actualiza posicion del jugador
    pub fn update_player_posicion(&mut self, position: Vec3) {
        self.player_position = position;
    }
}


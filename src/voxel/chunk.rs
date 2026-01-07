//! Sistema de chunks dinámicos inspirado en "Lay of the Land"
//! 
//! Este sistema usa chunks base de 32³ que se combinan dinámicamente según LOD
//! para lograr tanto detalle fino como terreno masivo eficientemente.

use bevy::{math::bounding::Aabb3d, prelude::*};
use noise::{NoiseFn, Perlin};
use std::collections::HashMap;
use crate::core::constants::{BASE_CHUNK_SIZE, MAX_WORLD_HEIGHT, VOXEL_SIZE, LOD_DISTANCES};
use super::voxel_types::VoxelType;

/// Chunk base de 32³ voxels - la unidad fundamental del sistema
/// 
/// # Diseño Inspirado en "Lay of the Land"
/// 
/// Cada chunk base es pequeño (32³) para eficiencia de memoria, pero múltiples
/// chunks se pueden combinar dinámicamente para crear chunks más grandes según LOD.
/// 
/// # Memoria por Chunk Base
/// - Densities: (33 × 33 × 33) × 4 bytes = ~140 KB
/// - Types: (32 × 32 × 32) × 1 byte = ~32 KB
/// - **Total: ~172 KB por chunk base** (vs ~42 MB con chunks 128³!)
#[derive(Component, Clone)]
pub struct BaseChunk {
    // Campo de densidad 3D. Positivo = solido, Negativo = aire
    // Tamaño +1 para permitir interpolación en bordes
    pub densities: [[[f32; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1],
    
    // Tipo de material de cada voxel
    pub voxel_types: [[[VoxelType; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE],
    
    // Posicion del chunk en coordenadas de chunk (X, Y, Z)
    pub position: IVec3,
    
    // LOD actual del chunk
    pub lod_level: ChunkLOD,
    
    // Si el chunk necesita re-meshing
    pub dirty: bool,
}

/// Chunk combinado que representa múltiples chunks base merged
/// 
/// Esto permite tener chunks efectivos de 64³, 128³, 256³, etc.
/// sin duplicar datos - solo referencias a los chunks base.
#[derive(Component)]
pub struct MergedChunk {
    // Referencias a los chunks base que componen este chunk merged
    pub base_chunks: Vec<IVec3>,
    
    // Mesh combinado de todos los chunks base
    pub combined_mesh: Option<Handle<Mesh>>,
    
    // LOD level del chunk merged
    pub lod_level: ChunkLOD,
    
    // Bounds del chunk merged
    pub bounds: Aabb3d,
    
    // Posición central del chunk merged
    pub center_position: IVec3,
}

/// Niveles de detalle para el sistema dinámico de chunks
/// 
/// Basado en el sistema de "Lay of the Land" donde chunks cercanos
/// mantienen máximo detalle y chunks lejanos se combinan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkLOD {
    Ultra,   // 0-50m: 32³ individual chunks (máximo detalle)
    High,    // 50-100m: 64³ (2x2x2 merged)
    Medium,  // 100-200m: 128³ (4x4x4 merged)
    Low,     // 200-400m: 256³ (8x8x8 merged)
    Minimal, // 400m+: 512³ (16x16x16 merged)
}

/// Sistema principal que maneja chunks dinámicos
/// 
/// Mantiene tanto chunks base individuales como chunks merged,
/// y actualiza el LOD dinámicamente según la distancia del jugador.
#[derive(Resource)]
pub struct DynamicChunkSystem {
    // Chunks base de 32³
    pub base_chunks: HashMap<IVec3, BaseChunk>,
    
    // Chunks combinados para LOD lejano
    pub merged_chunks: HashMap<IVec3, MergedChunk>,
    
    // Posición actual del jugador para cálculos de LOD
    pub player_position: Vec3,
    
    // Generador de ruido para terreno
    pub noise_generator: Perlin,
}

// ============================================================================
// IMPLEMENTACIÓN
// ============================================================================

impl BaseChunk {
    /// Crea un nuevo chunk base de 32³ con terreno generado proceduralmente
    pub fn new(position: IVec3, noise_generator: &Perlin) -> Self {
        let mut chunk = Self {
            densities: [[[0.0; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1],
            voxel_types: [[[VoxelType::Air; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE],
            position,
            lod_level: ChunkLOD::Ultra,
            dirty: true,
        };

        // Generar terreno para el chunk base usando el mismo algoritmo que antes
        for x in 0..=BASE_CHUNK_SIZE {
            for y in 0..=BASE_CHUNK_SIZE {
                for z in 0..=BASE_CHUNK_SIZE {
                    // Convierte coordenadas locales a mundiales
                    let world_x = (position.x * BASE_CHUNK_SIZE as i32 + x as i32) as f64 * VOXEL_SIZE as f64;
                    let world_y = (position.y * BASE_CHUNK_SIZE as i32 + y as i32) as f64 * VOXEL_SIZE as f64;
                    let world_z = (position.z * BASE_CHUNK_SIZE as i32 + z as i32) as f64 * VOXEL_SIZE as f64;

                    // Terreno base + ruido (igual que el sistema anterior)
                    // Altura base + variación con Perlin noise
                    let height = 1.5 + noise_generator.get([world_x * 0.2, world_z * 0.2]) * 0.5;
                    let density = height - world_y;

                    chunk.densities[x][y][z] = density as f32;
                    
                    // Determinar tipo de voxel
                    if x < BASE_CHUNK_SIZE && y < BASE_CHUNK_SIZE && z < BASE_CHUNK_SIZE {
                        chunk.voxel_types[x][y][z] = VoxelType::from_density(density as f32, world_y);
                    }
                }
            }
        }

        chunk
    }

    /// Obtiene el valor de densidad en una posición local del chunk
    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[x][y][z]
    }

    /// Obtiene el tipo de voxel en una posición local del chunk
    pub fn get_voxel_type(&self, x: usize, y: usize, z: usize) -> VoxelType {
        self.voxel_types[x][y][z]
    }

    /// Establece el tipo de voxel y marca el chunk como dirty
    pub fn set_voxel_type(&mut self, x: usize, y: usize, z: usize, voxel_type: VoxelType) {
        self.voxel_types[x][y][z] = voxel_type;
        self.dirty = true;
    }
}

impl ChunkLOD {
    /// Calcula el LOD basado en la distancia al jugador
    pub fn from_distance(distance: f32) -> Self {
        if distance < LOD_DISTANCES[0] {
            ChunkLOD::Ultra
        } else if distance < LOD_DISTANCES[1] {
            ChunkLOD::High
        } else if distance < LOD_DISTANCES[2] {
            ChunkLOD::Medium
        } else if distance < LOD_DISTANCES[3] {
            ChunkLOD::Low
        } else {
            ChunkLOD::Minimal
        }
    }

    /// Obtiene el tamaño de merge para este LOD
    pub fn merge_size(&self) -> usize {
        match self {
            ChunkLOD::Ultra => 1,   // 32³ individual
            ChunkLOD::High => 2,    // 64³ (2x2x2)
            ChunkLOD::Medium => 4,  // 128³ (4x4x4)
            ChunkLOD::Low => 8,     // 256³ (8x8x8)
            ChunkLOD::Minimal => 16, // 512³ (16x16x16)
        }
    }

    /// Obtiene el tamaño efectivo en voxels para este LOD
    pub fn effective_size(&self) -> usize {
        BASE_CHUNK_SIZE * self.merge_size()
    }
}

impl DynamicChunkSystem {
    /// Crea un nuevo sistema de chunks dinámicos
    pub fn new() -> Self {
        Self {
            base_chunks: HashMap::new(),
            merged_chunks: HashMap::new(),
            player_position: Vec3::ZERO,
            noise_generator: Perlin::new(12345),
        }
    }

    /// Actualiza la posición del jugador y recalcula LODs
    pub fn update_player_position(&mut self, new_position: Vec3) {
        self.player_position = new_position;
        self.update_chunk_lods();
    }

    /// Recalcula los LODs de todos los chunks basado en la distancia del jugador
    fn update_chunk_lods(&mut self) {
        for (pos, chunk) in &mut self.base_chunks {
            let chunk_world_pos = Vec3::new(
                pos.x as f32 * BASE_CHUNK_SIZE as f32 * VOXEL_SIZE,
                pos.y as f32 * BASE_CHUNK_SIZE as f32 * VOXEL_SIZE,
                pos.z as f32 * BASE_CHUNK_SIZE as f32 * VOXEL_SIZE,
            );
            
            let distance = chunk_world_pos.distance(self.player_position);
            let new_lod = ChunkLOD::from_distance(distance);
            
            if chunk.lod_level != new_lod {
                chunk.lod_level = new_lod;
                chunk.dirty = true;
                // TODO: Actualizar merging de chunks
            }
        }
    }

    /// Obtiene o crea un chunk base en la posición especificada
    pub fn get_or_create_chunk(&mut self, position: IVec3) -> &mut BaseChunk {
        self.base_chunks.entry(position).or_insert_with(|| {
            BaseChunk::new(position, &self.noise_generator)
        })
    }

    /// Calcula cuántos chunks verticales necesitamos para la altura máxima
    pub fn chunks_for_max_height() -> i32 {
        (MAX_WORLD_HEIGHT / BASE_CHUNK_SIZE) as i32
    }
}

impl Default for DynamicChunkSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_chunk_creation() {
        let noise = Perlin::new(12345);
        let chunk = BaseChunk::new(IVec3::ZERO, &noise);
        assert_eq!(chunk.position, IVec3::ZERO);
        assert_eq!(chunk.lod_level, ChunkLOD::Ultra);
    }

    #[test]
    fn test_chunk_lod_from_distance() {
        assert_eq!(ChunkLOD::from_distance(25.0), ChunkLOD::Ultra);
        assert_eq!(ChunkLOD::from_distance(75.0), ChunkLOD::High);
        assert_eq!(ChunkLOD::from_distance(150.0), ChunkLOD::Medium);
        assert_eq!(ChunkLOD::from_distance(300.0), ChunkLOD::Low);
        assert_eq!(ChunkLOD::from_distance(500.0), ChunkLOD::Minimal);
    }

    #[test]
    fn test_lod_merge_sizes() {
        assert_eq!(ChunkLOD::Ultra.merge_size(), 1);
        assert_eq!(ChunkLOD::High.merge_size(), 2);
        assert_eq!(ChunkLOD::Medium.merge_size(), 4);
        assert_eq!(ChunkLOD::Low.merge_size(), 8);
        assert_eq!(ChunkLOD::Minimal.merge_size(), 16);
    }

    #[test]
    fn test_effective_chunk_sizes() {
        assert_eq!(ChunkLOD::Ultra.effective_size(), 32);   // 32³
        assert_eq!(ChunkLOD::High.effective_size(), 64);    // 64³
        assert_eq!(ChunkLOD::Medium.effective_size(), 128); // 128³
        assert_eq!(ChunkLOD::Low.effective_size(), 256);    // 256³
        assert_eq!(ChunkLOD::Minimal.effective_size(), 512); // 512³
    }

    #[test]
    fn test_dynamic_chunk_system() {
        let mut system = DynamicChunkSystem::new();
        
        // Test chunk creation
        let chunk = system.get_or_create_chunk(IVec3::ZERO);
        assert_eq!(chunk.position, IVec3::ZERO);
        
        // Test LOD update
        system.update_player_position(Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(system.player_position, Vec3::new(100.0, 0.0, 0.0));
    }

    #[test]
    fn test_memory_efficiency() {
        // Verificar que los chunks base son mucho más pequeños
        let base_chunk_size = std::mem::size_of::<BaseChunk>();
        
        // Chunk base debería ser ~180KB vs ~42MB del sistema anterior
        assert!(base_chunk_size < 200_000, "Base chunk should be < 200KB, got {} bytes", base_chunk_size);
        
        println!("Base chunk size: {} bytes (~{} KB)", base_chunk_size, base_chunk_size / 1024);
    }

    #[test]
    fn test_vertical_chunks_calculation() {
        let vertical_chunks = DynamicChunkSystem::chunks_for_max_height();
        let expected = (MAX_WORLD_HEIGHT / BASE_CHUNK_SIZE) as i32;
        assert_eq!(vertical_chunks, expected);
        
        // Con 2048 altura máxima y chunks de 32, deberíamos tener 64 chunks verticales
        assert_eq!(vertical_chunks, 64);
    }
}

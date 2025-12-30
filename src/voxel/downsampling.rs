//! Sistema de downsampling para chunks distantes
//! Reduce la resolución de voxels para chunks lejanos

use crate::{
    core::BASE_CHUNK_SIZE,
    voxel::{BaseChunk, VoxelType},
};

/// Chunk con resolución reducida para LOD distante
pub struct DownsampledChunk {
    pub voxel_types: Box<[[[VoxelType; 16]; 16]; 16]>, // 16³ en lugar de 32³
    pub position: bevy::prelude::IVec3,
    pub downsample_factor: usize, // 2 = 16³, 4 = 8³, 8 = 4³
}

impl DownsampledChunk {
    /// Crea un chunk con downsampling desde un BaseChunk
    pub fn from_base_chunk(base_chunk: &BaseChunk, downsample_factor: usize) -> Self {
        match downsample_factor {
            2 => Self::downsample_2x(base_chunk),
            4 => Self::downsample_4x(base_chunk),
            8 => Self::downsample_8x(base_chunk),
            _ => Self::downsample_2x(base_chunk), // Default a 2x
        }
    }
    
    /// Downsampling 2x: 32³ → 16³
    fn downsample_2x(base_chunk: &BaseChunk) -> Self {
        let mut voxel_types = Box::new([[[VoxelType::Air; 16]; 16]; 16]);
        
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    // Tomar el voxel más común en un bloque 2x2x2
                    let mut counts = [0u16; 7]; // u16 para evitar overflow
                    
                    for dx in 0..2 {
                        for dy in 0..2 {
                            for dz in 0..2 {
                                let bx = (x * 2 + dx).min(31);
                                let by = (y * 2 + dy).min(31);
                                let bz = (z * 2 + dz).min(31);
                                let voxel = base_chunk.voxel_types[bx][by][bz];
                                counts[voxel as usize] += 1;
                            }
                        }
                    }
                    
                    let most_common = counts.iter()
                        .enumerate()
                        .max_by_key(|&(_, count)| count)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    
                    voxel_types[x][y][z] = match most_common {
                        0 => VoxelType::Air,
                        1 => VoxelType::Stone,
                        2 => VoxelType::Dirt,
                        3 => VoxelType::Wood,
                        4 => VoxelType::Metal,
                        5 => VoxelType::Grass,
                        6 => VoxelType::Sand,
                        _ => VoxelType::Air,
                    };
                }
            }
        }
        
        Self {
            voxel_types,
            position: base_chunk.position,
            downsample_factor: 2,
        }
    }
    
    /// Downsampling 4x: 32³ → 8³ (guardado como 16³ con padding)
    fn downsample_4x(base_chunk: &BaseChunk) -> Self {
        let mut voxel_types = Box::new([[[VoxelType::Air; 16]; 16]; 16]);
        
        for x in 0..8 {
            for y in 0..8 {
                for z in 0..8 {
                    // Tomar el voxel más común en un bloque 4x4x4
                    let mut counts = [0u16; 7]; // u16 para evitar overflow
                    
                    for dx in 0..4 {
                        for dy in 0..4 {
                            for dz in 0..4 {
                                let bx = (x * 4 + dx).min(31);
                                let by = (y * 4 + dy).min(31);
                                let bz = (z * 4 + dz).min(31);
                                let voxel = base_chunk.voxel_types[bx][by][bz];
                                counts[voxel as usize] += 1;
                            }
                        }
                    }
                    
                    let most_common = counts.iter()
                        .enumerate()
                        .max_by_key(|&(_, count)| count)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    
                    voxel_types[x][y][z] = match most_common {
                        0 => VoxelType::Air,
                        1 => VoxelType::Stone,
                        2 => VoxelType::Dirt,
                        3 => VoxelType::Wood,
                        4 => VoxelType::Metal,
                        5 => VoxelType::Grass,
                        6 => VoxelType::Sand,
                        _ => VoxelType::Air,
                    };
                }
            }
        }
        
        Self {
            voxel_types,
            position: base_chunk.position,
            downsample_factor: 4,
        }
    }
    
    /// Downsampling 8x: 32³ → 4³ (guardado como 16³ con padding)
    fn downsample_8x(base_chunk: &BaseChunk) -> Self {
        let mut voxel_types = Box::new([[[VoxelType::Air; 16]; 16]; 16]);
        
        for x in 0..4 {
            for y in 0..4 {
                for z in 0..4 {
                    // Tomar el voxel más común en un bloque 8x8x8
                    let mut counts = [0u16; 7]; // u16 para evitar overflow (8x8x8 = 512 max)
                    
                    for dx in 0..8 {
                        for dy in 0..8 {
                            for dz in 0..8 {
                                let bx = (x * 8 + dx).min(31);
                                let by = (y * 8 + dy).min(31);
                                let bz = (z * 8 + dz).min(31);
                                let voxel = base_chunk.voxel_types[bx][by][bz];
                                counts[voxel as usize] += 1;
                            }
                        }
                    }
                    
                    let most_common = counts.iter()
                        .enumerate()
                        .max_by_key(|&(_, count)| count)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    
                    voxel_types[x][y][z] = match most_common {
                        0 => VoxelType::Air,
                        1 => VoxelType::Stone,
                        2 => VoxelType::Dirt,
                        3 => VoxelType::Wood,
                        4 => VoxelType::Metal,
                        5 => VoxelType::Grass,
                        6 => VoxelType::Sand,
                        _ => VoxelType::Air,
                    };
                }
            }
        }
        
        Self {
            voxel_types,
            position: base_chunk.position,
            downsample_factor: 8,
        }
    }
    
    /// Obtiene el tamaño efectivo del chunk downsampled
    pub fn effective_size(&self) -> usize {
        BASE_CHUNK_SIZE / self.downsample_factor
    }
}

//! Sistema de chunks dinamicos
//! Chunks base de 32³ con generacion de terreno optimizada
//! Incluye sistema de biomas con montañas, valles, llanuras, etc.

use bevy::prelude::*;
use rayon::prelude::*;
use crate::core::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{VoxelType, TerrainGenerator};

/// Chunk base de 32³ (usa heap para evitar stack overflow)
#[derive(Component)]
pub struct BaseChunk {
    pub densities: Box<[[[f32; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]>,
    pub voxel_types: Box<[[[VoxelType; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]>,
    pub position: IVec3,
}

impl BaseChunk {
    pub fn new(position: IVec3) -> Self {
        let mut chunk = Self {
            densities: Box::new([[[0.0; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]),
            voxel_types: Box::new([[[VoxelType::Air; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]),
            position,
        };
        
        // Generar terreno usando la versión optimizada con biomas
        chunk.generate_terrain();
        chunk
    }

    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[x][y][z]
    }

    /// Generación de terreno con biomas
    /// Combina FastNoiseLite + Rayon + Sistema de Biomas
    pub fn generate_terrain(&mut self) {
        let chunk_pos = self.position;
        
        // Crear generador de terreno UNA VEZ para todo el chunk
        let mut terrain_gen = TerrainGenerator::new(12345);
        
        // Calcular todas las densidades en paralelo
        let total_size = (BASE_CHUNK_SIZE + 1).pow(3);
        
        // Pre-calcular coordenadas mundiales para evitar recalcular en cada thread
        let mut world_coords = Vec::with_capacity(total_size);
        for idx in 0..total_size {
            let x = idx % (BASE_CHUNK_SIZE + 1);
            let y = (idx / (BASE_CHUNK_SIZE + 1)) % (BASE_CHUNK_SIZE + 1);
            let z = idx / ((BASE_CHUNK_SIZE + 1) * (BASE_CHUNK_SIZE + 1));

            let world_x = (chunk_pos.x * BASE_CHUNK_SIZE as i32 + x as i32) as f32 * VOXEL_SIZE;
            let world_z = (chunk_pos.z * BASE_CHUNK_SIZE as i32 + z as i32) as f32 * VOXEL_SIZE;
            let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f32 * VOXEL_SIZE;
            
            world_coords.push((world_x, world_y, world_z));
        }
        
        // Calcular densidades (sin paralelización para evitar problemas con el generador)
        let densities_flat: Vec<f32> = world_coords.iter()
            .map(|(world_x, world_y, world_z)| {
                terrain_gen.get_density(*world_x, *world_y, *world_z)
            })
            .collect();

        // Copiar resultados al array 3D
        for (idx, &density) in densities_flat.iter().enumerate() {
            let x = idx % (BASE_CHUNK_SIZE + 1);
            let y = (idx / (BASE_CHUNK_SIZE + 1)) % (BASE_CHUNK_SIZE + 1);
            let z = idx / ((BASE_CHUNK_SIZE + 1) * (BASE_CHUNK_SIZE + 1));
            self.densities[x][y][z] = density;
        }

        // Calcular tipos de voxel también en paralelo
        let voxel_types_flat: Vec<VoxelType> = (0..BASE_CHUNK_SIZE.pow(3))
            .into_par_iter()
            .map(|idx| {
                let x = idx % BASE_CHUNK_SIZE;
                let y = (idx / BASE_CHUNK_SIZE) % BASE_CHUNK_SIZE;
                let z = idx / (BASE_CHUNK_SIZE * BASE_CHUNK_SIZE);
                
                let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f64 * VOXEL_SIZE as f64;
                let density = densities_flat[x + y * (BASE_CHUNK_SIZE + 1) + z * (BASE_CHUNK_SIZE + 1) * (BASE_CHUNK_SIZE + 1)];
                
                VoxelType::from_density(density, world_y)
            })
            .collect();

        // Copiar tipos de voxel
        for (idx, voxel_type) in voxel_types_flat.iter().enumerate() {
            let x = idx % BASE_CHUNK_SIZE;
            let y = (idx / BASE_CHUNK_SIZE) % BASE_CHUNK_SIZE;
            let z = idx / (BASE_CHUNK_SIZE * BASE_CHUNK_SIZE);
            self.voxel_types[x][y][z] = *voxel_type;
        }
    }
}
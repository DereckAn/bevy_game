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
    /// Optimizado: calcula heightmap una vez por columna XZ (33x33 = 1,089 evaluaciones)
    /// en lugar de por cada voxel (33³ = 35,937 evaluaciones)
    pub fn generate_terrain(&mut self) {
        let chunk_pos = self.position;

        // Crear generador de terreno UNA VEZ para todo el chunk
        let mut terrain_gen = TerrainGenerator::new(12345);

        // Paso 1: Calcular heightmap 2D (solo XZ, una vez por columna)
        // Necesitamos (BASE_CHUNK_SIZE + 1) para las densidades en los bordes
        let grid = BASE_CHUNK_SIZE + 1;
        let mut heightmap = vec![0.0f32; grid * grid];

        for z in 0..grid {
            for x in 0..grid {
                let world_x = (chunk_pos.x * BASE_CHUNK_SIZE as i32 + x as i32) as f32 * VOXEL_SIZE;
                let world_z = (chunk_pos.z * BASE_CHUNK_SIZE as i32 + z as i32) as f32 * VOXEL_SIZE;
                heightmap[x + z * grid] = terrain_gen.biome_gen.generate_height(world_x, world_z);
            }
        }

        // Paso 2: Calcular densidades usando el heightmap cacheado (solo aritmética)
        for z in 0..grid {
            for y in 0..grid {
                let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f32 * VOXEL_SIZE;
                for x in 0..grid {
                    let terrain_height = heightmap[x + z * grid];
                    self.densities[x][y][z] = terrain_height - world_y;
                }
            }
        }

        // Paso 3: Calcular tipos de voxel en paralelo usando las densidades ya calculadas
        let densities_ref = &self.densities;
        let voxel_types_flat: Vec<VoxelType> = (0..BASE_CHUNK_SIZE.pow(3))
            .into_par_iter()
            .map(|idx| {
                let x = idx % BASE_CHUNK_SIZE;
                let y = (idx / BASE_CHUNK_SIZE) % BASE_CHUNK_SIZE;
                let z = idx / (BASE_CHUNK_SIZE * BASE_CHUNK_SIZE);

                let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f64 * VOXEL_SIZE as f64;
                let density = densities_ref[x][y][z];

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
//! Sistema de chunks dinamicos
//! Chunks base de 32³ con generacion de terreno optimizada
//! Incluye sistema de biomas con montañas, valles, llanuras, etc.

use crate::core::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{TerrainGenerator, VoxelType};
use bevy::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;

/// Chunk base de 32³ (usa heap para evitar stack overflow)
#[derive(Component)]
pub struct BaseChunk {
    pub voxel_types: Box<[[[VoxelType; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]>,
    pub position: IVec3,
}

impl BaseChunk {
    /// Aplica los diffs del jugador encima del terreno procedural.
    ///
    /// Cada entrada (local_pos → tipo) sobrescribe un voxel ya generado desde
    /// el seed.
    pub fn apply_diffs(&mut self, diffs: &HashMap<IVec3, VoxelType>) {
        for (local_pos, voxel_type) in diffs {
            let x = local_pos.x as usize;
            let y = local_pos.y as usize;
            let z = local_pos.z as usize;

            self.voxel_types[x][y][z] = *voxel_type;
        }
    }

    pub fn new(position: IVec3, seed: i32) -> Self {
        let mut chunk = Self {
            voxel_types: Box::new(
                [[[VoxelType::Air; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE],
            ),
            position,
        };

        // Generar terreno usando la versión optimizada con biomas
        chunk.generate_terrain(seed);
        chunk
    }

    /// Un voxel es sólido si no es aire. Reemplaza a `get_density() <= 0.0`:
    /// `voxel_types` ya contiene exactamente esa información.
    pub fn is_solid(&self, x: usize, y: usize, z: usize) -> bool {
        self.voxel_types[x][y][z] != VoxelType::Air
    }

    /// Generación de terreno con biomas
    /// Combina FastNoiseLite + Rayon + Sistema de Biomas
    /// Optimizado: calcula heightmap una vez por columna XZ (33x33 = 1,089 evaluaciones)
    /// en lugar de por cada voxel (33³ = 35,937 evaluaciones)
    pub fn generate_terrain(&mut self, seed: i32) {
        let chunk_pos = self.position;

        // Crear generador de terreno UNA VEZ para todo el chunk
        let mut terrain_gen = TerrainGenerator::new(seed);

        // Paso 1: Calcular heightmap 2D (solo XZ, una vez por columna)
        let grid = BASE_CHUNK_SIZE + 1;
        let mut heightmap = vec![0.0f32; grid * grid];

        for z in 0..grid {
            for x in 0..grid {
                let world_x = (chunk_pos.x * BASE_CHUNK_SIZE as i32 + x as i32) as f32 * VOXEL_SIZE;
                let world_z = (chunk_pos.z * BASE_CHUNK_SIZE as i32 + z as i32) as f32 * VOXEL_SIZE;
                heightmap[x + z * grid] = terrain_gen.biome_gen.generate_height(world_x, world_z);
            }
        }

        // Paso 2: Calcular tipos de voxel en paralelo a partir de la profundidad.
        // Se clasifican por PROFUNDIDAD bajo la superficie (pasto/tierra/piedra),
        // no por altura absoluta, para que la superficie siempre sea excavable.
        let heightmap_ref = &heightmap;
        let voxel_types_flat: Vec<VoxelType> = (0..BASE_CHUNK_SIZE.pow(3))
            .into_par_iter()
            .map(|idx| {
                let x = idx % BASE_CHUNK_SIZE;
                let y = (idx / BASE_CHUNK_SIZE) % BASE_CHUNK_SIZE;
                let z = idx / (BASE_CHUNK_SIZE * BASE_CHUNK_SIZE);

                let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f32 * VOXEL_SIZE;
                let terrain_height = heightmap_ref[x + z * grid];
                let depth = terrain_height - world_y;

                VoxelType::from_depth(depth, depth)
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

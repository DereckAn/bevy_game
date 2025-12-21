use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SIZE: f32 = 0.1;

#[derive(Copy, Clone, PartialEq, Default)]
pub enum Voxel {
    #[default]
    Air,
    Solid(f32), // f32 = densidad (-1.0 a 1.0)
}

#[derive(Component)]
pub struct Chunk{
    pub densities: [[[f32; CHUNK_SIZE + 1]; CHUNK_SIZE + 1]; CHUNK_SIZE + 1],
    // pub voxels: [[[Voxel; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    pub position: IVec3
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        let perlin = Perlin::new(12345);
        let mut chunk = Self {
            densities: [[[0.0; CHUNK_SIZE + 1]; CHUNK_SIZE + 1]; CHUNK_SIZE + 1],
            position
        };

        // Terreno simple: mitad inferior solida
        for x in 0..=CHUNK_SIZE  {
            for y in 0..=CHUNK_SIZE {
                for z in 0..=CHUNK_SIZE {
                    let world_x = (position.x * CHUNK_SIZE as i32 + x as i32) as f64 * VOXEL_SIZE as f64;
                    let world_y = (position.y * CHUNK_SIZE as i32 + y as i32) as f64 * VOXEL_SIZE as f64;
                    let world_z = (position.z * CHUNK_SIZE as i32 + z as i32) as f64 * VOXEL_SIZE as f64;

                    // Terreno base + ruido
                    let height = 1.5 + perlin.get([world_x * 0.2, world_z * 0.2]) * 0.5;
                    let density = height - world_y;

                    chunk.densities[x][y][z] = density as f32;
                }
            }
        }

        chunk
    }

    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[x][y][z]
    }
}   
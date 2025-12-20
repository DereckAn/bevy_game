use bevy::prelude::*;

pub const CHUNK_SIZE: usize = 32;
pub const VOXEL_SIZE: f32 = 0.1;

#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub enum Voxel {
    #[default]
    Air,
    Solid
}

#[derive(Component)]
pub struct Chunk{
    pub voxels: [[[Voxel; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    pub position: IVec3
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        let mut chunk = Self {
            voxels: [[[Voxel::Air; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
            position
        };

        // Terreno simple: mitad inferior solida
        for x in 0..CHUNK_SIZE  {
            for y in 0..CHUNK_SIZE / 2 {
                for z in 0..CHUNK_SIZE {
                    chunk.voxels[x][y][z] = Voxel::Solid;
                }
            }
        }

        chunk
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Voxel {
        self.voxels[x][y][z]
    }
}   
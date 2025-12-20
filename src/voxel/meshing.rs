use bevy::prelude::*;
use bevy::mesh::{Indices, PrimitiveTopology};
use super::{Chunk, Voxel, CHUNK_SIZE, VOXEL_SIZE};

pub fn generate_mesh(chunk: &Chunk) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if chunk.get(x,y,z) == Voxel::Air{
                    continue;
                }

                let pos = Vec3::new(x as f32,y as f32,z as f32) * VOXEL_SIZE;

                // Revisar cada cara
                let faces = [
                    (IVec3::Y,[0,1,2,3]), //top
                    (IVec3::NEG_Y,[4,5,6,7]), // Bottom
                    (IVec3::X,[8,9,10,11]), // Right
                    (IVec3::NEG_X,[12,13,14,15]), // Left
                    (IVec3::Z,[16,17,18,19]), // Front
                    (IVec3::NEG_Z,[20,21,22,23]), //Back
                ];

                for (dir, _) in faces.iter() {
                    let nx = x as i32 + dir.x;
                    let ny = y as i32 + dir.y;
                    let nz = z as i32 + dir.z;

                    let neighbor_solid = if nx >= 0 && nx < CHUNK_SIZE as i32
                        && ny >= 0 && ny < CHUNK_SIZE as i32
                        && nz >= 0 && nz < CHUNK_SIZE as i32
                    {
                        chunk.get(nx as usize, ny as usize, nz as usize) == Voxel::Solid
                    } else {
                        false
                    };

                    if !neighbor_solid {
                        add_face(&mut positions, &mut normals, &mut indices, pos, *dir);
                    }
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh

}

fn add_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    pos: Vec3,
    dir: IVec3,
) {
    let s = VOXEL_SIZE;
    let base_index = positions.len() as u32;
    let normal = [dir.x as f32, dir.y as f32, dir.z as f32];

    let verts: [[f32; 3]; 4] = match dir {
        IVec3 { x: 0, y: 1, z: 0 } => [ // Top
            [pos.x, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z + s],
            [pos.x, pos.y + s, pos.z + s],
        ],
        IVec3 { x: 0, y: -1, z: 0 } => [ // Bottom
            [pos.x, pos.y, pos.z + s],
            [pos.x + s, pos.y, pos.z + s],
            [pos.x + s, pos.y, pos.z],
            [pos.x, pos.y, pos.z],
        ],
        IVec3 { x: 1, y: 0, z: 0 } => [ // Right
            [pos.x + s, pos.y, pos.z],
            [pos.x + s, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z + s],
            [pos.x + s, pos.y, pos.z + s],
        ],
        IVec3 { x: -1, y: 0, z: 0 } => [ // Left
            [pos.x, pos.y, pos.z + s],
            [pos.x, pos.y + s, pos.z + s],
            [pos.x, pos.y + s, pos.z],
            [pos.x, pos.y, pos.z],
        ],
        IVec3 { x: 0, y: 0, z: 1 } => [ // Front
            [pos.x + s, pos.y, pos.z + s],
            [pos.x + s, pos.y + s, pos.z + s],
            [pos.x, pos.y + s, pos.z + s],
            [pos.x, pos.y, pos.z + s],
        ],
        IVec3 { x: 0, y: 0, z: -1 } => [ // Back
            [pos.x, pos.y, pos.z],
            [pos.x, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z],
            [pos.x + s, pos.y, pos.z],
        ],
        _ => return,
    };

    for v in verts {
        positions.push(v);
        normals.push(normal);
    }

    indices.extend_from_slice(&[
        base_index, base_index + 1, base_index + 2,
        base_index, base_index + 2, base_index + 3,
    ]);
}
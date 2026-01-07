//! Sistema de meshing para chunks dinámicos
//!
//! Implementación temporal para el nuevo sistema de chunks dinámicos 3D.
//! Genera meshes básicos de cubos para visualizar el terreno.

use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use crate::core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::chunk::BaseChunk;

/// Genera un mesh 3D básico para un chunk.
/// 
/// Implementación temporal que crea cubos simples para cada voxel sólido.
/// TODO: Implementar greedy meshing y dual contouring en el futuro.
pub fn generate_mesh(chunk: &BaseChunk) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Iterar por todos los voxels del chunk
    for x in 0..BASE_CHUNK_SIZE {
        for y in 0..BASE_CHUNK_SIZE {
            for z in 0..BASE_CHUNK_SIZE {
                let voxel_type = chunk.get_voxel_type(x, y, z);
                
                // Solo generar geometría para voxels sólidos
                if voxel_type.is_solid() {
                    // Calcular posición mundial del voxel
                    let world_pos = Vec3::new(
                        (chunk.position.x * BASE_CHUNK_SIZE as i32 + x as i32) as f32 * VOXEL_SIZE,
                        (chunk.position.y * BASE_CHUNK_SIZE as i32 + y as i32) as f32 * VOXEL_SIZE,
                        (chunk.position.z * BASE_CHUNK_SIZE as i32 + z as i32) as f32 * VOXEL_SIZE,
                    );

                    // Face culling simple - verificar vecinos dentro del mismo chunk
                    let should_render_faces = [
                        // Top (+Y)
                        y == BASE_CHUNK_SIZE - 1 || !chunk.get_voxel_type(x, y + 1, z).is_solid(),
                        // Bottom (-Y)  
                        y == 0 || !chunk.get_voxel_type(x, y - 1, z).is_solid(),
                        // Right (+X)
                        x == BASE_CHUNK_SIZE - 1 || !chunk.get_voxel_type(x + 1, y, z).is_solid(),
                        // Left (-X)
                        x == 0 || !chunk.get_voxel_type(x - 1, y, z).is_solid(),
                        // Front (+Z)
                        z == BASE_CHUNK_SIZE - 1 || !chunk.get_voxel_type(x, y, z + 1).is_solid(),
                        // Back (-Z)
                        z == 0 || !chunk.get_voxel_type(x, y, z - 1).is_solid(),
                    ];

                    // Agregar caras que necesitan ser renderizadas
                    add_voxel_faces(&mut positions, &mut normals, &mut indices, world_pos, &should_render_faces);
                }
            }
        }
    }

    // Crear el mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    
    if !positions.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_indices(Indices::U32(indices));
    }

    mesh
}

/// Genera un mesh 3D con face culling inteligente entre chunks.
/// 
/// TODO: Implementar verificación de chunks vecinos.
/// Por ahora usa la misma implementación que generate_mesh.
pub fn generate_mesh_with_neighbors(
    chunk: &BaseChunk,
    _chunk_system: &crate::voxel::DynamicChunkSystem,
) -> Mesh {
    // Por ahora, usar la implementación simple
    // TODO: Implementar face culling entre chunks
    generate_mesh(chunk)
}

/// Agrega las caras de un voxel al mesh
fn add_voxel_faces(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    pos: Vec3,
    should_render: &[bool; 6],
) {
    let s = VOXEL_SIZE;
    
    // Definir las caras del cubo
    let faces = [
        // Top (+Y)
        ([
            [pos.x, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z + s],
            [pos.x, pos.y + s, pos.z + s],
        ], [0.0, 1.0, 0.0]),
        
        // Bottom (-Y)
        ([
            [pos.x, pos.y, pos.z + s],
            [pos.x + s, pos.y, pos.z + s],
            [pos.x + s, pos.y, pos.z],
            [pos.x, pos.y, pos.z],
        ], [0.0, -1.0, 0.0]),
        
        // Right (+X)
        ([
            [pos.x + s, pos.y, pos.z],
            [pos.x + s, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z + s],
            [pos.x + s, pos.y, pos.z + s],
        ], [1.0, 0.0, 0.0]),
        
        // Left (-X)
        ([
            [pos.x, pos.y, pos.z + s],
            [pos.x, pos.y + s, pos.z + s],
            [pos.x, pos.y + s, pos.z],
            [pos.x, pos.y, pos.z],
        ], [-1.0, 0.0, 0.0]),
        
        // Front (+Z)
        ([
            [pos.x + s, pos.y, pos.z + s],
            [pos.x + s, pos.y + s, pos.z + s],
            [pos.x, pos.y + s, pos.z + s],
            [pos.x, pos.y, pos.z + s],
        ], [0.0, 0.0, 1.0]),
        
        // Back (-Z)
        ([
            [pos.x, pos.y, pos.z],
            [pos.x, pos.y + s, pos.z],
            [pos.x + s, pos.y + s, pos.z],
            [pos.x + s, pos.y, pos.z],
        ], [0.0, 0.0, -1.0]),
    ];

    // Agregar cada cara que debe ser renderizada
    for (i, (verts, normal)) in faces.iter().enumerate() {
        if should_render[i] {
            let idx = positions.len() as u32;
            
            // Agregar vértices
            positions.extend_from_slice(verts);
            
            // Agregar normales (4 vértices por cara)
            normals.extend_from_slice(&[*normal; 4]);
            
            // Agregar índices (2 triángulos por cara)
            indices.extend_from_slice(&[
                idx, idx + 1, idx + 2,
                idx, idx + 2, idx + 3,
            ]);
        }
    }
}
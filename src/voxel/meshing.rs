//! # Generacion de Mesh con Surface Nets
//! 
//! Implementa el algoritmo Surface Nets para generar meshes sueaves
//! a partir de campos de densidad. Este algoritmo:
//! 
//! 1. Encuentra celdas que cruzan la superficie (isosuperficie donde densidad = 0)
//! 2. Cloca un vertice en cada celda usando interpolacion de las aristas
//! 3. Conecta vertices adyacentes con quads para formar la supercifie

use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use crate::voxel::chunk::{Chunk};
use crate::core::constants::{CHUNK_SIZE, VOXEL_SIZE};

enum Face {
    Bottom,
    Top,
    Left,
    Right,
    Front,
    Back,
}

// ============================================================================
// FUNCIONES PÚBLICAS
// ============================================================================

/// Genera un mesh 3D a partir de un chunk usando el algoritmo Surface Nets.
/// 
/// # Proceso
/// 1. Itera todas las celdas del chunk
/// 2. Para celdas que cruzan la superficie, calcula un vértice interpolado
/// 3. Calcula normales usando el gradiente del campo de densidad
/// 4. Conecta vértices adyacentes con quads (2 triángulos cada uno)
/// 
/// # Parámetros
/// - `chunk`: El chunk a convertir en mesh
/// 
/// # Retorna
/// Un `Mesh` de Bevy listo para renderizar
pub fn generate_mesh(chunk: &Chunk) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Paso 1: Generar vértices en celdas que cruzan la superficie
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if chunk.get_density(x,y,z) <= 0.0 {
                    continue; // Es aire, saltar
                }

                let base = Vec3::new(
                    (chunk.position.x * CHUNK_SIZE as i32 + x as i32) as f32,
                    (chunk.position.y * CHUNK_SIZE as i32 + y as i32) as f32,
                    (chunk.position.z * CHUNK_SIZE as i32 + z as i32) as f32
                ) * VOXEL_SIZE;

                // Cara +y (arriba)
                if y == CHUNK_SIZE - 1 || chunk.get_density(x, y + 1, z) <= 0.0 {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Top);
                }

                // Cada -Y (abajo)
                if y == 0 || chunk.get_density(x, y - 1, z) <= 0.0 {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Bottom);
                }   

                // Cara +X
                if x == CHUNK_SIZE - 1 || chunk.get_density(x + 1, y, z) <= 0.0 {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Right);
                }

                // Cara -X
                if x == 0 || chunk.get_density(x - 1, y, z) <= 0.0 {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Left);
                }   

                // Cara +Z
                if z == CHUNK_SIZE - 1 || chunk.get_density(x, y, z + 1) <= 0.0 {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Front);
                }   

                // Cara -Z 
                if z == 0 || chunk.get_density(x, y, z - 1) <= 0.0 {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Back);
                }   


            }
        }
    }

    println!("Vertices: {}, Indices: {}", positions.len(), indices.len());

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}


// ============================================================================
// FUNCIONES PRIVADAS
// ============================================================================

fn add_face(
    positions: &mut Vec<[f32; 3]>, 
    normals: &mut Vec<[f32; 3]>, 
    indices: &mut Vec<u32>, 
    base: Vec3, 
    face: Face,
){
    let s = VOXEL_SIZE;
    let idx = positions.len() as u32;

     let (verts, normal) = match face {
        Face::Top => ([
            [base.x, base.y + s, base.z],
            [base.x + s, base.y + s, base.z],
            [base.x + s, base.y + s, base.z + s],
            [base.x, base.y + s, base.z + s],
        ], [0.0, 1.0, 0.0]),
        Face::Bottom => ([
            [base.x, base.y, base.z + s],
            [base.x + s, base.y, base.z + s],
            [base.x + s, base.y, base.z],
            [base.x, base.y, base.z],
        ], [0.0, -1.0, 0.0]),
        Face::Right => ([
            [base.x + s, base.y, base.z],
            [base.x + s, base.y + s, base.z],
            [base.x + s, base.y + s, base.z + s],
            [base.x + s, base.y, base.z + s],
        ], [1.0, 0.0, 0.0]),
        Face::Left => ([
            [base.x, base.y, base.z + s],
            [base.x, base.y + s, base.z + s],
            [base.x, base.y + s, base.z],
            [base.x, base.y, base.z],
        ], [-1.0, 0.0, 0.0]),
        Face::Front => ([
            [base.x + s, base.y, base.z + s],
            [base.x + s, base.y + s, base.z + s],
            [base.x, base.y + s, base.z + s],
            [base.x, base.y, base.z + s],
        ], [0.0, 0.0, 1.0]),
        Face::Back => ([
            [base.x, base.y, base.z],
            [base.x, base.y + s, base.z],
            [base.x + s, base.y + s, base.z],
            [base.x + s, base.y, base.z],
        ], [0.0, 0.0, -1.0]),
    };

    positions.extend_from_slice(&verts);
    normals.extend_from_slice(&[normal; 4]);
    indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_generation() {
        let chunk = Chunk::new(IVec3::ZERO);
        let mesh = generate_mesh(&chunk);
        
        // El mesh debe tener posiciones y normales
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.indices().is_some());
    }

    #[test]
    fn test_mesh_has_vertices() {
        let chunk = Chunk::new(IVec3::ZERO);
        let mesh = generate_mesh(&chunk);
        
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) = 
            mesh.attribute(Mesh::ATTRIBUTE_POSITION) 
        {
            // Debe haber vértices ya que el chunk tiene terreno
            assert!(!positions.is_empty(), "Mesh should have vertices");
        }
    }

}
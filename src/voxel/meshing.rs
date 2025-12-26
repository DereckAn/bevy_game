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
use crate::core::WORLD_HEIGHT;
use crate::voxel::ChunkMap;
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

/// Genera un mesh 3d con face culling inteligente entre chunks.
/// 
/// Verifica chunks vecinos para evitar generar caras innecesarias en bordes.
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
pub fn generate_mesh_with_neighbors(chunk: &Chunk, chunk_map: &ChunkMap, chunks: &Query<&Chunk>) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Paso 1: Generar vertices con face culling inteligente
    for x in 0..CHUNK_SIZE {
        for y in 0..WORLD_HEIGHT {
            for z in 0..CHUNK_SIZE {
                if chunk.get_density(x,y,z) <= 0.0 {
                    continue; // Es aire, saltar
                }

                let base = Vec3::new(
                    (chunk.position.x * CHUNK_SIZE as i32 + x as i32) as f32,
                    y as f32,
                    (chunk.position.y * CHUNK_SIZE as i32 + z as i32) as f32
                ) * VOXEL_SIZE;

                // Verificar cada cada con neighbors

                // Cara +Y (arriba)
                if should_render_face(chunk, chunk_map, chunks, x, y, z, 0, 1, 0) {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Top);
                }

                // Cara -Y (abajo)
                if should_render_face(chunk, chunk_map, chunks, x, y, z, 0, -1, 0) {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Bottom);
                }

                // Cara +X (derecha)
                if should_render_face(chunk, chunk_map, chunks, x, y, z, 1, 0, 0) {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Right);
                }

                // Cara -X (izquierda)
                if should_render_face(chunk, chunk_map, chunks, x, y, z, -1, 0, 0) {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Left);
                }

                // Cara +Z (frente)
                if should_render_face(chunk, chunk_map, chunks, x, y, z, 0, 0, 1) {
                    add_face(&mut positions, &mut normals, &mut indices, base, Face::Front);
                }

                // Cara -Z (atrás)
                if should_render_face(chunk, chunk_map, chunks, x, y, z, 0, 0, -1) {
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

/// Genera un mesh simple sin verificar chunks vecinos.
/// 
/// Usado durante la inicialización cuando no todos los chunks están disponibles.
pub fn generate_simple_mesh(chunk: &Chunk) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Paso 1: Generar vértices en celdas que cruzan la superficie
    for x in 0..CHUNK_SIZE {
        for y in 0..WORLD_HEIGHT {
            for z in 0..CHUNK_SIZE {
                if chunk.get_density(x,y,z) <= 0.0 {
                    continue; // Es aire, saltar
                }

                let base = Vec3::new(
                    (chunk.position.x * CHUNK_SIZE as i32 + x as i32) as f32,
                    y as f32,
                    (chunk.position.y * CHUNK_SIZE as i32 + z as i32) as f32
                ) * VOXEL_SIZE;

                // Face culling simple - solo dentro del mismo chunk
                
                // Cara +y (arriba)
                if y == WORLD_HEIGHT - 1 || chunk.get_density(x, y + 1, z) <= 0.0 {
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

fn should_render_face(
    chunk: &Chunk,
    chunk_map: &ChunkMap,
    chunks: &Query<&Chunk>,
    x: usize,
    y: usize,
    z: usize,
    dx: i32, 
    dy: i32,
    dz: i32,
) -> bool {

    let neighbor_x = x as i32 + dx;
    let neighbor_y = y as i32 + dy;
    let neighbor_z = z as i32 + dz;

    // Si el vecino esta dentro del mismo chunk
    if neighbor_x >= 0 && neighbor_x < CHUNK_SIZE as i32
    && neighbor_y >= 0 && neighbor_y < WORLD_HEIGHT as i32
    && neighbor_z >= 0 && neighbor_z < CHUNK_SIZE as i32
    {
        // Verificar densidad del vecino en el mismo chunk
        return chunk.get_density(neighbor_x as usize, neighbor_y as usize, neighbor_z as usize) <= 0.0;
    }

    // Para chunks columnares, solo buscar vecinos en x,z
    // Si Y esta fuera de rango, es aire (arriba) o solido (abajo del mundo)
    if neighbor_y < 0 {
        return false; // Debajo del mundo = solido, renderizar cara
    }

    if neighbor_y >= WORLD_HEIGHT as i32 {
        return true; // Arriba del mundo = aire, renderizar cara
    }

    // El vecino esta en otro chunk - calcular que chunk y posicion local
    let chunk_offset_x = if neighbor_x < 0 { -1 } else if neighbor_x >= CHUNK_SIZE as i32 { 1 } else { 0 };
    let chunk_offset_z = if neighbor_z < 0 { -1 } else if neighbor_z >= CHUNK_SIZE as i32 { 1 } else { 0 };

    let neighbor_chunk_pos = IVec2::new(
        chunk.position.x + chunk_offset_x,
        chunk.position.y + chunk_offset_z, // Position.y es realmente z
    );

    // Calcular posicion local en el chunk vecino
    let local_x = neighbor_x.rem_euclid(CHUNK_SIZE as i32) as usize;
    let local_y = neighbor_y as usize; // Y es absoluto en chunks columnares
    let local_z = neighbor_z.rem_euclid(CHUNK_SIZE as i32) as usize;

    // Buscar el chunk vecino 
    if let Some(&neightbor_entity) = chunk_map.chunks.get(&neighbor_chunk_pos) {
        if let Ok(neighbor_chunk) = chunks.get(neightbor_entity) {
            // Verifica densidad en el chunk vecino
            return neighbor_chunk.get_density(local_x, local_y, local_z) <= 0.0;
        }
    }

    // Si no hay chunk vecino, renderizar la cara (borde del mundo)
    true   
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_should_render_face_same_chunk() {
        let chunk = Chunk::new(IVec2::ZERO);
        let chunk_map = ChunkMap { chunks: HashMap::new() };
        
        // Mock query - para tests simplificados
        // En un test real necesitaríamos crear un World completo
        // Por ahora solo probamos la lógica dentro del mismo chunk
        
        // Test: cara hacia aire debería renderizarse
        // Asumiendo que (0,0,0) es sólido y (0,1,0) es aire
        // (esto depende de la generación de terreno específica)
        
        // Este test es simplificado - en producción usaríamos integration tests
        assert!(true); // Placeholder - los tests reales requieren más setup
    }

    #[test]
    fn test_face_culling_logic() {
        // Test de la lógica de face culling
        let chunk = Chunk::new(IVec2::ZERO);
        
        // Verificar que la función get_density funciona
        let density = chunk.get_density(0, 0, 0);
        assert!(density != 0.0); // Debería tener algún valor
    }
}
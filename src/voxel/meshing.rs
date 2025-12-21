//! # Generacion de Mesh con Surface Nets
//! 
//! Implementa el algoritmo Surface Nets para generar meshes sueaves
//! a partir de campos de densidad. Este algoritmo:
//! 
//! 1. Encuentra celdas que cruzan la superficie (isosuperficie donde densidad = 0)
//! 2. Cloca un vertice en cada celda usando interpolacion de las aristas
//! 3. Conecta vertices adyacentes con quads para formar la supercifie

use super::{CHUNK_SIZE, Chunk, VOXEL_SIZE};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

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

    // Almacena índice del vértice generado en cada celda
    let mut vertex_indices = [[[-1i32; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE];

    // Paso 1: Generar vértices en celdas que cruzan la superficie
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if let Some(pos) = surface_net_vertex(chunk, x, y, z) {
                    vertex_indices[x][y][z] = positions.len() as i32;
                    positions.push(pos);

                    // Calcular normal por gradiente
                    let normal = calculate_normal(chunk, x, y, z);
                    normals.push(normal);
                }
            }
        }
    }

    // Paso 2: Conectar vértices con quads
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                if vertex_indices[x][y][z] < 0 {
                    continue;
                }

                // Conectar en X
                if x > 0 && y > 0 {
                    try_create_quad(
                        &vertex_indices,
                        &mut indices,
                        [x, y, z],
                        [x, y - 1, z],
                        [x - 1, y - 1, z],
                        [x - 1, y, z],
                        chunk.get_density(x, y, z) > 0.0,
                    );
                }

                // Conectar en Y
                if x > 0 && z > 0 {
                    try_create_quad(
                        &vertex_indices,
                        &mut indices,
                        [x, y, z],
                        [x - 1, y, z],
                        [x - 1, y, z - 1],
                        [x, y, z - 1],
                        chunk.get_density(x, y, z) > 0.0,
                    );
                }

                // Conectar en Z
                if y > 0 && z > 0 {
                    try_create_quad(
                        &vertex_indices,
                        &mut indices,
                        [x, y, z],
                        [x, y, z - 1],
                        [x, y - 1, z - 1],
                        [x, y - 1, z],
                        chunk.get_density(x, y, z) > 0.0,
                    );
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

/// Calcula la posición del vértice Surface Net para una celda.
/// 
/// Examina los 8 corners del cubo y, si la superficie cruza la celda,
/// calcula el punto promedio de todos los cruces en las aristas.
/// 
/// # Retorna
/// - `Some([x, y, z])`: Posición del vértice en coordenadas mundiales
/// - `None`: Si la celda está completamente dentro o fuera de la superficie
fn surface_net_vertex(chunk: &Chunk, x: usize, y: usize, z: usize) -> Option<[f32; 3]> {
    // Obtiene densidad de los 8 Corners de Cubo
    // Orden: 0-3 = cara inferior, 4-7 = cara superior  
    let corners = [
        chunk.get_density(x, y, z),
        chunk.get_density(x + 1, y, z),
        chunk.get_density(x + 1, y, z + 1),
        chunk.get_density(x, y, z + 1),
        chunk.get_density(x, y + 1, z),
        chunk.get_density(x + 1, y + 1, z),
        chunk.get_density(x + 1, y + 1, z + 1),
        chunk.get_density(x, y + 1, z + 1),
    ];

    // Contar cuántos están dentro/fuera de la superficie
    let mut inside = 0;
    let mut outside = 0;
    for &c in &corners {
        if c > 0.0 {
            inside += 1;
        } else {
            outside += 1;
        }
    }

    // Si todos dentro o todos fuera, no hay superficie
    if inside == 0 || outside == 0 {
        return None;
    }

    // Calcular posición promedio de cruces en las aristas
    let mut sum = Vec3::ZERO;
    let mut count = 0;

    let edges: [(usize, usize, Vec3, Vec3); 12] = [
        (0, 1, Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        (1, 2, Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 1.0)),
        (2, 3, Vec3::new(1.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 1.0)),
        (3, 0, Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0)),
        (4, 5, Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
        (5, 6, Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
        (6, 7, Vec3::new(1.0, 1.0, 1.0), Vec3::new(0.0, 1.0, 1.0)),
        (7, 4, Vec3::new(0.0, 1.0, 1.0), Vec3::new(0.0, 1.0, 0.0)),
        (0, 4, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
        (1, 5, Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)),
        (2, 6, Vec3::new(1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 1.0)),
        (3, 7, Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 1.0)),
    ];

    for (i0, i1, p0, p1) in edges {
        let d0 = corners[i0];
        let d1 = corners[i1];

        if (d0 > 0.0) != (d1 > 0.0) {
            let t = d0 / (d0 - d1);
            sum += p0.lerp(p1, t);
            count += 1;
        }
    }

    if count == 0 {
        return None;
    }

    let local_pos = sum / count as f32;

     // Agregar offset del chunk
    let chunk_offset = Vec3::new(
        chunk.position.x as f32 * CHUNK_SIZE as f32,
        chunk.position.y as f32 * CHUNK_SIZE as f32,
        chunk.position.z as f32 * CHUNK_SIZE as f32,
    );

    
    let world_pos = (chunk_offset + Vec3::new(x as f32, y as f32, z as f32) + local_pos) * VOXEL_SIZE;

    Some([world_pos.x, world_pos.y, world_pos.z])
}

/// Calcula la normal de la superficie usnado diferencias finitas del gradiente.
/// 
/// La normal apunta en direccion opuesta al gradiente de densidad. 
/// (desde solido hasta aire)
fn calculate_normal(chunk: &Chunk, x: usize, y: usize, z: usize) -> [f32; 3] {
    let d = 1;

    // Gradiente = diferencia de densidad en cada eje
    let dx = chunk.get_density((x + d).min(CHUNK_SIZE), y, z)
        - chunk.get_density(x.saturating_sub(d), y, z);
    let dy = chunk.get_density(x, (y + d).min(CHUNK_SIZE), z)
        - chunk.get_density(x, y.saturating_sub(d), z);
    let dz = chunk.get_density(x, y, (z + d).min(CHUNK_SIZE))
        - chunk.get_density(x, y, z.saturating_sub(d));

    let normal = Vec3::new(-dx, -dy, -dz).normalize_or_zero();
    [normal.x, normal.y, normal.z]
}

/// Intenta crear un quad (2 triangulos) entre 4 vertices.
/// 
/// # Parametros 
/// - 'vertex_indices': Mapa de coordenadas a indices de vertice
/// - 'indice': Vector de indices del mesh (se añaden aqui)
/// - a,b,c,d: Coordenadas de lo s4 vertices del quad
/// 'flip': Si es tru, invierte el winding order (para orientar correctamente.) 

fn try_create_quad(
    vertex_indices: &[[[i32; CHUNK_SIZE]; CHUNK_SIZE]; CHUNK_SIZE],
    indices: &mut Vec<u32>,
    a: [usize; 3],
    b: [usize; 3],
    c: [usize; 3],
    d: [usize; 3],
    flip: bool,
) {
    let ia = vertex_indices[a[0]][a[1]][a[2]];
    let ib = vertex_indices[b[0]][b[1]][b[2]];
    let ic = vertex_indices[c[0]][c[1]][c[2]];
    let id = vertex_indices[d[0]][d[1]][d[2]];

    if ia < 0 || ib < 0 || ic < 0 || id < 0 {
        return;
    }

    if flip {
        indices.extend_from_slice(&[ia as u32, ib as u32, ic as u32]);
        indices.extend_from_slice(&[ia as u32, ic as u32, id as u32]);
    } else {
        indices.extend_from_slice(&[ia as u32, ic as u32, ib as u32]);
        indices.extend_from_slice(&[ia as u32, id as u32, ic as u32]);
    }
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

    #[test]
    fn test_normal_computation() {
        let chunk = Chunk::new(IVec3::ZERO);
        let normal = calculate_normal(&chunk, 16, 16, 16);
        
        // La normal debe estar normalizada (longitud ≈ 1)
        let length = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
        assert!(
            (length - 1.0).abs() 

< 0.01 || length < 0.01,
            "Normal should be normalized or zero"
        );
    }
}
//! # Greedy Meshing Algorithm
//! 
//! Optimiza la generación de meshes combinando caras adyacentes del mismo material
//! en rectángulos más grandes. Reduce 70-95% de triángulos comparado con naive meshing.
//! 
//! ## Algoritmo:
//! 1. Para cada eje (X, Y, Z), procesar slices perpendiculares
//! 2. En cada slice, crear una máscara de caras visibles EN AMBAS DIRECCIONES
//! 3. Usar greedy algorithm para encontrar rectángulos máximos
//! 4. Generar quads en lugar de caras individuales

use bevy::prelude::*;
use bevy::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use crate::core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{BaseChunk, VoxelType, ChunkMap};

/// Genera mesh optimizado usando greedy meshing (versión simple sin vecinos)
/// 
/// Usado durante inicialización cuando no todos los chunks están cargados.
pub fn greedy_mesh_basechunk_simple(chunk: &BaseChunk) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Procesar cada eje (X, Y, Z) para greedy meshing
    for axis in 0..3 {
        for d in 0..BASE_CHUNK_SIZE {
            // Dirección positiva
            let mask_pos = generate_slice_mask_simple(chunk, axis, d, 1);
            greedy_mesh_slice(&mask_pos, chunk, axis, d, 1, &mut positions, &mut normals, &mut indices);
            
            // Dirección negativa
            let mask_neg = generate_slice_mask_simple(chunk, axis, d, -1);
            greedy_mesh_slice(&mask_neg, chunk, axis, d, -1, &mut positions, &mut normals, &mut indices);
        }
    }

    // Construir mesh final
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Genera máscara de caras visibles para un slice (versión simple sin vecinos)
fn generate_slice_mask_simple(
    chunk: &BaseChunk,
    axis: usize,
    d: usize,
    direction: i32, // +1 o -1
) -> Vec<Option<VoxelType>> {
    let u = (axis + 1) % 3;
    let v = (axis + 2) % 3;

    let mut mask = vec![None; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];

    for j in 0..BASE_CHUNK_SIZE {
        for i in 0..BASE_CHUNK_SIZE {
            let mut pos = [0; 3];
            pos[axis] = d;
            pos[u] = i;
            pos[v] = j;

            let x = pos[0];
            let y = pos[1];
            let z = pos[2];

            // Verificar si este voxel es sólido
            if chunk.get_density(x, y, z) <= 0.0 {
                continue;
            }

            // Verificar vecino en la dirección especificada
            let neighbor_x = x as i32 + if axis == 0 { direction } else { 0 };
            let neighbor_y = y as i32 + if axis == 1 { direction } else { 0 };
            let neighbor_z = z as i32 + if axis == 2 { direction } else { 0 };

            let is_face_visible = if neighbor_x < 0 || neighbor_y < 0 || neighbor_z < 0 ||
                                     neighbor_x >= BASE_CHUNK_SIZE as i32 ||
                                     neighbor_y >= BASE_CHUNK_SIZE as i32 ||
                                     neighbor_z >= BASE_CHUNK_SIZE as i32 {
                true // Borde del chunk
            } else {
                chunk.get_density(neighbor_x as usize, neighbor_y as usize, neighbor_z as usize) <= 0.0
            };

            if is_face_visible {
                mask[i + j * BASE_CHUNK_SIZE] = Some(chunk.voxel_types[x][y][z]);
            }
        }
    }

    mask
}

/// Genera mesh optimizado usando greedy meshing (con verificación de vecinos)
pub fn greedy_mesh_basechunk(
    chunk: &BaseChunk,
    chunk_map: &ChunkMap,
    chunks: &Query<&BaseChunk>,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Procesar cada eje (X, Y, Z) para greedy meshing
    for axis in 0..3 {
        for d in 0..BASE_CHUNK_SIZE {
            // Dirección positiva
            let mask_pos = generate_slice_mask(chunk, chunk_map, chunks, axis, d, 1);
            greedy_mesh_slice(&mask_pos, chunk, axis, d, 1, &mut positions, &mut normals, &mut indices);
            
            // Dirección negativa
            let mask_neg = generate_slice_mask(chunk, chunk_map, chunks, axis, d, -1);
            greedy_mesh_slice(&mask_neg, chunk, axis, d, -1, &mut positions, &mut normals, &mut indices);
        }
    }

    // Construir mesh final
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Genera máscara de caras visibles para un slice (con verificación de vecinos)
fn generate_slice_mask(
    chunk: &BaseChunk,
    chunk_map: &ChunkMap,
    chunks: &Query<&BaseChunk>,
    axis: usize,
    d: usize,
    direction: i32,
) -> Vec<Option<VoxelType>> {
    let u = (axis + 1) % 3;
    let v = (axis + 2) % 3;

    let mut mask = vec![None; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];

    for j in 0..BASE_CHUNK_SIZE {
        for i in 0..BASE_CHUNK_SIZE {
            let mut pos = [0; 3];
            pos[axis] = d;
            pos[u] = i;
            pos[v] = j;

            let x = pos[0];
            let y = pos[1];
            let z = pos[2];

            if chunk.get_density(x, y, z) <= 0.0 {
                continue;
            }

            // Calcular posición del vecino
            let neighbor_x = x as i32 + if axis == 0 { direction } else { 0 };
            let neighbor_y = y as i32 + if axis == 1 { direction } else { 0 };
            let neighbor_z = z as i32 + if axis == 2 { direction } else { 0 };

            let is_face_visible = if neighbor_x < 0 || neighbor_y < 0 || neighbor_z < 0 ||
                                     neighbor_x >= BASE_CHUNK_SIZE as i32 ||
                                     neighbor_y >= BASE_CHUNK_SIZE as i32 ||
                                     neighbor_z >= BASE_CHUNK_SIZE as i32 {
                // Fuera del chunk - verificar chunk vecino
                is_face_visible_cross_chunk(chunk, chunk_map, chunks, x, y, z, axis, direction)
            } else {
                // Dentro del chunk
                chunk.get_density(neighbor_x as usize, neighbor_y as usize, neighbor_z as usize) <= 0.0
            };

            if is_face_visible {
                mask[i + j * BASE_CHUNK_SIZE] = Some(chunk.voxel_types[x][y][z]);
            }
        }
    }

    mask
}

/// Aplica greedy meshing a un slice usando la máscara
fn greedy_mesh_slice(
    mask: &[Option<VoxelType>],
    chunk: &BaseChunk,
    axis: usize,
    d: usize,
    direction: i32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
) {
    let mut processed = vec![false; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];

    for j in 0..BASE_CHUNK_SIZE {
        for i in 0..BASE_CHUNK_SIZE {
            let idx = i + j * BASE_CHUNK_SIZE;

            if processed[idx] || mask[idx].is_none() {
                continue;
            }

            let voxel_type = mask[idx].unwrap();

            // Encontrar el rectángulo máximo
            let (width, height) = find_max_rect(mask, &mut processed, i, j, voxel_type);

            // Generar quad
            add_greedy_quad(
                chunk,
                axis,
                d,
                i,
                j,
                width,
                height,
                direction,
                positions,
                normals,
                indices,
            );
        }
    }
}

/// Encuentra el rectángulo máximo que comienza en (start_i, start_j)
fn find_max_rect(
    mask: &[Option<VoxelType>],
    processed: &mut [bool],
    start_i: usize,
    start_j: usize,
    voxel_type: VoxelType,
) -> (usize, usize) {
    // Expandir en dirección U (horizontal)
    let mut width = 1;
    while start_i + width < BASE_CHUNK_SIZE {
        let idx = (start_i + width) + start_j * BASE_CHUNK_SIZE;
        if processed[idx] || mask[idx] != Some(voxel_type) {
            break;
        }
        width += 1;
    }

    // Expandir en dirección V (vertical)
    let mut height = 1;
    'outer: while start_j + height < BASE_CHUNK_SIZE {
        for w in 0..width {
            let idx = (start_i + w) + (start_j + height) * BASE_CHUNK_SIZE;
            if processed[idx] || mask[idx] != Some(voxel_type) {
                break 'outer;
            }
        }
        height += 1;
    }

    // Marcar como procesados
    for h in 0..height {
        for w in 0..width {
            let idx = (start_i + w) + (start_j + h) * BASE_CHUNK_SIZE;
            processed[idx] = true;
        }
    }

    (width, height)
}

/// Agrega un quad optimizado al mesh
fn add_greedy_quad(
    chunk: &BaseChunk,
    axis: usize,
    d: usize,
    i: usize,
    j: usize,
    width: usize,
    height: usize,
    direction: i32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
) {
    let u = (axis + 1) % 3;
    let v = (axis + 2) % 3;

    // Posición base en coordenadas mundiales
    let mut base_pos = [0.0; 3];
    base_pos[axis] = (chunk.position[axis as usize] * BASE_CHUNK_SIZE as i32 + d as i32) as f32 * VOXEL_SIZE;
    base_pos[u] = (chunk.position[u as usize] * BASE_CHUNK_SIZE as i32 + i as i32) as f32 * VOXEL_SIZE;
    base_pos[v] = (chunk.position[v as usize] * BASE_CHUNK_SIZE as i32 + j as i32) as f32 * VOXEL_SIZE;

    // Dimensiones del quad
    let mut size = [0.0; 3];
    size[axis] = VOXEL_SIZE;
    size[u] = width as f32 * VOXEL_SIZE;
    size[v] = height as f32 * VOXEL_SIZE;

    // Ajustar posición según dirección
    if direction > 0 {
        base_pos[axis] += VOXEL_SIZE;
    }

    // Generar 4 vértices del quad
    let idx = positions.len() as u32;

    let v0 = base_pos;
    let mut v1 = base_pos;
    v1[u] += size[u];
    let mut v2 = base_pos;
    v2[u] += size[u];
    v2[v] += size[v];
    let mut v3 = base_pos;
    v3[v] += size[v];

    positions.push(v0);
    positions.push(v1);
    positions.push(v2);
    positions.push(v3);

    // Normal según dirección
    let mut normal = [0.0; 3];
    normal[axis] = direction as f32;
    normals.extend_from_slice(&[normal; 4]);

    // Índices (invertir winding si dirección negativa)
    if direction > 0 {
        indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
    } else {
        indices.extend_from_slice(&[idx, idx + 2, idx + 1, idx, idx + 3, idx + 2]);
    }
}

/// Verifica si una cara es visible en chunk vecino
fn is_face_visible_cross_chunk(
    chunk: &BaseChunk,
    chunk_map: &ChunkMap,
    chunks: &Query<&BaseChunk>,
    x: usize,
    y: usize,
    z: usize,
    axis: usize,
    direction: i32,
) -> bool {
    let mut neighbor_chunk_offset = IVec3::ZERO;
    neighbor_chunk_offset[axis as usize] = direction;

    let neighbor_chunk_pos = chunk.position + neighbor_chunk_offset;

    if let Some(&neighbor_entity) = chunk_map.chunks.get(&neighbor_chunk_pos) {
        if let Ok(neighbor_chunk) = chunks.get(neighbor_entity) {
            // Posición local en chunk vecino
            let local_x = if axis == 0 && direction < 0 { BASE_CHUNK_SIZE - 1 } else if axis == 0 && direction > 0 { 0 } else { x };
            let local_y = if axis == 1 && direction < 0 { BASE_CHUNK_SIZE - 1 } else if axis == 1 && direction > 0 { 0 } else { y };
            let local_z = if axis == 2 && direction < 0 { BASE_CHUNK_SIZE - 1 } else if axis == 2 && direction > 0 { 0 } else { z };

            return neighbor_chunk.get_density(local_x, local_y, local_z) <= 0.0;
        }
    }

    true // Sin chunk vecino, renderizar cara
}

/// Genera mesh usando greedy meshing para un DownsampledChunk
pub fn greedy_mesh_downsampled(chunk: &crate::voxel::DownsampledChunk) -> Mesh {
    let size = chunk.effective_size();
    let scale = chunk.downsample_factor as f32;
    
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    
    // Procesar cada eje (X, Y, Z) para greedy meshing
    for axis in 0..3 {
        for d in 0..size {
            // Dirección positiva
            let mask_pos = generate_downsampled_mask(chunk, axis, d, 1, size);
            greedy_mesh_downsampled_slice(&mask_pos, chunk, axis, d, 1, size, scale, &mut positions, &mut normals, &mut indices);
            
            // Dirección negativa
            let mask_neg = generate_downsampled_mask(chunk, axis, d, -1, size);
            greedy_mesh_downsampled_slice(&mask_neg, chunk, axis, d, -1, size, scale, &mut positions, &mut normals, &mut indices);
        }
    }
    
    // Construir mesh final
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Genera máscara para chunk downsampled
fn generate_downsampled_mask(
    chunk: &crate::voxel::DownsampledChunk,
    axis: usize,
    d: usize,
    direction: i32,
    size: usize,
) -> Vec<Option<VoxelType>> {
    let mut mask = vec![None; size * size];
    
    for i in 0..size {
        for j in 0..size {
            let (x, y, z) = match axis {
                0 => (d, i, j),
                1 => (i, d, j),
                _ => (i, j, d),
            };
            
            if x >= size || y >= size || z >= size {
                continue;
            }
            
            let current_voxel = chunk.voxel_types[x][y][z];
            
            if current_voxel == VoxelType::Air {
                continue;
            }
            
            // Verificar si la cara es visible
            let (nx, ny, nz) = match axis {
                0 => ((d as i32 + direction) as usize, i, j),
                1 => (i, (d as i32 + direction) as usize, j),
                _ => (i, j, (d as i32 + direction) as usize),
            };
            
            let neighbor_is_air = if nx >= size || ny >= size || nz >= size {
                true
            } else {
                chunk.voxel_types[nx][ny][nz] == VoxelType::Air
            };
            
            if neighbor_is_air {
                mask[i * size + j] = Some(current_voxel);
            }
        }
    }
    
    mask
}

/// Aplica greedy meshing a un slice downsampled
fn greedy_mesh_downsampled_slice(
    mask: &[Option<VoxelType>],
    chunk: &crate::voxel::DownsampledChunk,
    axis: usize,
    d: usize,
    direction: i32,
    size: usize,
    scale: f32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
) {
    let mut visited = vec![false; size * size];
    
    for i in 0..size {
        for j in 0..size {
            let idx = i * size + j;
            
            if visited[idx] || mask[idx].is_none() {
                continue;
            }
            
            let voxel_type = mask[idx].unwrap();
            
            // Encontrar rectángulo máximo
            let (width, height) = find_max_rect_downsampled(mask, &mut visited, i, j, voxel_type, size);
            
            // Agregar quad
            add_downsampled_quad(
                chunk, axis, d, i, j, width, height, direction, voxel_type, scale,
                positions, normals, indices
            );
        }
    }
}

/// Encuentra rectángulo máximo en chunk downsampled
fn find_max_rect_downsampled(
    mask: &[Option<VoxelType>],
    visited: &mut [bool],
    start_i: usize,
    start_j: usize,
    voxel_type: VoxelType,
    size: usize,
) -> (usize, usize) {
    let mut width = 1;
    while start_j + width < size {
        let idx = start_i * size + start_j + width;
        if visited[idx] || mask[idx] != Some(voxel_type) {
            break;
        }
        width += 1;
    }
    
    let mut height = 1;
    'outer: while start_i + height < size {
        for w in 0..width {
            let idx = (start_i + height) * size + start_j + w;
            if visited[idx] || mask[idx] != Some(voxel_type) {
                break 'outer;
            }
        }
        height += 1;
    }
    
    // Marcar como visitados
    for h in 0..height {
        for w in 0..width {
            visited[(start_i + h) * size + start_j + w] = true;
        }
    }
    
    (width, height)
}

/// Agrega quad para chunk downsampled
fn add_downsampled_quad(
    chunk: &crate::voxel::DownsampledChunk,
    axis: usize,
    d: usize,
    i: usize,
    j: usize,
    width: usize,
    height: usize,
    direction: i32,
    voxel_type: VoxelType,
    scale: f32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
) {
    let voxel_size = VOXEL_SIZE * scale;
    
    let base_idx = positions.len() as u32;
    
    let (v0, v1, v2, v3, normal) = match (axis, direction) {
        (0, 1) => {
            let x = (d + 1) as f32 * voxel_size;
            let y0 = i as f32 * voxel_size;
            let y1 = (i + height) as f32 * voxel_size;
            let z0 = j as f32 * voxel_size;
            let z1 = (j + width) as f32 * voxel_size;
            (
                [x, y0, z0], [x, y1, z0], [x, y1, z1], [x, y0, z1],
                [1.0, 0.0, 0.0]
            )
        },
        (0, -1) => {
            let x = d as f32 * voxel_size;
            let y0 = i as f32 * voxel_size;
            let y1 = (i + height) as f32 * voxel_size;
            let z0 = j as f32 * voxel_size;
            let z1 = (j + width) as f32 * voxel_size;
            (
                [x, y0, z1], [x, y1, z1], [x, y1, z0], [x, y0, z0],
                [-1.0, 0.0, 0.0]
            )
        },
        (1, 1) => {
            let y = (d + 1) as f32 * voxel_size;
            let x0 = i as f32 * voxel_size;
            let x1 = (i + height) as f32 * voxel_size;
            let z0 = j as f32 * voxel_size;
            let z1 = (j + width) as f32 * voxel_size;
            (
                [x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1],
                [0.0, 1.0, 0.0]
            )
        },
        (1, -1) => {
            let y = d as f32 * voxel_size;
            let x0 = i as f32 * voxel_size;
            let x1 = (i + height) as f32 * voxel_size;
            let z0 = j as f32 * voxel_size;
            let z1 = (j + width) as f32 * voxel_size;
            (
                [x0, y, z1], [x1, y, z1], [x1, y, z0], [x0, y, z0],
                [0.0, -1.0, 0.0]
            )
        },
        (2, 1) => {
            let z = (d + 1) as f32 * voxel_size;
            let x0 = i as f32 * voxel_size;
            let x1 = (i + height) as f32 * voxel_size;
            let y0 = j as f32 * voxel_size;
            let y1 = (j + width) as f32 * voxel_size;
            (
                [x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z],
                [0.0, 0.0, 1.0]
            )
        },
        _ => {
            let z = d as f32 * voxel_size;
            let x0 = i as f32 * voxel_size;
            let x1 = (i + height) as f32 * voxel_size;
            let y0 = j as f32 * voxel_size;
            let y1 = (j + width) as f32 * voxel_size;
            (
                [x0, y1, z], [x1, y1, z], [x1, y0, z], [x0, y0, z],
                [0.0, 0.0, -1.0]
            )
        },
    };
    
    positions.extend_from_slice(&[v0, v1, v2, v3]);
    normals.extend_from_slice(&[normal, normal, normal, normal]);
    indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3]);
}

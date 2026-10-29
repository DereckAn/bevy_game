//! # Greedy Meshing Algorithm
//!
//! Optimiza la generación de meshes combinando caras adyacentes del mismo material
//! en rectángulos más grandes. Reduce 70-95% de triángulos comparado con naive meshing.
//!
//! ## Algoritmo (binary greedy meshing):
//! 1. Codificar el chunk como columnas de bits por eje (32 voxeles = un `u32`),
//!    con el borde de los vecinos como bits de padding
//! 2. Caras visibles de una columna entera con una operación: `celda & !vecino`
//! 3. Repartir las caras en planos binarios por (profundidad, material)
//! 4. Greedy por plano con `trailing_zeros`/`trailing_ones` → quads

use crate::core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{voxel_color, BaseChunk, ChunkMap, VoxelType, VOXEL_TYPE_COUNT};
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

/// Mesh simple (sin vecinos), para RENDER inicial. Usado al arrancar cuando no
/// todos los chunks están cargados.
pub fn greedy_mesh_basechunk_simple(chunk: &BaseChunk) -> Mesh {
    greedy_mesh_with_borders(chunk, &NeighborBorders::default(), MeshLayer::Opaque)
}

/// Mesh simple del AGUA (sin vecinos), para los chunks del arranque. Igual que
/// `greedy_mesh_basechunk_water` pero sin consultar el `ChunkMap`, que en `setup`
/// todavía no existe.
pub fn greedy_mesh_basechunk_water_simple(chunk: &BaseChunk) -> Mesh {
    greedy_mesh_with_borders(chunk, &NeighborBorders::default(), MeshLayer::Water)
}

/// Mesh simple SOLO-COLISIONABLE (sin vecinos, ignora el follaje). Se usa para
/// construir el collider en el hilo de fondo: no necesita chunks vecinos, así que
/// puede correr dentro de la tarea async de generación. Las caras extra en los
/// bordes del chunk son inofensivas para la colisión.
pub fn greedy_mesh_basechunk_collider_simple(chunk: &BaseChunk) -> Mesh {
    greedy_mesh_with_borders(chunk, &NeighborBorders::default(), MeshLayer::Collider)
}

/// Capa de voxeles de cada uno de los 6 chunks vecinos que toca este chunk.
/// Es una COPIA, no una referencia al ECS: así el mallado con vecinos puede
/// correr en un hilo de fondo. `None` = vecino desconocido (no cargado o aún
/// generándose).
#[derive(Default)]
pub struct NeighborBorders([Option<Box<[VoxelType]>>; 6]);

impl NeighborBorders {
    /// Copia la capa de contacto de cada vecino que ya tenga voxeles en el ECS.
    pub fn gather(chunk_pos: IVec3, chunk_map: &ChunkMap, chunks: &Query<&BaseChunk>) -> Self {
        Self::from_neighbors(chunk_pos, |pos| {
            chunk_map
                .chunks
                .get(&pos)
                .and_then(|&entity| chunks.get(entity).ok())
        })
    }

    /// Como `gather`, pero con cualquier fuente de chunks (los tests no tienen ECS).
    fn from_neighbors<'a>(
        chunk_pos: IVec3,
        neighbor_at: impl Fn(IVec3) -> Option<&'a BaseChunk>,
    ) -> Self {
        let mut borders = Self::default();
        for axis in 0..3 {
            for direction in [-1, 1] {
                let mut offset = IVec3::ZERO;
                offset[axis] = direction;
                let Some(neighbor) = neighbor_at(chunk_pos + offset) else {
                    continue;
                };

                // El vecino en -dir nos toca con su última capa; el de +dir, con la primera.
                let d = if direction < 0 {
                    BASE_CHUNK_SIZE - 1
                } else {
                    0
                };
                let u = (axis + 1) % 3;
                let v = (axis + 2) % 3;
                let mut layer = vec![VoxelType::Air; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];
                for j in 0..BASE_CHUNK_SIZE {
                    for i in 0..BASE_CHUNK_SIZE {
                        let mut pos = [0; 3];
                        pos[axis] = d;
                        pos[u] = i;
                        pos[v] = j;
                        layer[i + j * BASE_CHUNK_SIZE] =
                            neighbor.voxel_types[pos[0]][pos[1]][pos[2]];
                    }
                }
                borders.0[face_index(axis, direction)] = Some(layer.into_boxed_slice());
            }
        }
        borders
    }

    /// Voxel al otro lado de la cara (axis, direction) en la celda (i, j) del slice.
    fn get(&self, axis: usize, direction: i32, i: usize, j: usize) -> Option<VoxelType> {
        self.0[face_index(axis, direction)]
            .as_ref()
            .map(|layer| layer[i + j * BASE_CHUNK_SIZE])
    }
}

fn face_index(axis: usize, direction: i32) -> usize {
    axis * 2 + (direction > 0) as usize
}

/// Qué capa de malla se está construyendo. El agua es translúcida, así que se
/// mallea aparte de la geometría opaca y con su propio material.
#[derive(Clone, Copy, PartialEq)]
pub enum MeshLayer {
    /// Render opaco: todo lo sólido MENOS el agua (el suelo se ve bajo el agua).
    Opaque,
    /// Render del agua: solo voxeles de agua, malla translúcida.
    Water,
    /// Colisión: solo lo colisionable (ignora follaje y agua).
    Collider,
}

/// ¿Pertenece este voxel a la malla de esta capa?
#[inline]
fn cell_in_layer(vt: VoxelType, layer: MeshLayer) -> bool {
    match layer {
        MeshLayer::Opaque => vt.is_solid() && !vt.is_water(),
        MeshLayer::Water => vt.is_water(),
        MeshLayer::Collider => vt.is_collidable(),
    }
}

/// ¿El vecino OCULTA la cara (no hay que dibujarla)? Para opaco/colisión, oculta
/// lo que pertenece a la misma capa. Para el agua, oculta TODO lo que no sea aire
/// (agua contra agua se fusiona; agua contra suelo la tapa el suelo opaco → se
/// descarta para no malgastar triángulos ni pelear el z-buffer).
#[inline]
fn face_hidden_by(vt: VoxelType, layer: MeshLayer) -> bool {
    match layer {
        MeshLayer::Water => !vt.is_air(),
        _ => cell_in_layer(vt, layer),
    }
}

/// ¿Se dibuja una cara contra un vecino DESCONOCIDO? Pasa cuando el vecino cae
/// fuera del chunk y no hay datos: mallado simple (sin vecinos) o chunk vecino
/// todavía generándose.
///
/// Opaco/colisión dibujan la cara: mejor una cara de más que un hueco en el suelo.
/// El agua la OMITE. Al dibujarla, los DOS chunks a cada lado del borde emiten un
/// quad en el mismo plano (ninguno se remalla cuando el otro carga), y dos quads
/// translúcidos coplanares producen una costura que parpadea. Omitirla acierta
/// casi siempre: bajo el nivel del mar el vecino es agua o terreno, no aire —
/// `chunk_is_above_terrain` ya impide que esos chunks se salten como vacíos.
#[inline]
fn face_visible_against_unknown(layer: MeshLayer) -> bool {
    layer != MeshLayer::Water
}

/// Mesh OPACO para RENDER (todo lo sólido menos el agua, incluido el follaje).
pub fn greedy_mesh_basechunk(
    chunk: &BaseChunk,
    chunk_map: &ChunkMap,
    chunks: &Query<&BaseChunk>,
) -> Mesh {
    let borders = NeighborBorders::gather(chunk.position, chunk_map, chunks);
    greedy_mesh_with_borders(chunk, &borders, MeshLayer::Opaque)
}

/// Mesh translúcido del AGUA (con verificación de vecinos, sin costuras entre chunks).
pub fn greedy_mesh_basechunk_water(
    chunk: &BaseChunk,
    chunk_map: &ChunkMap,
    chunks: &Query<&BaseChunk>,
) -> Mesh {
    let borders = NeighborBorders::gather(chunk.position, chunk_map, chunks);
    greedy_mesh_with_borders(chunk, &borders, MeshLayer::Water)
}

/// Greedy meshing contra una copia de los bordes vecinos. No toca el ECS, así que
/// puede correr en un hilo de fondo. `layer` decide qué voxeles entran (opaco /
/// agua / colisión).
pub fn greedy_mesh_with_borders(
    chunk: &BaseChunk,
    borders: &NeighborBorders,
    layer: MeshLayer,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();

    let (in_layer, hidden) = build_axis_columns(chunk, borders, layer);

    // Un plano de 32 filas por (profundidad d, material): la fila j guarda en el
    // bit i si hay cara visible de ese material en (i, j). Separar por material
    // permite fusionar con operaciones de bits sin mezclar colores. El greedy
    // vacía cada plano que procesa, así que se reutilizan entre ejes sin limpiar.
    let mut planes = vec![[0u32; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE * VOXEL_TYPE_COUNT];
    // Bit t de `used[d]` = el plano (d, material t) tiene caras: evita recorrer
    // los 24 planos vacíos de cada capa.
    let mut used = [0u32; BASE_CHUNK_SIZE];

    for axis in 0..3 {
        for direction in [1, -1] {
            collect_face_planes(
                chunk,
                &in_layer,
                &hidden,
                axis,
                direction,
                &mut planes,
                &mut used,
            );

            for d in 0..BASE_CHUNK_SIZE {
                let mut types = std::mem::take(&mut used[d]);
                while types != 0 {
                    let t = types.trailing_zeros() as usize;
                    types &= types - 1;
                    greedy_mesh_plane(
                        &mut planes[d * VOXEL_TYPE_COUNT + t],
                        VoxelType::from_u8(t as u8),
                        chunk,
                        axis,
                        d,
                        direction,
                        &mut positions,
                        &mut normals,
                        &mut indices,
                        &mut colors,
                    );
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Columnas binarias del chunk, una por (eje, celda i, celda j) en
/// `[axis * N² + i + j * N]`, con i/j sobre los ejes u/v. Bit k = celda k a lo
/// largo del eje.
///
/// - `in_layer` (u32): la celda pertenece a la malla de esta capa.
/// - `hidden` (u64, 34 bits): la celda TAPA una cara vecina. Va desplazada un
///   bit: el bit 0 y el 33 son el borde copiado de los chunks vecinos, así una
///   sola operación de bits resuelve también las caras del borde.
fn build_axis_columns(
    chunk: &BaseChunk,
    borders: &NeighborBorders,
    layer: MeshLayer,
) -> (Vec<u32>, Vec<u64>) {
    const N: usize = BASE_CHUNK_SIZE;
    let mut in_layer = vec![0u32; 3 * N * N];
    let mut hidden = vec![0u64; 3 * N * N];

    for x in 0..N {
        for y in 0..N {
            for z in 0..N {
                let vt = chunk.voxel_types[x][y][z];
                let is_in_layer = cell_in_layer(vt, layer);
                let is_hidden = face_hidden_by(vt, layer);
                if !is_in_layer && !is_hidden {
                    continue;
                }
                let pos = [x, y, z];
                for axis in 0..3 {
                    let col = axis * N * N + pos[(axis + 1) % 3] + pos[(axis + 2) % 3] * N;
                    if is_in_layer {
                        in_layer[col] |= 1 << pos[axis];
                    }
                    if is_hidden {
                        hidden[col] |= 1 << (pos[axis] + 1);
                    }
                }
            }
        }
    }

    for axis in 0..3 {
        for direction in [-1, 1] {
            let pad_bit = if direction < 0 { 1 } else { 1 << (N + 1) };
            for j in 0..N {
                for i in 0..N {
                    let is_hidden = match borders.get(axis, direction, i, j) {
                        Some(neighbor) => face_hidden_by(neighbor, layer),
                        None => !face_visible_against_unknown(layer),
                    };
                    if is_hidden {
                        hidden[axis * N * N + i + j * N] |= pad_bit;
                    }
                }
            }
        }
    }

    (in_layer, hidden)
}

/// Reparte las caras visibles de (axis, direction) en los planos por material.
fn collect_face_planes(
    chunk: &BaseChunk,
    in_layer: &[u32],
    hidden: &[u64],
    axis: usize,
    direction: i32,
    planes: &mut [[u32; BASE_CHUNK_SIZE]],
    used: &mut [u32; BASE_CHUNK_SIZE],
) {
    const N: usize = BASE_CHUNK_SIZE;
    let u = (axis + 1) % 3;
    let v = (axis + 2) % 3;

    for j in 0..N {
        for i in 0..N {
            let col = axis * N * N + i + j * N;
            // Bit k de `neighbor_hidden` = la celda vecina de k (k+1 o k-1) tapa
            // la cara. Con el desplazamiento de `hidden`, la celda k±1 vive en el
            // bit k+1±1: >> 2 para +dir, >> 0 para -dir.
            let neighbor_hidden = if direction > 0 {
                (hidden[col] >> 2) as u32
            } else {
                hidden[col] as u32
            };
            let mut faces = in_layer[col] & !neighbor_hidden;

            while faces != 0 {
                let d = faces.trailing_zeros() as usize;
                faces &= faces - 1;
                let mut pos = [0; 3];
                pos[axis] = d;
                pos[u] = i;
                pos[v] = j;
                let t = chunk.voxel_types[pos[0]][pos[1]][pos[2]] as usize;
                planes[d * VOXEL_TYPE_COUNT + t][j] |= 1 << i;
                used[d] |= 1 << t;
            }
        }
    }
}

/// Greedy sobre un plano binario de un solo material: toma el primer bit, lo
/// extiende en U con `trailing_ones` y luego en V mientras las filas siguientes
/// contengan el mismo tramo. Deja el plano vacío.
fn greedy_mesh_plane(
    plane: &mut [u32; BASE_CHUNK_SIZE],
    voxel_type: VoxelType,
    chunk: &BaseChunk,
    axis: usize,
    d: usize,
    direction: i32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    colors: &mut Vec<[f32; 4]>,
) {
    for j in 0..BASE_CHUNK_SIZE {
        while plane[j] != 0 {
            let i = plane[j].trailing_zeros() as usize;
            let width = (plane[j] >> i).trailing_ones() as usize;
            let run = (u32::MAX >> (32 - width)) << i;
            plane[j] &= !run;

            let mut height = 1;
            while j + height < BASE_CHUNK_SIZE && plane[j + height] & run == run {
                plane[j + height] &= !run;
                height += 1;
            }

            add_greedy_quad(
                chunk, axis, d, i, j, width, height, direction, voxel_type, positions, normals,
                indices, colors,
            );
        }
    }
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
    voxel_type: VoxelType,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    colors: &mut Vec<[f32; 4]>,
) {
    let u = (axis + 1) % 3;
    let v = (axis + 2) % 3;

    // Posición base en coordenadas mundiales
    let mut base_pos = [0.0; 3];
    base_pos[axis] =
        (chunk.position[axis as usize] * BASE_CHUNK_SIZE as i32 + d as i32) as f32 * VOXEL_SIZE;
    base_pos[u] =
        (chunk.position[u as usize] * BASE_CHUNK_SIZE as i32 + i as i32) as f32 * VOXEL_SIZE;
    base_pos[v] =
        (chunk.position[v as usize] * BASE_CHUNK_SIZE as i32 + j as i32) as f32 * VOXEL_SIZE;

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

    // Sombreado por orientación de cara (AO barato): la cima a brillo pleno, los
    // lados más oscuros y la base la más oscura. Da volumen aun sin sombras reales.
    let face_shade = match (axis, direction) {
        (1, 1) => 1.0,
        (1, -1) => 0.5,
        _ => 0.75,
    };

    // El agua se dibuja con su propio material translúcido (color y alpha vienen
    // del material, no del voxel), así que su vertex color es blanco opaco. El
    // resto usa `voxel_color`, cuyo alpha lleva el id para la paleta en GPU.
    let color = if voxel_type == VoxelType::Water {
        [face_shade, face_shade, face_shade, 1.0]
    } else {
        let c = voxel_color(voxel_type);
        [
            c[0] * face_shade,
            c[1] * face_shade,
            c[2] * face_shade,
            c[3],
        ]
    };
    colors.extend_from_slice(&[color; 4]);

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

/// Mesher ANTERIOR (celda a celda, máscara `Option<VoxelType>` por slice), tal
/// cual estaba antes del binary greedy meshing. Solo existe como oráculo: los
/// tests exigen que el mesher binario produzca exactamente los mismos quads.
#[cfg(test)]
mod reference {
    use super::*;

    pub(super) fn greedy_mesh(
        chunk: &BaseChunk,
        borders: &NeighborBorders,
        layer: MeshLayer,
    ) -> Mesh {
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut normals: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut colors: Vec<[f32; 4]> = Vec::new();

        // Procesar cada eje (X, Y, Z) para greedy meshing
        for axis in 0..3 {
            for d in 0..BASE_CHUNK_SIZE {
                // Dirección positiva
                let mask_pos = generate_slice_mask(chunk, borders, axis, d, 1, layer);
                greedy_mesh_slice(
                    &mask_pos,
                    chunk,
                    axis,
                    d,
                    1,
                    &mut positions,
                    &mut normals,
                    &mut indices,
                    &mut colors,
                );

                // Dirección negativa
                let mask_neg = generate_slice_mask(chunk, borders, axis, d, -1, layer);
                greedy_mesh_slice(
                    &mask_neg,
                    chunk,
                    axis,
                    d,
                    -1,
                    &mut positions,
                    &mut normals,
                    &mut indices,
                    &mut colors,
                );
            }
        }

        // Construir mesh final
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }

    /// Genera máscara de caras visibles para un slice (con verificación de vecinos)
    fn generate_slice_mask(
        chunk: &BaseChunk,
        borders: &NeighborBorders,
        axis: usize,
        d: usize,
        direction: i32,
        layer: MeshLayer,
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

                if !cell_in_layer(chunk.voxel_types[x][y][z], layer) {
                    continue;
                }

                // Calcular posición del vecino
                let neighbor_x = x as i32 + if axis == 0 { direction } else { 0 };
                let neighbor_y = y as i32 + if axis == 1 { direction } else { 0 };
                let neighbor_z = z as i32 + if axis == 2 { direction } else { 0 };

                let is_face_visible = if neighbor_x < 0
                    || neighbor_y < 0
                    || neighbor_z < 0
                    || neighbor_x >= BASE_CHUNK_SIZE as i32
                    || neighbor_y >= BASE_CHUNK_SIZE as i32
                    || neighbor_z >= BASE_CHUNK_SIZE as i32
                {
                    // Fuera del chunk - verificar el borde copiado del vecino
                    match borders.get(axis, direction, i, j) {
                        Some(neighbor) => !face_hidden_by(neighbor, layer),
                        None => face_visible_against_unknown(layer),
                    }
                } else {
                    // Dentro del chunk
                    !face_hidden_by(
                        chunk.voxel_types[neighbor_x as usize][neighbor_y as usize]
                            [neighbor_z as usize],
                        layer,
                    )
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
        colors: &mut Vec<[f32; 4]>,
    ) {
        let mut processed = vec![false; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];

        for j in 0..BASE_CHUNK_SIZE {
            for i in 0..BASE_CHUNK_SIZE {
                let idx = i + j * BASE_CHUNK_SIZE;

                if processed[idx] || mask[idx].is_none() {
                    continue;
                }

                let voxel_type = mask[idx].unwrap();
                let (width, height) = find_max_rect(mask, &mut processed, i, j, voxel_type);

                // Generar quad
                add_greedy_quad(
                    chunk, axis, d, i, j, width, height, direction, voxel_type, positions,
                    normals, indices, colors,
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::LazyLock;

    use bevy::mesh::VertexAttributeValues;

    use super::*;
    use crate::core::WorldKind;

    fn chunk_filled_with(vt: VoxelType) -> BaseChunk {
        BaseChunk {
            voxel_types: Box::new([[[vt; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]),
            position: IVec3::ZERO,
        }
    }

    /// Regresión: dos chunks de agua vecinos emitían cada uno un muro en el plano
    /// que comparten (ninguno se remalla cuando el otro carga), y dos quads
    /// translúcidos coplanares parpadean al mezclarse.
    #[test]
    fn water_emits_no_faces_against_an_unknown_neighbor() {
        let chunk = chunk_filled_with(VoxelType::Water);

        assert_eq!(
            greedy_mesh_basechunk_water_simple(&chunk).count_vertices(),
            0
        );
    }

    #[test]
    fn water_under_air_meshes_the_surface_as_one_merged_quad() {
        let mut chunk = chunk_filled_with(VoxelType::Water);
        for x in 0..BASE_CHUNK_SIZE {
            for z in 0..BASE_CHUNK_SIZE {
                chunk.voxel_types[x][BASE_CHUNK_SIZE - 1][z] = VoxelType::Air;
            }
        }

        // Un único quad (4 vértices): la capa de aire deja ver solo la superficie,
        // y el greedy la fusiona en un rectángulo de 32x32.
        assert_eq!(
            greedy_mesh_basechunk_water_simple(&chunk).count_vertices(),
            4
        );
    }

    #[test]
    fn opaque_still_draws_faces_against_an_unknown_neighbor() {
        let chunk = chunk_filled_with(VoxelType::Stone);

        // 6 caras de chunk, cada una fusionada en un quad de 4 vértices.
        assert_eq!(greedy_mesh_basechunk_simple(&chunk).count_vertices(), 24);
    }

    #[test]
    fn a_solid_neighbor_border_hides_the_shared_face() {
        let chunk = chunk_filled_with(VoxelType::Stone);
        let mut borders = NeighborBorders::default();
        borders.0[face_index(0, 1)] =
            Some(vec![VoxelType::Stone; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE].into_boxed_slice());

        // Sin la cara +X: quedan 5 quads.
        let mesh = greedy_mesh_with_borders(&chunk, &borders, MeshLayer::Opaque);

        assert_eq!(mesh.count_vertices(), 20);
    }

    #[test]
    fn faces_of_different_materials_are_not_merged() {
        let mut chunk = chunk_filled_with(VoxelType::Stone);
        for y in 0..BASE_CHUNK_SIZE {
            for z in 0..BASE_CHUNK_SIZE {
                chunk.voxel_types[BASE_CHUNK_SIZE / 2][y][z] = VoxelType::Dirt;
            }
        }

        // Una capa de tierra en x=16 parte en 3 (piedra | tierra | piedra) las 4
        // caras que la cruzan (±Y, ±Z); ±X siguen enteras → 2 + 4×3 = 14 quads.
        let mesh = greedy_mesh_basechunk_simple(&chunk);

        assert_eq!(mesh.count_vertices(), 56);
    }

    // --- Equivalencia con el mesher anterior (`reference`) ---

    /// Chunks reales (4 biomas, varias alturas) + uno sintético, cada uno sin
    /// vecinos y con los 6 bordes vecinos. Se generan una vez para los 3 tests.
    static EQUIVALENCE_CASES: LazyLock<Vec<(BaseChunk, NeighborBorders)>> = LazyLock::new(|| {
        let seed = 12345;
        let centers = [
            IVec3::new(0, 0, 0),
            IVec3::new(7, 1, -3),
            IVec3::new(-12, 2, 20),
            IVec3::new(3, -1, 9),
        ];
        let mut cases = Vec::new();
        for kind in [
            WorldKind::Normal,
            WorldKind::Desert,
            WorldKind::Ice,
            WorldKind::Mangrove,
        ] {
            for center in centers {
                cases.extend(cases_around(center, |pos| BaseChunk::new(pos, seed, kind)));
            }
        }
        cases.extend(cases_around(IVec3::new(2, 0, -5), mixed_material_chunk));
        cases
    });

    /// El chunk en `center` dos veces: sin vecinos (bordes desconocidos) y con
    /// los bordes de sus 6 vecinos.
    fn cases_around(
        center: IVec3,
        make: impl Fn(IVec3) -> BaseChunk,
    ) -> [(BaseChunk, NeighborBorders); 2] {
        let neighbors: HashMap<IVec3, BaseChunk> = [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ]
        .map(|offset| (center + offset, make(center + offset)))
        .into_iter()
        .collect();
        let borders = NeighborBorders::from_neighbors(center, |pos| neighbors.get(&pos));
        let chunk = make(center);
        [
            (chunk.clone(), NeighborBorders::default()),
            (chunk, borders),
        ]
    }

    /// Bloques de 4³ con material pseudoaleatorio entre los 24 tipos (aire y agua
    /// incluidos): fuerza fusiones, cortes entre materiales y caras agua/sólido
    /// que el terreno real produce poco.
    fn mixed_material_chunk(position: IVec3) -> BaseChunk {
        let mut chunk = chunk_filled_with(VoxelType::Air);
        chunk.position = position;
        let origin = position * BASE_CHUNK_SIZE as i32;
        for x in 0..BASE_CHUNK_SIZE {
            for y in 0..BASE_CHUNK_SIZE {
                for z in 0..BASE_CHUNK_SIZE {
                    let local = IVec3::new(x as i32, y as i32, z as i32);
                    let block = (origin + local).div_euclid(IVec3::splat(4));
                    let hash = block.x.wrapping_mul(73_856_093)
                        ^ block.y.wrapping_mul(19_349_663)
                        ^ block.z.wrapping_mul(83_492_791);
                    let t = hash.rem_euclid(VOXEL_TYPE_COUNT as i32) as u8;
                    chunk.voxel_types[x][y][z] = VoxelType::from_u8(t);
                }
            }
        }
        chunk
    }

    /// Quads de la malla como claves comparables (bits exactos de posiciones,
    /// normales y colores + índices locales), ordenados: el mesher binario emite
    /// los mismos quads en otro orden.
    fn sorted_quads(mesh: &Mesh) -> Vec<Vec<u32>> {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("mesh sin posiciones");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("mesh sin normales");
        };
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("mesh sin colores");
        };
        let Some(Indices::U32(indices)) = mesh.indices() else {
            panic!("mesh sin índices u32");
        };

        let mut quads: Vec<Vec<u32>> = (0..positions.len() / 4)
            .map(|q| {
                let vertices = 4 * q..4 * q + 4;
                let floats = vertices
                    .flat_map(|v| positions[v].into_iter().chain(normals[v]).chain(colors[v]));
                let local_indices = indices[6 * q..6 * q + 6].iter().map(|&i| i - 4 * q as u32);
                floats.map(f32::to_bits).chain(local_indices).collect()
            })
            .collect();
        quads.sort();
        quads
    }

    /// Primer caso cuya malla binaria difiere de la de referencia, o `None`.
    fn first_mismatch_with_reference(layer: MeshLayer) -> Option<String> {
        EQUIVALENCE_CASES.iter().find_map(|(chunk, borders)| {
            let expected = sorted_quads(&reference::greedy_mesh(chunk, borders, layer));
            let actual = sorted_quads(&greedy_mesh_with_borders(chunk, borders, layer));
            (expected != actual).then(|| {
                let known = borders.0.iter().filter(|b| b.is_some()).count();
                format!(
                    "chunk {:?} ({known} vecinos): {} quads esperados, {} obtenidos",
                    chunk.position,
                    expected.len(),
                    actual.len()
                )
            })
        })
    }

    #[test]
    fn binary_mesher_matches_reference_for_opaque_layer() {
        assert_eq!(first_mismatch_with_reference(MeshLayer::Opaque), None);
    }

    #[test]
    fn binary_mesher_matches_reference_for_water_layer() {
        assert_eq!(first_mismatch_with_reference(MeshLayer::Water), None);
    }

    #[test]
    fn binary_mesher_matches_reference_for_collider_layer() {
        assert_eq!(first_mismatch_with_reference(MeshLayer::Collider), None);
    }
}

//! Sistema de chunks LOD para renderizaqdo a distancia
//! Similar a Distan Horizons - solo almacena superficie, sin colision
//!

use crate::{
    core::VOXEL_SIZE,
    voxel::{TerrainGenerator, VoxelType, voxel_color},
};
use bevy::{
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// Nivel de detalle para chunks distantes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodLevel {
    Medium,
    Low,
    Minimal,
}

impl LodLevel {
    /// Obtiene el tamano de la grilla para este nivel LOD
    pub fn grid_size(&self) -> usize {
        match self {
            LodLevel::Medium => 16,
            LodLevel::Low => 8,
            LodLevel::Minimal => 4,
        }
    }

    /// Determina si nivel LOD basado en distancia en chunks
    pub fn from_distance(distance_chunks: i32) -> Self {
        match distance_chunks {
            d if d < 64 => LodLevel::Medium,
            d if d < 128 => LodLevel::Low,
            _ => LodLevel::Minimal,
        }
    }
}

/// Chunk LOD que solo almacena la superficie del terreno
/// Mucho mas eficiente que un chunk completo
#[derive(Component, Clone)]
pub struct LodChunk {
    // Posicion del chunk en coordenads del chunk
    pub position: IVec3,

    // Nivel de detalle de este chunk
    pub lod_level: LodLevel,

    // Altura de la superficie en cada punto del grid
    // Solo almacenamos Y, no todo el volumen 3d
    pub surface_heights: Vec<f32>,

    // Tipo de voxel en la superficie
    pub surface_types: Vec<VoxelType>,
}

impl LodChunk {
    // Crea un nuevo chunk LOD generando solo la superficie
    pub fn new(position: IVec3, lod_level: LodLevel) -> Self {
        let grid_size = lod_level.grid_size();
        let total_points = grid_size * grid_size; // solo eje X Z, no Y

        Self {
            position,
            lod_level,
            surface_heights: vec![0.0; total_points],
            surface_types: vec![VoxelType::Air; total_points],
        }
    }

    /// Genera la superficie del terreno para este chunk LOD
    pub fn generate_surface(&mut self, terrain_gen: &mut TerrainGenerator) {
        let grid_size = self.lod_level.grid_size();

        // Calcular cuántos voxels del chunk real representa cada punto LOD
        let step_size = 32 / grid_size;

        // Recorrer cada punto del grid en X y Z
        for z in 0..grid_size {
            for x in 0..grid_size {
                // Calcular posición mundial del CENTRO de este "super-voxel"
                // Esto asegura mejor muestreo
                let local_x = (x * step_size + step_size / 2) as i32;
                let local_z = (z * step_size + step_size / 2) as i32;

                let world_x = (self.position.x * 32 + local_x) as f32 * 0.1;
                let world_z = (self.position.z * 32 + local_z) as f32 * 0.1;

                // La densidad es `generate_height(x,z) - y` (monótona en Y),
                // así que la superficie ES la altura del terreno directamente:
                // una sola evaluación de noise en vez de binary search.
                let surface_y = terrain_gen.biome_gen.generate_height(world_x, world_z);

                // Guardar la altura de la superficie
                let index = x + z * grid_size;
                self.surface_heights[index] = surface_y;

                // Determinar el tipo de voxel en la superficie
                // La superficie siempre es pasto (profundidad 0), igual que BaseChunk
                self.surface_types[index] = VoxelType::from_depth(1.0, 0.0);
            }
        }
    }
}

// Genera un mesh para renderizar el chunk LOD
// Incluye cara superior y caras laterales para verse bien desde cualquier angulo
pub fn mesh_lod_chunk(lod_chunk: &LodChunk) -> Mesh {
    let grid_size = lod_chunk.lod_level.grid_size();
    let step_size = 32 / grid_size;
    let voxel_step = step_size as f32 * VOXEL_SIZE;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Offset base del chunk en el mundo
    let chunk_offset_x = lod_chunk.position.x as f32 * 32.0 * VOXEL_SIZE;
    let chunk_offset_z = lod_chunk.position.z as f32 * 32.0 * VOXEL_SIZE;

    // Generar quads para cada punto del grid
    for z in 0..grid_size {
        for x in 0..grid_size {
            let index = x + z * grid_size;
            let height = lod_chunk.surface_heights[index];

            // Posicion de esta columna
            let pos_x = chunk_offset_x + x as f32 * voxel_step;
            let pos_z = chunk_offset_z + z as f32 * voxel_step;

            // Color del terreno para esta celda, muestreado en su centro en
            // COORDENADAS MUNDIALES: es el mismo campo de ruido que usan los
            // chunks reales, así el color del LOD encaja con el terreno cercano.
            let base = voxel_color(
                lod_chunk.surface_types[index],
                pos_x + voxel_step * 0.5,
                height,
                pos_z + voxel_step * 0.5,
                0.0,
            );
            // Sombreado por cara igual que el greedy mesher: cima a brillo pleno,
            // lados al 75% para dar volumen.
            let side_color = [base[0] * 0.75, base[1] * 0.75, base[2] * 0.75, base[3]];

            // Cara superior
            add_top_face(
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
                pos_x,
                height,
                pos_z,
                voxel_step,
                base,
            );

            // --- CARAS LATERALES ---
            // Solo renderizar caras que están en el borde o tienen vecinos más bajos

            // Cara -X (izquierda)
            if x == 0 || lod_chunk.surface_heights[index - 1] < height {
                let neighbor_height = if x == 0 {
                    0.0
                } else {
                    lod_chunk.surface_heights[index - 1]
                };
                add_side_face(
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                    pos_x,
                    neighbor_height,
                    height,
                    pos_z,
                    pos_z + voxel_step,
                    [-1.0, 0.0, 0.0], // Normal apuntando a -X
                    side_color,
                );
            }

            // Cara +X (derecha)
            if x == grid_size - 1 || lod_chunk.surface_heights[index + 1] < height {
                let neighbor_height = if x == grid_size - 1 {
                    0.0
                } else {
                    lod_chunk.surface_heights[index + 1]
                };
                add_side_face(
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                    pos_x + voxel_step,
                    neighbor_height,
                    height,
                    pos_z,
                    pos_z + voxel_step,
                    [1.0, 0.0, 0.0], // Normal apuntando a +X
                    side_color,
                );
            }

            // Cara -Z (atrás)
            if z == 0 || lod_chunk.surface_heights[index - grid_size] < height {
                let neighbor_height = if z == 0 {
                    0.0
                } else {
                    lod_chunk.surface_heights[index - grid_size]
                };
                add_side_face(
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                    pos_x,
                    neighbor_height,
                    height,
                    pos_z,
                    pos_z,
                    [0.0, 0.0, -1.0], // Normal apuntando a -Z
                    side_color,
                );
            }

            // Cara +Z (adelante)
            if z == grid_size - 1 || lod_chunk.surface_heights[index + grid_size] < height {
                let neighbor_height = if z == grid_size - 1 {
                    0.0
                } else {
                    lod_chunk.surface_heights[index + grid_size]
                };
                add_side_face(
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                    pos_x,
                    neighbor_height,
                    height,
                    pos_z + voxel_step,
                    pos_z + voxel_step,
                    [0.0, 0.0, 1.0], // Normal apuntando a +Z
                    side_color,
                );
            }
        }
    }

    // Construir el mesh
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Agrega una cara superior (horizontal)
fn add_top_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    x: f32,
    y: f32,
    z: f32,
    size: f32,
    color: [f32; 4],
) {
    let base_idx = positions.len() as u32;

    // 4 vértices del quad superior
    positions.push([x, y, z]);
    positions.push([x + size, y, z]);
    positions.push([x + size, y, z + size]);
    positions.push([x, y, z + size]);

    // Normal apuntando hacia arriba
    normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
    colors.extend_from_slice(&[color; 4]);

    // 2 triángulos (winding CCW visto desde +Y, para backface culling)
    indices.extend_from_slice(&[
        base_idx,
        base_idx + 2,
        base_idx + 1,
        base_idx,
        base_idx + 3,
        base_idx + 2,
    ]);
}
/// Agrega una cara lateral (vertical)
fn add_side_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    x: f32,
    y_bottom: f32,
    y_top: f32,
    z_start: f32,
    z_end: f32,
    normal: [f32; 3],
    color: [f32; 4],
) {
    let base_idx = positions.len() as u32;

    // 4 vértices del quad lateral
    positions.push([x, y_bottom, z_start]);
    positions.push([x, y_bottom, z_end]);
    positions.push([x, y_top, z_end]);
    positions.push([x, y_top, z_start]);

    // Normal de la cara
    normals.extend_from_slice(&[normal; 4]);
    colors.extend_from_slice(&[color; 4]);

    // 2 triángulos. El orden de vértices produce winding frontal hacia el eje
    // negativo; para caras que miran al eje positivo hay que invertirlo
    // (necesario con backface culling activo).
    if normal[0] > 0.0 || normal[2] > 0.0 {
        indices.extend_from_slice(&[
            base_idx,
            base_idx + 2,
            base_idx + 1,
            base_idx,
            base_idx + 3,
            base_idx + 2,
        ]);
    } else {
        indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx,
            base_idx + 2,
            base_idx + 3,
        ]);
    }
}
//! Sistema de chunks LOD para renderizaqdo a distancia
//! Similar a Distan Horizons - solo almacena superficie, sin colision
//!

use crate::{
    core::{WorldKind, VOXEL_SIZE},
    voxel::{voxel_color, TerrainGenerator, VoxelType},
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
    /// Obtiene el tamano de la grilla para este nivel LOD.
    ///
    /// DEBE dividir a 32 (tamaño del chunk en voxels): 1, 2, 4, 8, 16. Si no, las
    /// celdas no cubren el chunk y aparecen huecos en los bordes. Los triángulos
    /// por chunk son `grid_size² * 2`; como los LOD están a 100 m+, una grilla
    /// baja es imperceptible pero recorta mucha geometría.
    pub fn grid_size(&self) -> usize {
        match self {
            LodLevel::Medium => 8,
            LodLevel::Low => 4,
            LodLevel::Minimal => 2,
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
                self.surface_types[index] =
                    VoxelType::from_depth(1.0, 0.0, terrain_gen.biome_gen.kind());
            }
        }
    }
}

// Genera un mesh para renderizar el chunk LOD
// Incluye cara superior y caras laterales para verse bien desde cualquier angulo
pub fn mesh_lod_chunk(lod_chunk: &LodChunk, seed: i32, kind: WorldKind) -> Mesh {
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

    // Alturas en las ESQUINAS del grid ((grid+1)²), muestreadas del MISMO campo de
    // ruido que los chunks reales. Al caer en coordenadas de mundo exactas, dos
    // chunks LOD vecinos comparten la altura de su borde → la superficie es
    // CONTINUA y estanca. Antes cada celda era una loseta horizontal plana con
    // faldones solo cuesta abajo: de canto (mirando montañas lejanas) las losetas
    // quedaban de perfil y los faldones de una sola cara se descartaban por
    // backface culling → se veía a través como una telaraña. Con esquinas
    // compartidas la malla se inclina siguiendo el terreno y no necesita faldones
    // para cerrarse entre LODs (los de abajo son para el borde con el lejano).
    let corners = grid_size + 1;
    let mut terrain_gen = TerrainGenerator::new(seed, kind);
    let mut corner_h = vec![0.0_f32; corners * corners];
    for cz in 0..corners {
        for cx in 0..corners {
            let world_x = chunk_offset_x + cx as f32 * voxel_step;
            let world_z = chunk_offset_z + cz as f32 * voxel_step;
            corner_h[cx + cz * corners] = terrain_gen.biome_gen.generate_height(world_x, world_z);
        }
    }

    // Un quad INCLINADO por celda, usando sus 4 alturas de esquina.
    for z in 0..grid_size {
        for x in 0..grid_size {
            let index = x + z * grid_size;

            let h00 = corner_h[x + z * corners];
            let h10 = corner_h[(x + 1) + z * corners];
            let h11 = corner_h[(x + 1) + (z + 1) * corners];
            let h01 = corner_h[x + (z + 1) * corners];

            let x0 = chunk_offset_x + x as f32 * voxel_step;
            let x1 = x0 + voxel_step;
            let z0 = chunk_offset_z + z as f32 * voxel_step;
            let z1 = z0 + voxel_step;

            let p0 = Vec3::new(x0, h00, z0);
            let p1 = Vec3::new(x1, h10, z0);
            let p2 = Vec3::new(x1, h11, z1);
            let p3 = Vec3::new(x0, h01, z1);

            // Mismo material de paleta que el terreno cercano: el shader pinta el
            // pasto desde la posición mundial, así encaja sin costura de color.
            let color = voxel_color(lod_chunk.surface_types[index]);

            // Dos triángulos (winding CCW visto desde +Y → cara frontal hacia arriba).
            add_terrain_tri(
                p0,
                p2,
                p1,
                color,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
            );
            add_terrain_tri(
                p0,
                p3,
                p2,
                color,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
            );
        }
    }

    // Faldones hacia FUERA en los 4 bordes, colgando bajo la arista compartida.
    // Entre dos LOD quedan bajo la superficie del vecino (que arranca en la misma
    // arista) y no se ven; donde el vecino lo dibuja el terreno lejano (columna
    // aún cargando, rodeada de LODs), cierran el escalón si el lejano queda más
    // bajo. Miran hacia fuera: desde dentro del campo cercano se descartan por
    // backface culling.
    let last = grid_size;
    let corner = |cx: usize, cz: usize| {
        Vec3::new(
            chunk_offset_x + cx as f32 * voxel_step,
            corner_h[cx + cz * corners],
            chunk_offset_z + cz as f32 * voxel_step,
        )
    };
    for i in 0..grid_size {
        let edges = [
            (corner(i, 0), corner(i + 1, 0), Vec3::NEG_Z, i),
            (corner(i, last), corner(i + 1, last), Vec3::Z, i + (last - 1) * grid_size),
            (corner(0, i), corner(0, i + 1), Vec3::NEG_X, i * grid_size),
            (corner(last, i), corner(last, i + 1), Vec3::X, (last - 1) + i * grid_size),
        ];
        for (a, b, outward, cell) in edges {
            let color = voxel_color(lod_chunk.surface_types[cell]);
            add_skirt_quad(a, b, outward, color, &mut positions, &mut normals, &mut colors, &mut indices);
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

/// Agrega un triángulo del terreno LOD con su normal de cara. Los vértices llegan
/// en winding CCW visto desde +Y (cara frontal hacia arriba), así el backface
/// culling conserva la superficie vista desde arriba/de canto.
fn add_terrain_tri(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    // Normal de la cara según el winding; para una superficie de altura apunta
    // hacia arriba y da relieve real a los planos inclinados.
    let n = (v1 - v0).cross(v2 - v0).normalize_or_zero();
    let base_idx = positions.len() as u32;
    positions.extend_from_slice(&[[v0.x, v0.y, v0.z], [v1.x, v1.y, v1.z], [v2.x, v2.y, v2.z]]);
    normals.extend_from_slice(&[[n.x, n.y, n.z]; 3]);
    colors.extend_from_slice(&[color; 3]);
    indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
}

/// Profundidad (m) de los faldones de borde. Junto a una arista LOD, el tope de
/// una columna lejana difiere del terreno real en ≤ `LOD_DROP` (0.3) + redondeo
/// (0.2) + pendiente × media celda (0.2 m): 4 m cubren pendientes de hasta ~17:1.
const LOD_SKIRT_DEPTH_M: f32 = 4.0;

/// Quad vertical de `a`–`b` hacia abajo `LOD_SKIRT_DEPTH_M`, con la cara frontal
/// hacia `outward`.
#[allow(clippy::too_many_arguments)]
fn add_skirt_quad(
    a: Vec3,
    b: Vec3,
    outward: Vec3,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let down = Vec3::Y * LOD_SKIRT_DEPTH_M;
    let (p, q) = if (b - a).cross(-down).dot(outward) >= 0.0 { (a, b) } else { (b, a) };
    add_terrain_tri(p, q, p - down, color, positions, normals, colors, indices);
    add_terrain_tri(q, q - down, p - down, color, positions, normals, colors, indices);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Normales de los triángulos de un faldón de `a`–`b` hacia `outward`.
    fn skirt_normals(a: Vec3, b: Vec3, outward: Vec3) -> Vec<Vec3> {
        let (mut p, mut n, mut c, mut i) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        add_skirt_quad(a, b, outward, [1.0; 4], &mut p, &mut n, &mut c, &mut i);
        n.into_iter().map(Vec3::from).collect()
    }

    #[test]
    fn skirt_faces_outward() {
        let normals = skirt_normals(Vec3::new(0.0, 5.0, 0.0), Vec3::new(1.0, 6.0, 0.0), Vec3::NEG_Z);
        assert!(normals.iter().all(|n| n.dot(Vec3::NEG_Z) > 0.99));
    }

    #[test]
    fn skirt_faces_outward_whatever_the_edge_direction() {
        let normals = skirt_normals(Vec3::new(1.0, 6.0, 0.0), Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Z);
        assert!(normals.iter().all(|n| n.dot(Vec3::NEG_Z) > 0.99));
    }

}

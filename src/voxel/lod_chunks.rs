//! Sistema de chunks LOD para renderizaqdo a distancia
//! Similar a Distan Horizons - solo almacena superficie, sin colision
//!

use crate::{
    core::VOXEL_SIZE,
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
pub fn mesh_lod_chunk(lod_chunk: &LodChunk, seed: i32) -> Mesh {
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

    // Impostores de arboles distantes. Solo pinos y solo en los
    // niveles LOD mas cercanos (Medium y Low). Diujar arboles en todo el alcance
    // LOD (~200 chunks) dispararia el conteo; minimal (128) va sin arboles.
    if matches!(lod_chunk.lod_level, LodLevel::Medium | LodLevel::Low) {
        add_tree_impostors(
            lod_chunk,
            seed,
            &mut positions,
            &mut normals,
            &mut colors,
            &mut indices,
        );
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

/// Lados del cono/bola de la copa del impostor. 8 se ve suave a distancia LOD.
const IMPOSTOR_SIDES: usize = 8;

/// Color RGBA lineal de un tipo de voxel (igual que la rama no-pasto de
/// `voxel_color`): el alpha lleva el discriminante de `VoxelType` para que el
/// shader de paleta aplique la variación tonal (troncos/copas de los impostores).
fn linear_rgba(voxel_type: VoxelType) -> [f32; 4] {
    let l = voxel_type.properties().color.to_linear();
    [l.red, l.green, l.blue, voxel_type as u8 as f32 / 255.0]
}

/// Punto `i` de un anillo horizontal de `sides` lados, radio `radius`, centrado en `center`.
fn ring_point(center: Vec3, radius: f32, i: usize, sides: usize) -> Vec3 {
    let a = std::f32::consts::TAU * i as f32 / sides as f32;
    Vec3::new(
        center.x + radius * a.cos(),
        center.y,
        center.z + radius * a.sin(),
    )
}

/// Añade un triángulo con normal plana, garantizando que su cara FRONTAL mire
/// hacia afuera (lejos de `interior`). Así no importa el orden de los vértices:
/// el winding se corrige para que el backface culling no lo oculte.
fn add_tri_outward(
    a: Vec3,
    b: Vec3,
    c: Vec3,
    interior: Vec3,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let centroid = (a + b + c) / 3.0;
    let outward = (centroid - interior).normalize_or_zero();
    let mut n = (b - a).cross(c - a).normalize_or_zero();

    // Si la normal apunta hacia adentro, invertir winding y normal.
    let (v0, v1, v2) = if n.dot(outward) >= 0.0 {
        (a, b, c)
    } else {
        n = -n;
        (a, c, b)
    };

    // Volumen barato: caras que miran hacia arriba más claras; lados y bajo, más
    // oscuros. Da relieve al impostor sin luces reales.
    let shade = 0.6 + 0.4 * (n.y * 0.5 + 0.5);
    let shaded = [
        color[0] * shade,
        color[1] * shade,
        color[2] * shade,
        color[3],
    ];

    let base = positions.len() as u32;
    positions.extend_from_slice(&[[v0.x, v0.y, v0.z], [v1.x, v1.y, v1.z], [v2.x, v2.y, v2.z]]);
    normals.extend_from_slice(&[[n.x, n.y, n.z]; 3]);
    colors.extend_from_slice(&[shaded; 3]);
    indices.extend_from_slice(&[base, base + 1, base + 2]);
}

/// Cono: abanico de triángulos desde un anillo de `IMPOSTOR_SIDES` lados hacia `apex`.
fn add_cone(
    ring_center: Vec3,
    radius: f32,
    apex: Vec3,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    for i in 0..IMPOSTOR_SIDES {
        let p0 = ring_point(ring_center, radius, i, IMPOSTOR_SIDES);
        let p1 = ring_point(
            ring_center,
            radius,
            (i + 1) % IMPOSTOR_SIDES,
            IMPOSTOR_SIDES,
        );
        add_tri_outward(
            p0,
            p1,
            apex,
            ring_center,
            color,
            positions,
            normals,
            colors,
            indices,
        );
    }
}

/// Caja de 6 caras centrada en `center` con semiejes `half`. `add_tri_outward`
/// corrige el winding, así las 6 caras miran hacia afuera.
fn add_box(
    center: Vec3,
    half: Vec3,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let (hx, hy, hz) = (half.x, half.y, half.z);
    let v = [
        center + Vec3::new(-hx, -hy, -hz),
        center + Vec3::new(hx, -hy, -hz),
        center + Vec3::new(hx, -hy, hz),
        center + Vec3::new(-hx, -hy, hz),
        center + Vec3::new(-hx, hy, -hz),
        center + Vec3::new(hx, hy, -hz),
        center + Vec3::new(hx, hy, hz),
        center + Vec3::new(-hx, hy, hz),
    ];
    let faces = [
        [0, 1, 2, 3], // abajo
        [4, 5, 6, 7], // arriba
        [0, 1, 5, 4], // -Z
        [2, 3, 7, 6], // +Z
        [1, 2, 6, 5], // +X
        [3, 0, 4, 7], // -X
    ];
    for f in faces {
        add_tri_outward(
            v[f[0]], v[f[1]], v[f[2]], center, color, positions, normals, colors, indices,
        );
        add_tri_outward(
            v[f[0]], v[f[2]], v[f[3]], center, color, positions, normals, colors, indices,
        );
    }
}

/// Añade un impostor de pino barato (tronco tipo prisma + 3 conos apilados) a
/// los buffers del mesh, en `base` (nivel del suelo) y altura `height_m` metros.
/// Unos pocos triángulos por árbol, sin detalle por voxel. Usa los mismos tipos
/// de voxel que el pino real → el color combina al convertirse LOD → Real.
fn add_pine_impostor(
    base: Vec3,
    height_m: f32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let trunk_color = linear_rgba(VoxelType::Wood);
    let canopy_color = linear_rgba(VoxelType::PineNeedles);

    // --- Tronco: prisma cuadrado delgado en el tercio inferior ---
    let trunk_h = height_m * 0.35;
    let trunk_r = (height_m * 0.03).max(0.05);
    add_trunk_prism(
        base,
        trunk_r,
        trunk_h,
        trunk_color,
        positions,
        normals,
        colors,
        indices,
    );

    // --- Copa: 3 conos apilados (base ancha → punta) = silueta de conífera ---
    // (base_frac, radio_frac, apex_frac) como fracción de la altura.
    let tiers = [(0.25, 0.22, 0.58), (0.48, 0.17, 0.80), (0.70, 0.11, 1.00)];
    for (base_frac, r_frac, apex_frac) in tiers {
        let ring = base + Vec3::Y * (height_m * base_frac);
        let apex = base + Vec3::Y * (height_m * apex_frac);
        add_cone(
            ring,
            height_m * r_frac,
            apex,
            canopy_color,
            positions,
            normals,
            colors,
            indices,
        );
    }
}

/// Añade un impostor de roble: tronco tipo prisma + copa en caja achatada.
/// Mismos tipos de voxel que el roble real (madera + hojas) → el color combina
/// al convertirse LOD → Real.
fn add_oak_impostor(
    base: Vec3,
    height_m: f32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let trunk_color = linear_rgba(VoxelType::Wood);
    let canopy_color = linear_rgba(VoxelType::Leaves);

    // Tronco más corto y grueso que el del pino.
    let trunk_h = height_m * 0.4;
    let trunk_r = (height_m * 0.04).max(0.06);
    add_trunk_prism(
        base,
        trunk_r,
        trunk_h,
        trunk_color,
        positions,
        normals,
        colors,
        indices,
    );

    // Copa: caja achatada (cubo) en vez del diamante alto: más pequeña y menos
    // puntiaguda. Más ancha que alta, como una frondosa.
    let center = base + Vec3::Y * (height_m * 0.58);
    let half = Vec3::new(height_m * 0.22, height_m * 0.20, height_m * 0.22);
    add_box(
        center,
        half,
        canopy_color,
        positions,
        normals,
        colors,
        indices,
    );
}

/// Tronco: prisma cuadrado vertical (4 caras) desde `base`, radio `r`, alto `h`.
fn add_trunk_prism(
    base: Vec3,
    r: f32,
    h: f32,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let axis = base + Vec3::Y * h * 0.5;
    for i in 0..4 {
        let b0 = ring_point(base, r, i, 4);
        let b1 = ring_point(base, r, (i + 1) % 4, 4);
        let t0 = b0 + Vec3::Y * h;
        let t1 = b1 + Vec3::Y * h;
        add_tri_outward(b0, b1, t1, axis, color, positions, normals, colors, indices);
        add_tri_outward(b0, t1, t0, axis, color, positions, normals, colors, indices);
    }
}

/// Coloca impostores de pino en la huella de este chunk LOD, con la MISMA lógica
/// determinista que `place_trees` (mismo rango de celdas y misma base de altura),
/// así los conos coinciden con los pinos voxel al convertirse LOD → Real (sin pop).
/// Solo pinos: los arbustos y árboles pequeños son invisibles a distancia.
fn add_tree_impostors(
    lod_chunk: &LodChunk,
    seed: i32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    use crate::vegetation::trees::{tree_in_cell, TreeKind, MAX_CANOPY_RADIUS, TREE_CELL_SIZE};

    let n: i32 = 32; // BASE_CHUNK_SIZE en voxels
    let origin = lod_chunk.position * n;

    // Rango de celdas que alcanzan este chunk (expandido por la copa), idéntico a place_trees.
    let r = MAX_CANOPY_RADIUS;
    let cell_x_min = (origin.x - r).div_euclid(TREE_CELL_SIZE);
    let cell_x_max = (origin.x + n - 1 + r).div_euclid(TREE_CELL_SIZE);
    let cell_z_min = (origin.z - r).div_euclid(TREE_CELL_SIZE);
    let cell_z_max = (origin.z + n - 1 + r).div_euclid(TREE_CELL_SIZE);

    let mut terrain_gen = TerrainGenerator::new(seed);

    for cell_x in cell_x_min..=cell_x_max {
        for cell_z in cell_z_min..=cell_z_max {
            let Some(tree) = tree_in_cell(cell_x, cell_z, seed) else {
                continue;
            };

            if !matches!(tree.kind, TreeKind::Pine | TreeKind::Oak) {
                continue;
            }

            let world_x_m = tree.world_x as f32 * VOXEL_SIZE;
            let world_z_m = tree.world_z as f32 * VOXEL_SIZE;
            let surface_m = terrain_gen.biome_gen.generate_height(world_x_m, world_z_m);
            let surface_voxel_y = (surface_m / VOXEL_SIZE).floor() as i32;

            // Misma base que place_trees (surface_voxel_y + 1): el cono cae donde
            // arrancará el tronco real.
            let base = Vec3::new(
                world_x_m,
                (surface_voxel_y + 1) as f32 * VOXEL_SIZE,
                world_z_m,
            );
            let height_m = tree.height() as f32 * VOXEL_SIZE;

            match tree.kind {
                TreeKind::Oak => {
                    add_oak_impostor(base, height_m, positions, normals, colors, indices)
                }
                _ => add_pine_impostor(base, height_m, positions, normals, colors, indices),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pine_impostor_emits_triangles() {
        let (mut p, mut n, mut c, mut i) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        add_pine_impostor(Vec3::ZERO, 5.0, &mut p, &mut n, &mut c, &mut i);
        assert!(!i.is_empty());
    }

    #[test]
    fn pine_impostor_buffers_are_aligned() {
        let (mut p, mut n, mut c, mut i) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        add_pine_impostor(Vec3::ZERO, 5.0, &mut p, &mut n, &mut c, &mut i);
        assert!(n.len() == p.len() && c.len() == p.len());
    }
}

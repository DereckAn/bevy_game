//! Plantillas de árboles: patrones de voxels (tronco + copa) que luego se
//! "estampan" en los chunks durante la generación.

use crate::core::constants::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{BaseChunk, BiomeGenerator, VoxelType};
use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

/// Tamano de la celda de espaciado (en voxels). Como maximo un arbol por celda
pub const TREE_CELL_SIZE: i32 = 12;

/// Probabilidad (0..1) de que una celda contenga un arbol.
const TREE_PROBABILITY: f32 = 0.45;

/// Alcance horizontal máximo de una copa: dice qué celdas vecinas pueden invadir
/// este chunk. Debe ser >= el mayor `canopy_radius` posible.
const MAX_CANOPY_RADIUS: i32 = 8;

/// Tipo de árbol: arbusto pequeño (esfera) o pino grande (cónico, con ramas).
#[derive(Clone, Copy, PartialEq)]
pub enum TreeKind {
    Small,
    Pine,
}

/// Un voxel de una plantilla de árbol: su posición RELATIVA a la base del árbol
/// (la base = el voxel justo sobre el suelo) y su tipo de material.
#[derive(Clone, Copy)]
pub struct TreeVoxel {
    pub offset: IVec3,
    pub voxel_type: VoxelType,
}

/// Un segmento de rama: una "vara" de `start` a `end` con grosor `thickness`,
/// en VOXELS relativos a la base del árbol. Usa floats porque las ramas van en
/// ángulo (posiciones fraccionarias); se rasteriza a voxels de `Wood`.
#[derive(Clone, Copy)]
pub struct Segment {
    pub start: Vec3,
    pub end: Vec3,
    pub thickness: f32,
}

/// Un arbol candidato : su columna (x,z) en VOXELS de mundo y su forma
pub struct TreeInstance {
    pub world_x: i32,
    pub world_z: i32,
    pub kind: TreeKind,
    pub trunk_height: i32,
    pub canopy_radius: i32,
    pub rng_seed: u32, // para variar la forma de cada pino (ramas + mechón)
}

impl TreeInstance {
    /// Altura del árbol SOBRE su base, en voxels (cima del follaje).
    pub fn height(&self) -> i32 {
        match self.kind {
            TreeKind::Small => self.trunk_height + self.canopy_radius,
            TreeKind::Pine => self.trunk_height + 2, // mechón en la punta
        }
    }
}

/// Proyecta `p` sobre el segmento `a`→`b`. Devuelve `(distancia², t)`, donde
/// `t ∈ [0,1]` dice DÓNDE a lo largo del segmento cae el punto más cercano
/// (0 = en `a`, 1 = en `b`). Sin raíz (barato).
fn project_point_on_segment(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    let ab = b - a;
    let len_sq = ab.length_squared();
    let t = if len_sq > 0.0 {
        ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest = a + ab * t;
    ((p - closest).length_squared(), t)
}

/// Rasteriza un segmento a voxels de `Wood`, añadiéndolos a `out`. Recorre la
/// caja contenedora (expandida por el grosor) y conserva los voxels cuyo centro
/// queda a <= thickness de la recta del segmento.
fn voxelize_segment(seg: Segment, out: &mut Vec<TreeVoxel>) {
    let t = seg.thickness;
    let min = seg.start.min(seg.end) - Vec3::splat(t);
    let max = seg.start.max(seg.end) + Vec3::splat(t);

    let (x0, y0, z0) = (
        min.x.floor() as i32,
        min.y.floor() as i32,
        min.z.floor() as i32,
    );
    let (x1, y1, z1) = (
        max.x.ceil() as i32,
        max.y.ceil() as i32,
        max.z.ceil() as i32,
    );

    let thickness_sq = t * t;
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                let p = Vec3::new(x as f32, y as f32, z as f32);
                let (dist_sq, _t) = project_point_on_segment(p, seg.start, seg.end);
                if dist_sq <= thickness_sq {
                    out.push(TreeVoxel {
                        offset: IVec3::new(x, y, z),
                        voxel_type: VoxelType::Wood,
                    });
                }
            }
        }
    }
}

/// Rasteriza un tronco que se AFINA: grosor `base_thickness` en `start` y
/// `tip_thickness` en `end`, interpolado por `t`. Grueso abajo, fino arriba → la
/// copa luce mejor y el tronco parece más natural.
fn voxelize_tapered(
    start: Vec3,
    end: Vec3,
    base_thickness: f32,
    tip_thickness: f32,
    out: &mut Vec<TreeVoxel>,
) {
    let max_thick = base_thickness.max(tip_thickness);
    let min = start.min(end) - Vec3::splat(max_thick);
    let max = start.max(end) + Vec3::splat(max_thick);

    let (x0, y0, z0) = (
        min.x.floor() as i32,
        min.y.floor() as i32,
        min.z.floor() as i32,
    );
    let (x1, y1, z1) = (
        max.x.ceil() as i32,
        max.y.ceil() as i32,
        max.z.ceil() as i32,
    );

    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                let p = Vec3::new(x as f32, y as f32, z as f32);
                let (dist_sq, t) = project_point_on_segment(p, start, end);
                // Grosor local interpolado: base (t=0) → punta (t=1).
                let radius = base_thickness + (tip_thickness - base_thickness) * t;
                if dist_sq <= radius * radius {
                    out.push(TreeVoxel {
                        offset: IVec3::new(x, y, z),
                        voxel_type: VoxelType::Wood,
                    });
                }
            }
        }
    }
}

/// PRNG determinista (xorshift32): avanza el estado y devuelve un u32.
/// Sembrado desde el hash de la celda → cada árbol varía pero es reproducible.
fn next_rand(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// Esfera de Leaves de radio `radius` centrada en `center` (voxels, float).
/// Como la copa esférica, pero con centro fraccionario (puntas de rama).
fn add_leaf_blob(center: Vec3, radius: f32, out: &mut Vec<TreeVoxel>) {
    let r = radius.ceil() as i32;
    let (cx, cy, cz) = (
        center.x.round() as i32,
        center.y.round() as i32,
        center.z.round() as i32,
    );
    let r_sq = radius * radius;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                if (dx * dx + dy * dy + dz * dz) as f32 <= r_sq {
                    out.push(TreeVoxel {
                        offset: IVec3::new(cx + dx, cy + dy, cz + dz),
                        voxel_type: VoxelType::Leaves,
                    });
                }
            }
        }
    }
}

/// Genera un PINO: tronco recto + verticilos de ramas cortas que se acortan hacia
/// la cima (silueta cónica), con mechones de hojas en las puntas.
///
/// `rng_seed` (del hash de la celda) hace que cada pino varíe pero sea
/// reproducible. Pura: mismo `rng_seed` + `trunk_height` → mismos voxels.
pub fn pine_template(rng_seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
    let mut voxels: Vec<TreeVoxel> = Vec::new();
    let mut leaf_centers: Vec<(Vec3, f32)> = Vec::new();
    let mut rng = rng_seed | 1; // evitar estado 0 (xorshift se queda atascado en 0)

    // 1. Tronco recto (voxeles de Wood).
    let h = trunk_height as f32;
    let base_thickness = 1.0 + h * 0.04; // grueso en la base
    let tip_thickness = 0.6; // fino en la cima

    // 1. Tronco recto que se afina hacia arriba (voxeles de Wood).
    voxelize_tapered(
        Vec3::ZERO,
        Vec3::new(0.0, h, 0.0),
        base_thickness,
        tip_thickness,
        &mut voxels,
    );

    // 2. Verticilos de ramas: cada ~3 voxels desde el 35% de la altura a la cima.
    let lowest = (h * 0.35) as i32;
    let top = trunk_height - 1;
    for y in (lowest..=top).step_by(3) {
        let up = (y - lowest) as f32 / (top - lowest).max(1) as f32; // 0 abajo, 1 cima
        let branch_len = 1.0 + 4.5 * (1.0 - up); // largas abajo, cortas arriba → cono
        let count = 3 + (next_rand(&mut rng) % 2) as i32; // 3 o 4 ramas por verticilo
        let base_angle = (next_rand(&mut rng) % 360) as f32 * PI / 180.0;
        for b in 0..count {
            let angle = base_angle + b as f32 * (TAU / count as f32);
            let dir = Vec3::new(angle.cos(), -0.3, angle.sin()).normalize();
            let start = Vec3::new(0.0, y as f32, 0.0);
            let end = start + dir * branch_len;
            voxelize_segment(
                Segment {
                    start,
                    end,
                    thickness: 0.5,
                },
                &mut voxels,
            );
            leaf_centers.push((end, 1.3 + (1.0 - up) * 1.0)); // copa más ancha abajo
        }
    }

    // 3. Mechón en la punta.
    leaf_centers.push((Vec3::new(0.0, h, 0.0), 1.6));

    // 4. Hojas DESPUÉS de la madera. Al estampar (solo sobre aire), la madera gana
    //    los solapes (tronco/ramas visibles) y las hojas rellenan alrededor.
    for (center, radius) in leaf_centers {
        add_leaf_blob(center, radius, &mut voxels);
    }

    voxels
}

/// Hash determinista de una celda + seed -> u32 bien mezclado.
fn hash_cell(cell_x: i32, cell_z: i32, seed: i32) -> u32 {
    let mut h = (cell_x as u32).wrapping_mul(0x9e37_79b9);
    h = (h ^ (cell_z as u32)).wrapping_mul(0x85eb_ca6b);
    h = (h ^ (seed as u32)).wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

/// Decide de forma DETERMINISTA si la celda `(cell_x, cell_z)` tiene un árbol.
///
/// Devuelve `Some(TreeInstance)` (posición con jitter dentro de la celda + forma)
/// o `None`. Función PURA de `(cell, seed)`: cualquier chunk que pregunte por la
/// misma celda obtiene el mismo resultado → árboles consistentes en los bordes.
pub fn tree_in_cell(cell_x: i32, cell_z: i32, seed: i32) -> Option<TreeInstance> {
    let h = hash_cell(cell_x, cell_z, seed);

    // Bits 0–11 → probabilidad. Si no pasa, no hay árbol en esta celda.
    let p = (h & 0xfff) as f32 / 0xfff as f32;
    if p > TREE_PROBABILITY {
        return None;
    }

    let jitter_x = ((h >> 12) & 0xff) as i32 % TREE_CELL_SIZE;
    let jitter_z = ((h >> 20) & 0xff) as i32 % TREE_CELL_SIZE;
    let world_x = cell_x * TREE_CELL_SIZE + jitter_x;
    let world_z = cell_z * TREE_CELL_SIZE + jitter_z;

    // Segundo hash con "sal" → bits independientes para tipo y forma del pino.
    let h2 = hash_cell(cell_x, cell_z, seed ^ 0x5f37_59df);

    if h2 % 5 == 0 {
        // ~1 de cada 5: pino grande.
        let trunk_height = 40 + ((h2 >> 8) % 20) as i32; // 40..=59
        Some(TreeInstance {
            world_x,
            world_z,
            kind: TreeKind::Pine,
            trunk_height,
            canopy_radius: 0,
            rng_seed: h2,
        })
    } else {
        // El resto: arbusto pequeño (como antes).
        let trunk_height = 4 + ((h >> 28) & 0x3) as i32; // 4..=7
        let canopy_radius = 2 + ((h >> 30) & 0x1) as i32; // 2..=3
        Some(TreeInstance {
            world_x,
            world_z,
            kind: TreeKind::Small,
            trunk_height,
            canopy_radius,
            rng_seed: h2,
        })
    }
}

/// Genera la plantilla de un árbol simple: un tronco vertical de `Wood` y una
/// copa esférica de `Leaves` cerca de la cima.
///
/// Las posiciones son RELATIVAS a la base `(0, 0, 0)` = el voxel sobre el suelo.
/// Es una función PURA: misma entrada → misma salida, sin aleatoriedad ni estado.
pub fn tree_template(trunk_height: i32, canopy_radius: i32) -> Vec<TreeVoxel> {
    let mut voxels = Vec::new();

    // Tronco: columna vertical de Wood desde la base (y=0) hasta trunk_height-1.
    for y in 0..trunk_height {
        voxels.push(TreeVoxel {
            offset: IVec3::new(0, y, 0),
            voxel_type: VoxelType::Wood,
        });
    }

    // Copa: esfera de Leaves centrada cerca de la cima del tronco.
    let center_y = trunk_height;
    let r = canopy_radius;
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                if dx * dx + dy * dy + dz * dz <= r * r {
                    voxels.push(TreeVoxel {
                        offset: IVec3::new(dx, center_y + dy, dz),
                        voxel_type: VoxelType::Leaves,
                    });
                }
            }
        }
    }

    voxels
}

/// Estampa en `chunk` los árboles deterministas que caen dentro de él.
///
/// Recorre las celdas de espaciado que se solapan con el chunk (expandido por el
/// alcance de copa), calcula cada árbol con `tree_in_cell`, obtiene la altura de
/// su columna con `biome` (función pura → vale también para columnas fuera del
/// chunk) y escribe los voxels de la plantilla que caen DENTRO de este chunk.
/// Solo escribe sobre aire, para no perforar el terreno.
pub fn place_trees(chunk: &mut BaseChunk, biome: &mut BiomeGenerator, seed: i32) {
    let n = BASE_CHUNK_SIZE as i32;

    // Origen del chunk en voxels de mundo.
    let origin = chunk.position * n;

    // Rango de celdas que pueden alcanzar este chunk en XZ (expandido por la copa).
    let r = MAX_CANOPY_RADIUS;
    let cell_x_min = (origin.x - r).div_euclid(TREE_CELL_SIZE);
    let cell_x_max = (origin.x + n - 1 + r).div_euclid(TREE_CELL_SIZE);
    let cell_z_min = (origin.z - r).div_euclid(TREE_CELL_SIZE);
    let cell_z_max = (origin.z + n - 1 + r).div_euclid(TREE_CELL_SIZE);

    for cell_x in cell_x_min..=cell_x_max {
        for cell_z in cell_z_min..=cell_z_max {
            let Some(tree) = tree_in_cell(cell_x, cell_z, seed) else {
                continue;
            };

            // Altura de la superficie en la columna del árbol (metros → voxels).
            let world_x_m = tree.world_x as f32 * VOXEL_SIZE;
            let world_z_m = tree.world_z as f32 * VOXEL_SIZE;
            let surface_m = biome.generate_height(world_x_m, world_z_m);
            let surface_voxel_y = (surface_m / VOXEL_SIZE).floor() as i32;

            // Base = el voxel justo SOBRE el suelo (donde arranca el tronco).
            let base = IVec3::new(tree.world_x, surface_voxel_y + 1, tree.world_z);

            let template = match tree.kind {
                TreeKind::Small => tree_template(tree.trunk_height, tree.canopy_radius),
                TreeKind::Pine => pine_template(tree.rng_seed, tree.trunk_height),
            };
            for tv in template {
                let world = base + tv.offset;
                let local = world - origin;

                // ¿Cae este voxel dentro de ESTE chunk?
                if local.x < 0
                    || local.x >= n
                    || local.y < 0
                    || local.y >= n
                    || local.z < 0
                    || local.z >= n
                {
                    continue;
                }

                let (lx, ly, lz) = (local.x as usize, local.y as usize, local.z as usize);
                // Solo sobre aire: no perforamos el terreno ya generado.
                if chunk.voxel_types[lx][ly][lz] == VoxelType::Air {
                    chunk.voxel_types[lx][ly][lz] = tv.voxel_type;
                }
            }
        }
    }
}

/// Y de voxel más alto alcanzado por algún árbol cuya base cae en la huella XZ de
/// este chunk (expandida por el alcance horizontal). `None` si no hay árboles.
///
/// El loader lo consulta para NO marcar como "aire" un chunk que un árbol cruza.
pub fn tree_ceiling_for_chunk(
    chunk_pos: IVec3,
    biome: &mut BiomeGenerator,
    seed: i32,
) -> Option<i32> {
    let n = BASE_CHUNK_SIZE as i32;
    let origin = chunk_pos * n;
    let r = MAX_CANOPY_RADIUS;
    let cell_x_min = (origin.x - r).div_euclid(TREE_CELL_SIZE);
    let cell_x_max = (origin.x + n - 1 + r).div_euclid(TREE_CELL_SIZE);
    let cell_z_min = (origin.z - r).div_euclid(TREE_CELL_SIZE);
    let cell_z_max = (origin.z + n - 1 + r).div_euclid(TREE_CELL_SIZE);

    let mut ceiling: Option<i32> = None;
    for cell_x in cell_x_min..=cell_x_max {
        for cell_z in cell_z_min..=cell_z_max {
            let Some(tree) = tree_in_cell(cell_x, cell_z, seed) else {
                continue;
            };
            let wx_m = tree.world_x as f32 * VOXEL_SIZE;
            let wz_m = tree.world_z as f32 * VOXEL_SIZE;
            let surface_voxel_y = (biome.generate_height(wx_m, wz_m) / VOXEL_SIZE).floor() as i32;
            let top = surface_voxel_y + 1 + tree.height();
            ceiling = Some(ceiling.map_or(top, |c| c.max(top)));
        }
    }
    ceiling
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_thin_segment_stays_a_single_column() {
        let mut out = Vec::new();
        voxelize_segment(
            Segment {
                start: Vec3::new(0.0, 0.0, 0.0),
                end: Vec3::new(0.0, 10.0, 0.0),
                thickness: 0.5,
            },
            &mut out,
        );
        assert!(out.iter().all(|v| v.offset.x == 0 && v.offset.z == 0));
    }

    #[test]
    fn pine_has_wood_near_the_top_of_its_trunk() {
        let voxels = pine_template(12345, 20);
        assert!(
            voxels
                .iter()
                .any(|v| v.voxel_type == VoxelType::Wood && v.offset.y >= 18)
        );
    }
}

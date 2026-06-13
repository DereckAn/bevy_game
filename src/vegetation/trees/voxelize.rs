//! Herramientas de geometría compartidas por las plantillas de árboles:
//! rasterizar segmentos/troncos a voxels, esferas de hojas y un PRNG determinista.
//! Es el "kit" reutilizable; cada especie (small/pine/oak) lo usa para construir
//! su forma sin repetir la matemática.

use crate::voxel::VoxelType;
use bevy::prelude::*;

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
pub fn voxelize_segment(seg: Segment, out: &mut Vec<TreeVoxel>) {
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
pub fn voxelize_tapered(
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
pub fn next_rand(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// Esfera de follaje de radio `radius` centrada en `center` (voxels, float).
/// Como la copa esférica, pero con centro fraccionario (puntas de rama). El
/// `leaf_type` deja que cada especie use su propio material (pino/roble/pequeño).
pub fn add_leaf_blob(center: Vec3, radius: f32, leaf_type: VoxelType, out: &mut Vec<TreeVoxel>) {
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
                        voxel_type: leaf_type,
                    });
                }
            }
        }
    }
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
}

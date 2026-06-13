//! Pino: tronco recto que se afina + verticilos de ramas cortas que se acortan
//! hacia la cima (silueta cónica), con mechones de hojas en las puntas.

use super::voxelize::{
    add_leaf_blob, next_rand, voxelize_segment, voxelize_tapered, Segment, TreeVoxel,
};
use crate::voxel::VoxelType;
use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

/// Genera un PINO. `rng_seed` (del hash de la celda) hace que cada pino varíe
/// pero sea reproducible. Pura: mismo `rng_seed` + `trunk_height` → mismos voxels.
pub fn pine_template(rng_seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
    let mut voxels: Vec<TreeVoxel> = Vec::new();
    let mut leaf_centers: Vec<(Vec3, f32)> = Vec::new();
    let mut rng = rng_seed | 1; // evitar estado 0 (xorshift se queda atascado en 0)

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
        add_leaf_blob(center, radius, VoxelType::PineNeedles, &mut voxels);
    }

    voxels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::VoxelType;

    #[test]
    fn pine_has_wood_near_the_top_of_its_trunk() {
        let voxels = pine_template(12345, 20);
        assert!(voxels
            .iter()
            .any(|v| v.voxel_type == VoxelType::Wood && v.offset.y >= 18));
    }
}

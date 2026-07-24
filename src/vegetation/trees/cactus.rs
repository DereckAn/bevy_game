//! Cactus (saguaro): una columna central con 1–2 brazos que salen en horizontal
//! y luego suben. Todo el cuerpo es `VoxelType::Cactus` (verde), sin madera.

use super::voxelize::{next_rand, TreeVoxel};
use crate::voxel::VoxelType;
use bevy::prelude::*;

/// Añade un voxel de cactus en `(x, y, z)` relativo a la base.
fn push(out: &mut Vec<TreeVoxel>, x: i32, y: i32, z: i32) {
    out.push(TreeVoxel {
        offset: IVec3::new(x, y, z),
        voxel_type: VoxelType::Cactus,
    });
}

/// Genera un cactus. `rng_seed` (del hash de la celda) → forma reproducible:
/// mismo `rng_seed` + `trunk_height` → mismos voxels.
pub fn cactus_template(rng_seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
    let mut out = Vec::new();
    let mut rng = rng_seed | 1;

    // Cuerpo central: columna de 1 voxel de ancho.
    for y in 0..trunk_height {
        push(&mut out, 0, y, 0);
    }

    // 1 o 2 brazos, en lados opuestos: 2 voxels en horizontal y 3 hacia arriba.
    let arms = 1 + (next_rand(&mut rng) % 2) as i32; // 1..=2
    for i in 0..arms {
        let dir = if i % 2 == 0 { 1 } else { -1 };
        // Arranque en la mitad superior del cuerpo.
        let base_y =
            trunk_height / 2 + (next_rand(&mut rng) % (trunk_height as u32 / 3).max(1)) as i32;
        for k in 1..=2 {
            push(&mut out, dir * k, base_y, 0);
        }
        for k in 1..=3 {
            push(&mut out, dir * 2, base_y + k, 0);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cactus_is_all_cactus_voxels() {
        let v = cactus_template(1234, 12);
        assert!(v.iter().all(|t| t.voxel_type == VoxelType::Cactus));
    }
}

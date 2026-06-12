//! Arbusto (bush): un montículo pequeño de follaje ATRAVESABLE sobre el suelo.
//! Usa `VoxelType::Foliage` (se ve pero no colisiona), así que se puede caminar
//! a través de él.

use super::voxelize::TreeVoxel;
use crate::voxel::VoxelType;
use bevy::prelude::*;

/// Cúpula de follaje de radio `radius` apoyada en el suelo (media esfera, sin
/// parte enterrada). Posiciones RELATIVAS a la base `(0,0,0)` = sobre el suelo.
pub fn bush_template(radius: i32) -> Vec<TreeVoxel> {
    let mut voxels = Vec::new();
    let r = radius.max(1);
    for dx in -r..=r {
        for dy in 0..=r {
            for dz in -r..=r {
                if dx * dx + dy * dy + dz * dz <= r * r {
                    voxels.push(TreeVoxel {
                        offset: IVec3::new(dx, dy, dz),
                        voxel_type: VoxelType::Bush,
                    });
                }
            }
        }
    }
    voxels
}

//! Árbol pequeño (arbusto): un tronco vertical de `Wood` y una copa esférica de
//! `Leaves`. La forma más simple; cubre la mayoría de los árboles del bosque.

use super::voxelize::TreeVoxel;
use crate::voxel::VoxelType;
use bevy::prelude::*;

/// Genera la plantilla de un árbol pequeño. Posiciones RELATIVAS a la base
/// `(0,0,0)` = el voxel sobre el suelo. Pura: misma entrada → misma salida.
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
                        voxel_type: VoxelType::SmallLeaves,
                    });
                }
            }
        }
    }

    voxels
}

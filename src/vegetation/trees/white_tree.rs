//! Árboles del bioma helado: un pino nevado (reutiliza la forma cónica del pino,
//! reetiquetando su madera/acículas a materiales blancos) y un abedul blanco
//! (tronco esbelto y pálido con una pequeña copa nevada).

use super::pine::pine_template;
use super::voxelize::{add_leaf_blob, TreeVoxel};
use crate::voxel::VoxelType;
use bevy::prelude::*;

/// Pino nevado: misma silueta cónica que el pino normal, pero con corteza pálida
/// (`WhiteWood`) y acículas blancas (`WhiteLeaves`). Pura: reetiqueta la plantilla
/// del pino, así comparte su geometría reproducible.
pub fn snowy_pine_template(rng_seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
    let mut voxels = pine_template(rng_seed, trunk_height);
    for v in &mut voxels {
        v.voxel_type = match v.voxel_type {
            VoxelType::PineWood => VoxelType::WhiteWood,
            VoxelType::PineNeedles => VoxelType::WhiteLeaves,
            other => other,
        };
    }
    voxels
}

/// Abedul blanco: tronco vertical esbelto de `WhiteWood` con una copa esférica
/// pequeña de `WhiteLeaves` en la cima. Pura: misma entrada → misma salida.
pub fn white_birch_template(trunk_height: i32, canopy_radius: i32) -> Vec<TreeVoxel> {
    let mut voxels = Vec::new();

    // Tronco: columna de 2×2 voxeles (≈4 voxeles de grosor) de WhiteWood.
    for y in 0..trunk_height {
        for dz in 0..2 {
            for dx in 0..2 {
                voxels.push(TreeVoxel {
                    offset: IVec3::new(dx, y, dz),
                    voxel_type: VoxelType::WhiteWood,
                });
            }
        }
    }

    // Copa: esfera de WhiteLeaves centrada sobre el tronco (centro del 2×2 = 0.5, 0.5).
    add_leaf_blob(
        Vec3::new(0.5, trunk_height as f32, 0.5),
        canopy_radius as f32,
        VoxelType::WhiteLeaves,
        &mut voxels,
    );

    voxels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snowy_pine_uses_white_wood_near_the_top_of_its_trunk() {
        let voxels = snowy_pine_template(12345, 20);
        assert!(voxels
            .iter()
            .any(|v| v.voxel_type == VoxelType::WhiteWood && v.offset.y >= 18));
    }

    #[test]
    fn white_birch_has_white_leaves_at_its_crown() {
        let voxels = white_birch_template(20, 3);
        assert!(voxels
            .iter()
            .any(|v| v.voxel_type == VoxelType::WhiteLeaves && v.offset.y >= 20));
    }
}

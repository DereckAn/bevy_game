//! Mangle del bioma manglar: tronco corto y grueso, copa ancha y densa, y las
//! características raíces zancudas que salen en arco desde la base hacia el suelo.
//! Reutiliza el kit de `voxelize` (que rasteriza como `Wood`) y reetiqueta la
//! madera a `MangroveWood`, igual que el pino nevado reetiqueta a blanco.

use super::voxelize::{add_leaf_blob, next_rand, voxelize_tapered, TreeVoxel};
use crate::voxel::VoxelType;
use bevy::prelude::*;

/// Nº de raíces zancudas (rango): salen en abanico alrededor del tronco.
const MIN_ROOTS: u32 = 4;
const ROOT_COUNT_RANGE: u32 = 3; // 4..=6

/// Mangle: tronco `MangroveWood`, copa ancha de `MangroveLeaves` y raíces
/// zancudas en arco. Pura: mismo `(rng_seed, trunk_height)` → misma forma.
pub fn mangrove_template(rng_seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
    let mut voxels = Vec::new();
    let mut rng = rng_seed | 1; // evita el estado 0 del xorshift

    // Tronco: cónico, grueso abajo y algo más fino arriba.
    voxelize_tapered(
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, trunk_height as f32, 0.0),
        1.6,
        1.0,
        &mut voxels,
    );

    // Raíces zancudas: arrancan a media altura del tronco y bajan en diagonal
    // hasta clavarse en el suelo (y = -1) separadas del tronco → el look de zancos.
    let root_count = MIN_ROOTS + next_rand(&mut rng) % (ROOT_COUNT_RANGE + 1);
    let start_y = (trunk_height as f32 * 0.45).max(2.0);
    for k in 0..root_count {
        let base_angle = std::f32::consts::TAU * k as f32 / root_count as f32;
        // Pequeño jitter angular determinista para que no queden perfectamente radiales.
        let jitter = (next_rand(&mut rng) % 100) as f32 / 100.0 - 0.5;
        let angle = base_angle + jitter;
        let spread = 3.0 + (next_rand(&mut rng) % 3) as f32; // 3..=5 voxeles
        let end = Vec3::new(angle.cos() * spread, -1.0, angle.sin() * spread);
        voxelize_tapered(Vec3::new(0.0, start_y, 0.0), end, 1.1, 0.7, &mut voxels);
    }

    // Reetiqueta la madera del kit (`Wood`) a `MangroveWood`.
    for v in &mut voxels {
        if v.voxel_type == VoxelType::Wood {
            v.voxel_type = VoxelType::MangroveWood;
        }
    }

    // Copa ancha y baja: un blob central grande más dos laterales → dosel tupido.
    let canopy = 6.0;
    let top = trunk_height as f32;
    add_leaf_blob(
        Vec3::new(0.0, top, 0.0),
        canopy,
        VoxelType::MangroveLeaves,
        &mut voxels,
    );
    add_leaf_blob(
        Vec3::new(canopy * 0.6, top - 1.0, 0.0),
        canopy * 0.7,
        VoxelType::MangroveLeaves,
        &mut voxels,
    );
    add_leaf_blob(
        Vec3::new(-canopy * 0.5, top - 1.0, canopy * 0.4),
        canopy * 0.7,
        VoxelType::MangroveLeaves,
        &mut voxels,
    );

    voxels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangrove_has_leaves_above_its_trunk() {
        let voxels = mangrove_template(12345, 20);
        assert!(voxels
            .iter()
            .any(|v| v.voxel_type == VoxelType::MangroveLeaves && v.offset.y >= 20));
    }

    #[test]
    fn mangrove_prop_roots_reach_the_ground() {
        let voxels = mangrove_template(777, 20);
        assert!(voxels
            .iter()
            .any(|v| v.voxel_type == VoxelType::MangroveWood && v.offset.y < 0));
    }
}

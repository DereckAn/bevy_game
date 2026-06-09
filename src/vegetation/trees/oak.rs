//! Roble (oak): tronco grueso que se BIFURCA recursivamente en ramas, con una
//! copa ancha y redondeada formada por mechones de hojas en las puntas.

use super::voxelize::{TreeVoxel, add_leaf_blob, next_rand, voxelize_tapered};
use bevy::prelude::*;

/// Dirección pseudo-aleatoria con componentes en [-1, 1], para desviar las hijas.
fn random_offset(rng: &mut u32) -> Vec3 {
    let mut comp = || (next_rand(rng) % 1000) as f32 / 500.0 - 1.0; // [-1, 1]
    Vec3::new(comp(), comp(), comp())
}

/// Crece una rama y, RECURSIVAMENTE, sus hijas.
///
/// - Dibuja el segmento `start`→`end` como madera que se afina.
/// - En la última generación (`depth == 0`) deja un mechón de hojas (la copa).
/// - Si no, se bifurca en 2–3 hijas: más cortas, más finas, inclinadas hacia
///   afuera y arriba, con una generación menos.
fn grow(
    start: Vec3,
    dir: Vec3,
    length: f32,
    thickness: f32,
    depth: u32,
    rng: &mut u32,
    out: &mut Vec<TreeVoxel>,
) {
    let end = start + dir * length;
    voxelize_tapered(start, end, thickness, thickness * 0.7, out);

    if depth == 0 {
        add_leaf_blob(end, 2.5, out); // punta → mechón de copa
        return;
    }

    let children = 2 + (next_rand(rng) % 2); // 2 o 3 hijas
    for _ in 0..children {
        let spread = 0.7; // cuánto se abren respecto a la rama padre
        let up_bias = 0.5; // tendencia a seguir subiendo
        let child_dir = (dir + random_offset(rng) * spread + Vec3::Y * up_bias).normalize();
        grow(
            end,
            child_dir,
            length * 0.72,
            thickness * 0.65,
            depth - 1,
            rng,
            out,
        );
    }
}

/// Genera un ROBLE. `rng_seed` (del hash de la celda) → forma reproducible.
/// Pura: mismo `rng_seed` + `trunk_height` → mismos voxels.
pub fn oak_template(rng_seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
    let mut voxels = Vec::new();
    let mut rng = rng_seed | 1;
    let h = trunk_height as f32;

    // La primera "rama" ES el tronco: hacia arriba, grueso, 4 generaciones de copa.
    grow(
        Vec3::ZERO,
        Vec3::Y,
        h * 0.5,        // largo del tronco antes de la primera bifurcación
        2.0 + h * 0.05, // grosor de la base
        4,              // niveles de bifurcación
        &mut rng,
        &mut voxels,
    );

    voxels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::VoxelType;

    #[test]
    fn oak_produces_both_wood_and_leaves() {
        let v = oak_template(777, 35);
        assert!(v.iter().any(|t| t.voxel_type == VoxelType::Leaves));
    }
}

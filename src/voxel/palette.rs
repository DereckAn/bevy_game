//! Registro de paletas tonales por material.
//!
//! Fuente única de verdad: cada `VoxelType` con variación de color declara aquí
//! su paleta (color base + rango de brillo + nº de tonos). El pintado por CPU
//! (`voxel_color`) la consume hoy; el shader de paleta (fase 4b) subirá estos
//! mismos datos a la GPU. Materiales sin entrada usan su color plano.

use crate::vegetation::config;
use crate::voxel::VoxelType;

/// Parámetros de una paleta tonal. Se generan `steps` tonos escalando el brillo
/// del color base: el central es el base (×1.0), los inferiores bajan hasta
/// `dark_mul` y los superiores suben hasta `light_mul`.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Color base en sRGB (0.0..=1.0).
    pub base: [f32; 3],
    /// Multiplicador de brillo del tono más oscuro (< 1.0).
    pub dark_mul: f32,
    /// Multiplicador de brillo del tono más claro (> 1.0).
    pub light_mul: f32,
    /// Número de tonos discretos de la paleta.
    pub steps: u8,
}

/// Paleta de un material, o `None` si usa un color plano (sin variación tonal).
pub fn palette_of(voxel_type: VoxelType) -> Option<Palette> {
    match voxel_type {
        VoxelType::Wood => Some(Palette {
            base: config::WOOD_COLOR,
            dark_mul: config::DARK_MUL,
            light_mul: config::LIGHT_MUL,
            steps: 5,
        }),
        VoxelType::PineWood => Some(Palette {
            base: config::PINE_WOOD_COLOR,
            dark_mul: config::DARK_MUL,
            light_mul: config::LIGHT_MUL,
            steps: 5,
        }),
        _ => None,
    }
}

/// Multiplicador de brillo del tono `i` de una paleta de `steps` tonos.
///
/// Mapea `i` a un valor con signo en [-1, 1] (centro = base): el lado negativo
/// interpola hacia `dark_mul`, el positivo hacia `light_mul`. Con `steps` impar
/// el tono central es exactamente el color base (×1.0). Esta regla la replica el
/// shader en WGSL, así CPU y GPU generan la MISMA paleta.
pub fn step_multiplier(i: u8, palette: &Palette) -> f32 {
    if palette.steps <= 1 {
        return 1.0;
    }
    let t = 2.0 * i as f32 / (palette.steps - 1) as f32 - 1.0; // [-1, 1]
    if t < 0.0 {
        1.0 + t * (1.0 - palette.dark_mul)
    } else {
        1.0 + t * (palette.light_mul - 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_step_is_the_base_color() {
        let p = Palette {
            base: [0.4, 0.2, 0.02],
            dark_mul: 0.7,
            light_mul: 1.25,
            steps: 5,
        };
        assert_eq!(step_multiplier(2, &p), 1.0);
    }

    #[test]
    fn extreme_steps_hit_the_configured_multipliers() {
        let p = Palette {
            base: [0.4, 0.2, 0.02],
            dark_mul: 0.7,
            light_mul: 1.25,
            steps: 5,
        };
        assert_eq!(step_multiplier(0, &p), 0.7);
        assert_eq!(step_multiplier(4, &p), 1.25);
    }
}

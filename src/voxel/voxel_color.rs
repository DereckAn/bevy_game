//! Coloreado de voxels (vertex colors).
//!
//! El meshing escribe un color por vértice; este módulo decide ese color a
//! partir del tipo de voxel y su posición mundial. El pasto usa una paleta de
//! verdes en parches pseudo-aleatorios (value noise); el resto usa el color real
//! de su material. Los colores se devuelven en RGB LINEAL (el shader PBR
//! interpreta las vertex colors como lineales).

use crate::voxel::VoxelType;
use bevy::prelude::*;
use std::sync::LazyLock;

/// Paleta de verdes para el pasto (los hex provistos), precomputada a RGB lineal.
///
/// Convertimos los sRGB una sola vez (LazyLock) para que el color renderizado
/// coincida con los hex.
static GRASS_PALETTE: LazyLock<[[f32; 4]; 11]> = LazyLock::new(|| {
    [
        Color::srgb_u8(0x07, 0x13, 0x09),
        Color::srgb_u8(0x12, 0x35, 0x1A),
        Color::srgb_u8(0x1E, 0x57, 0x2B),
        Color::srgb_u8(0x2E, 0x85, 0x42),
        Color::srgb_u8(0x35, 0x9C, 0x4D),
        Color::srgb_u8(0x41, 0xBE, 0x5E),
        Color::srgb_u8(0x20, 0x61, 0x0A),
        Color::srgb_u8(0x0A, 0x61, 0x20),
        Color::srgb_u8(0x0A, 0x61, 0x4B),
        Color::srgb_u8(0x36, 0x6E, 0x44),
        Color::srgb_u8(0x1F, 0x45, 0x13),
    ]
    .map(|c| {
        let l = c.to_linear();
        [l.red, l.green, l.blue, 1.0]
    })
});

/// Hash entero → [0,1) pseudo-aleatorio y estable por celda.
fn hash01(x: i32, z: i32) -> f32 {
    let h = (x as u32).wrapping_mul(73856093) ^ (z as u32).wrapping_mul(19349663);
    let h = (h ^ (h >> 13)).wrapping_mul(0x5bd1_e995);
    let h = h ^ (h >> 15);
    (h & 0x00ff_ffff) as f32 / 0x00ff_ffff as f32
}

/// Value noise 2D: hash en una rejilla entera + interpolación smoothstep. Da un
/// campo pseudo-aleatorio SUAVE (parches orgánicos), no ruido por voxel.
fn value_noise(x: f32, z: f32) -> f32 {
    let (xi, zi) = (x.floor(), z.floor());
    let (xf, zf) = (x - xi, z - zi);
    let (x0, z0) = (xi as i32, zi as i32);

    let v00 = hash01(x0, z0);
    let v10 = hash01(x0 + 1, z0);
    let v01 = hash01(x0, z0 + 1);
    let v11 = hash01(x0 + 1, z0 + 1);

    let sx = xf * xf * (3.0 - 2.0 * xf);
    let sz = zf * zf * (3.0 - 2.0 * zf);
    let a = v00 + (v10 - v00) * sx;
    let b = v01 + (v11 - v01) * sx;
    a + (b - a) * sz
}

/// Color del pasto: parche pseudo-aleatorio de la paleta. El value_noise da un
/// campo suave que cuantizamos a un índice de paleta → parches orgánicos de cada
/// verde (aleatorios pero coherentes, sin "estática"). La frecuencia controla el
/// tamaño del parche (~0.7 m con 1.4).
fn grass_color(world_x: f32, world_z: f32) -> [f32; 4] {
    let n = value_noise(world_x * 1.4, world_z * 1.4).clamp(0.0, 1.0);
    let palette = &*GRASS_PALETTE;
    let idx = ((n * palette.len() as f32) as usize).min(palette.len() - 1);
    palette[idx]
}

/// Color (RGB lineal) de un vértice según el tipo de voxel: el pasto usa la
/// paleta de verdes variada; el resto usa el color real de su material.
pub fn voxel_color(voxel_type: VoxelType, world_x: f32, world_z: f32) -> [f32; 4] {
    if voxel_type == VoxelType::Grass {
        grass_color(world_x, world_z)
    } else {
        let l = voxel_type.properties().color.to_linear();
        [l.red, l.green, l.blue, 1.0]
    }
}

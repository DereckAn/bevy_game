//! Coloreado de voxels (vertex colors).
//!
//! El meshing escribe un color por vértice; este módulo decide ese color a
//! partir del tipo de voxel y su posición mundial. El pasto se construye
//! interpolando entre dos verdes (oscuro↔claro) según un ruido coherente (fbm)
//! muestreado en COORDENADAS MUNDIALES, así los parches fluyen continuos entre
//! chunks. La interpolación es directa en RGB LINEAL (sin pasar por HSL ni
//! sRGB→linear por voxel: barato). Encima se aplican dos moduladores: tinte por
//! altura (más alto = más claro) y oscurecimiento por pendiente (laderas
//! empinadas = más oscuras). El sombreado por cara (lados más oscuros que la
//! cima) lo aplica el mesher, no este módulo. Los colores se devuelven en RGB
//! LINEAL (el shader PBR los interpreta así).

use crate::core::constants::VOXEL_SIZE;
use crate::vegetation::config;
use crate::voxel::VoxelType;
use bevy::prelude::*;
use std::sync::LazyLock;

/// Escala del ruido: controla el tamaño de los parches de color. El mundo está en
/// metros y un voxel mide `VOXEL_SIZE` (0.1 m), así que para parches de ~2 voxels
/// la escala debe ser alta (una celda de ruido cada ~0.4 m).
const NOISE_SCALE: f32 = 5.0;

/// Expande el contraste del fbm (que tiende a agruparse en torno a 0.5) para que
/// el verde alcance de verdad los tonos oscuros y claros.
const CONTRAST: f32 = 1.8;

/// Mezcla de grano por voxel (0 = solo parches suaves, 1 = ruido por voxel). Un
/// poco de grano rompe las rampas suaves dentro de cada celda de ruido.
const GRAIN: f32 = 0.25;

/// Tinte por altura: rango (m) sobre el que el pasto pasa de exuberante a seco.
const HEIGHT_LOW: f32 = 8.0;
const HEIGHT_HIGH: f32 = 80.0;

/// Verdes extremos del pasto en sRGB: oscuro (sombra/grietas) y claro (algo más
/// amarillento, cimas soleadas). El color final interpola entre ambos.
const GRASS_SRGB_DARK: [f32; 3] = [0.08, 0.22, 0.08];
const GRASS_SRGB_LIGHT: [f32; 3] = [0.45, 0.65, 0.30];

/// Endpoints precomputados a RGB LINEAL (una sola vez): así la conversión sRGB→
/// lineal NO se paga por voxel, solo el lerp.
static GRASS_LINEAR: LazyLock<[[f32; 3]; 2]> = LazyLock::new(|| {
    [GRASS_SRGB_DARK, GRASS_SRGB_LIGHT].map(|c| {
        let l = Color::srgb(c[0], c[1], c[2]).to_linear();
        [l.red, l.green, l.blue]
    })
});

/// Hash entero → [0,1) pseudo-aleatorio y estable por celda.
///
/// Finalizador estilo murmur sobre x y z combinados secuencialmente (no por XOR
/// de dos productos, que deja correlación diagonal y "patrones" visibles).
fn hash01(x: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9e37_79b9);
    h = (h ^ (z as u32)).wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    (h >> 8) as f32 / 0x00ff_ffff as f32
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

/// fbm (fractional Brownian motion): suma 4 octavas de value noise con amplitud
/// decreciente → un campo coherente con variación fina y gruesa. Devuelve [0,1].
fn fbm(x: f32, z: f32) -> f32 {
    let mut total = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut norm = 0.0;
    for _ in 0..2 {
        total += value_noise(x * frequency, z * frequency) * amplitude;
        norm += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    let n = total / norm;
    // Expandir contraste alrededor de 0.5.
    ((n - 0.5) * CONTRAST + 0.5).clamp(0.0, 1.0)
}

/// Color del pasto: interpola entre el verde oscuro y el claro según el ruido
/// coherente, modulado por altura y pendiente.
///
/// - `n` (fbm + grano) recorre oscuro↔claro → parches orgánicos sin "estática".
/// - Altura: empuja hacia el verde claro (cimas soleadas).
/// - Pendiente (`slope` ∈ [0,1]): empuja hacia el verde oscuro (laderas sombreadas).
fn grass_color(world_x: f32, world_y: f32, world_z: f32, slope: f32) -> [f32; 4] {
    // Rotar las coordenadas del ruido ~0.5 rad para que su rejilla NO se alinee
    // con la de voxels (rompe el "patrón" en diamante del value noise).
    const COS_R: f32 = 0.877_582_56; // cos(0.5)
    const SIN_R: f32 = 0.479_425_54; // sin(0.5)
    let rx = world_x * COS_R - world_z * SIN_R;
    let rz = world_x * SIN_R + world_z * COS_R;
    let patches = fbm(rx * NOISE_SCALE, rz * NOISE_SCALE);

    // Grano por voxel: rompe las rampas suaves dentro de cada celda de ruido.
    let vx = (world_x / VOXEL_SIZE).round() as i32;
    let vz = (world_z / VOXEL_SIZE).round() as i32;
    let grain = hash01(vx, vz);

    let n = (patches * (1.0 - GRAIN) + grain * GRAIN).clamp(0.0, 1.0);

    let height = ((world_y - HEIGHT_LOW) / (HEIGHT_HIGH - HEIGHT_LOW)).clamp(0.0, 1.0);
    let slope = slope.clamp(0.0, 1.0);

    // Posición [0,1] entre el verde oscuro y el claro: el ruido es el grueso, la
    // altura aclara y la pendiente oscurece. Lerp directo en RGB lineal.
    let t = (n + height * 0.30 - slope * 0.45).clamp(0.0, 1.0);
    let [dark, light] = *GRASS_LINEAR;
    [
        dark[0] + (light[0] - dark[0]) * t,
        dark[1] + (light[1] - dark[1]) * t,
        dark[2] + (light[2] - dark[2]) * t,
        1.0,
    ]
}

// ============================================================================
// PALETA TONAL DE MADERA (tints & shades desde un color base)
// ============================================================================

/// Genera una paleta tonal de 5 colores a partir de un color base sRGB: dos
/// tonos oscuros, el normal, y dos claros. Escala el brillo del base por cada
/// multiplicador (mantiene el tono, solo varía la luminosidad); `DARK_MUL` y
/// `LIGHT_MUL` fijan los extremos y los intermedios caen a mitad de camino del
/// base. Devuelve RGB LINEAL, listo para pintar.
fn tonal_palette(base_srgb: [f32; 3]) -> [[f32; 3]; 5] {
    let to_lin = |m: f32| {
        let l = Color::srgb(base_srgb[0] * m, base_srgb[1] * m, base_srgb[2] * m).to_linear();
        [l.red, l.green, l.blue]
    };
    let muls = [
        config::DARK_MUL,                // más oscuro
        (config::DARK_MUL + 1.0) * 0.5,  // oscuro
        1.0,                             // normal (color base)
        (config::LIGHT_MUL + 1.0) * 0.5, // claro
        config::LIGHT_MUL,               // más claro
    ];
    muls.map(to_lin)
}

/// Paletas precomputadas (una sola vez) para cada tipo de madera.
static OAK_WOOD_PALETTE: LazyLock<[[f32; 3]; 5]> =
    LazyLock::new(|| tonal_palette(config::WOOD_COLOR));
static PINE_WOOD_PALETTE: LazyLock<[[f32; 3]; 5]> =
    LazyLock::new(|| tonal_palette(config::PINE_WOOD_COLOR));

/// Pinta un voxel de madera eligiendo uno de los 5 tonos de la paleta según un
/// valor pseudo-aleatorio estable de la posición: cada árbol (x,z distintos) y
/// cada banda del tronco (y distinto) cae en un tono. El greedy meshing promedia
/// por quad, así que el efecto es por-quad (bandas del tronco), no por voxel.
fn wood_color(world_x: f32, world_y: f32, world_z: f32, palette: &[[f32; 3]; 5]) -> [f32; 4] {
    let vx = (world_x / VOXEL_SIZE).round() as i32;
    let vy = (world_y / VOXEL_SIZE).round() as i32;
    let vz = (world_z / VOXEL_SIZE).round() as i32;
    let idx = (hash01(vx.wrapping_add(vy.wrapping_mul(31)), vz) * 5.0) as usize;
    let c = palette[idx.min(4)];
    [c[0], c[1], c[2], 1.0]
}

/// Color (RGB lineal) de un vértice según el tipo de voxel: el pasto interpola
/// entre dos verdes (ruido + altura + pendiente); la madera interpola su paleta
/// tonal; el resto usa el color real de su material. `slope` solo afecta al pasto.
pub fn voxel_color(
    voxel_type: VoxelType,
    world_x: f32,
    world_y: f32,
    world_z: f32,
    slope: f32,
) -> [f32; 4] {
    match voxel_type {
        VoxelType::Grass => grass_color(world_x, world_y, world_z, slope),
        VoxelType::Wood => wood_color(world_x, world_y, world_z, &OAK_WOOD_PALETTE),
        VoxelType::PineWood => wood_color(world_x, world_y, world_z, &PINE_WOOD_PALETTE),
        _ => {
            let l = voxel_type.properties().color.to_linear();
            [l.red, l.green, l.blue, 1.0]
        }
    }
}

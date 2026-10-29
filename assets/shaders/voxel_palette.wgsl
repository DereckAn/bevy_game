// Paleta tonal por voxel, compartida por el terreno cercano
// (`palette_extension.wgsl`) y el lejano (`far_terrain.wgsl`) para que ambos
// pinten igual voxel a voxel. `hash01` y `step_multiplier` replican las de
// `src/voxel/palette.rs`.
//
// El pasto no usa paleta: su verde (parches de ruido + grano + altura) se calcula
// aquí por celda, así sus caras superiores se fusionan como cualquier material.

#define_import_path voxel::palette

const VOXEL_SIZE: f32 = 0.1; // = core::constants::VOXEL_SIZE

// Rango tonal por material, indexado por el discriminante de VoxelType. Cada
// entrada: (dark_mul, light_mul, steps, 0); steps < 1 = material plano.
// Hardcodeado aquí (no un uniform) porque el StandardMaterial bindless de Bevy
// 0.17 descarta bindings de extensión en el grupo 2.
// ESPEJO de `src/voxel/palette.rs::palette_of` — mantener en sync.
var<private> SPREADS: array<vec4<f32>, 24> = array<vec4<f32>, 24>(
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 0  Air
    vec4<f32>(0.80, 1.20, 4.0, 0.0), // 1  Dirt
    vec4<f32>(0.60, 1.35, 6.0, 0.0), // 2  Stone
    vec4<f32>(0.70, 1.25, 5.0, 0.0), // 3  Wood
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 4  Metal
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 5  Grass (verde propio: grass_color)
    vec4<f32>(0.88, 1.10, 4.0, 0.0), // 6  Sand
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 7  Leaves
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 8  Foliage
    vec4<f32>(0.80, 1.20, 4.0, 0.0), // 9  Bush
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 10 PineNeedles
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 11 SmallLeaves
    vec4<f32>(0.70, 1.25, 5.0, 0.0), // 12 PineWood
    vec4<f32>(0.80, 1.20, 4.0, 0.0), // 13 Cactus
    vec4<f32>(0.60, 1.20, 5.0, 0.0), // 14 DesertGrass
    vec4<f32>(0.70, 1.20, 4.0, 0.0), // 15 DesertBush
    vec4<f32>(0.92, 1.06, 4.0, 0.0), // 16 Snow
    vec4<f32>(0.85, 1.12, 5.0, 0.0), // 17 Ice
    vec4<f32>(0.82, 1.10, 5.0, 0.0), // 18 WhiteWood
    vec4<f32>(0.88, 1.08, 4.0, 0.0), // 19 WhiteLeaves
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 20 Water (malla/material propio, no usa paleta)
    vec4<f32>(0.80, 1.20, 4.0, 0.0), // 21 Mud
    vec4<f32>(0.70, 1.25, 5.0, 0.0), // 22 MangroveWood
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 23 MangroveLeaves
);

fn hash01(x: i32, z: i32) -> f32 {
    var h: u32 = u32(x) * 0x9e3779b9u;
    h = (h ^ u32(z)) * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return f32(h >> 8u) / 16777215.0;
}

fn step_multiplier(i: u32, steps: u32, dark: f32, light: f32) -> f32 {
    if (steps <= 1u) {
        return 1.0;
    }
    let t = 2.0 * f32(i) / f32(steps - 1u) - 1.0; // [-1, 1]
    if (t < 0.0) {
        return 1.0 + t * (1.0 - dark);
    }
    return 1.0 + t * (light - 1.0);
}

// Parches del pasto: escala del ruido (una celda cada ~0.2 m), contraste del fbm
// y peso del grano por voxel.
const GRASS_NOISE_SCALE: f32 = 5.0;
const GRASS_CONTRAST: f32 = 1.8;
const GRASS_GRAIN: f32 = 0.25;
// Tinte por altura: de exuberante a seco entre estas alturas (m).
const GRASS_HEIGHT_LOW: f32 = 8.0;
const GRASS_HEIGHT_HIGH: f32 = 80.0;
// Verdes extremos en RGB LINEAL (sRGB (0.08,0.22,0.08) y (0.45,0.65,0.30)).
const GRASS_DARK: vec3<f32> = vec3<f32>(0.00719, 0.03968, 0.00719);
const GRASS_LIGHT: vec3<f32> = vec3<f32>(0.17064, 0.38006, 0.07324);
const GRASS_ID: u32 = 5u;

// Value noise 2D: hash en rejilla entera + smoothstep → parches suaves.
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let x0 = i32(i.x);
    let z0 = i32(i.y);
    let s = f * f * (3.0 - 2.0 * f);
    let a = mix(hash01(x0, z0), hash01(x0 + 1, z0), s.x);
    let b = mix(hash01(x0, z0 + 1), hash01(x0 + 1, z0 + 1), s.x);
    return mix(a, b, s.y);
}

// Verde del pasto en el centro de la celda `cell` (coordenadas de voxel).
fn grass_color(cell: vec3<f32>) -> vec3<f32> {
    let center = (cell + 0.5) * VOXEL_SIZE;
    // Rotar ~0.5 rad para que la rejilla del ruido no se alinee con la de voxels
    // (evita el patrón en diamante del value noise).
    let r = vec2<f32>(
        center.x * 0.87758256 - center.z * 0.47942554,
        center.x * 0.47942554 + center.z * 0.87758256,
    ) * GRASS_NOISE_SCALE;
    let fbm = (value_noise(r) + 0.5 * value_noise(r * 2.0)) / 1.5;
    let patches = clamp((fbm - 0.5) * GRASS_CONTRAST + 0.5, 0.0, 1.0);
    let grain = hash01(i32(cell.x), i32(cell.z));
    let n = patches * (1.0 - GRASS_GRAIN) + grain * GRASS_GRAIN;
    let height = clamp((center.y - GRASS_HEIGHT_LOW) / (GRASS_HEIGHT_HIGH - GRASS_HEIGHT_LOW), 0.0, 1.0);
    let t = clamp(n + height * 0.30, 0.0, 1.0);
    return mix(GRASS_DARK, GRASS_LIGHT, t);
}

// Celda de voxel sólida de un punto de su superficie: retrocede medio voxel por
// la normal y trunca.
fn voxel_cell(world_position: vec3<f32>, world_normal: vec3<f32>) -> vec3<f32> {
    return floor((world_position - world_normal * (VOXEL_SIZE * 0.5)) / VOXEL_SIZE);
}

// Color de un voxel: `base` (RGB lineal; blanco × sombreado para el pasto) con
// el tono de la celda según su material `id`.
fn palette_color(base: vec3<f32>, id: u32, cell: vec3<f32>) -> vec3<f32> {
    let spread = SPREADS[min(id, 23u)]; // (dark_mul, light_mul, steps, 0)
    let steps = u32(spread.z);
    if (id == GRASS_ID) {
        return base * grass_color(cell);
    } else if (steps >= 1u) {
        let h = hash01(i32(cell.x) + i32(cell.y) * 31, i32(cell.z));
        let idx = min(u32(h * f32(steps)), steps - 1u);
        return base * step_multiplier(idx, steps, spread.x, spread.y);
    }
    return base;
}

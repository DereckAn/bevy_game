// Extensión de StandardMaterial: aplica una paleta tonal por voxel en el
// fragment shader. El color base llega en el vertex color RGB (uniforme por quad,
// así el greedy meshing puede fusionar) y el vertex ALPHA lleva el discriminante
// de VoxelType (id/255), que indexa el rango tonal del material en `spreads`.
//
// Para cada fragment se deriva la CELDA sólida desde la posición mundial
// (retrocediendo medio voxel por la normal), se hashea a un índice de tono y se
// escala el brillo del color base. `hash01` y `step_multiplier` replican las de
// `src/voxel/palette.rs` para que el resultado sea idéntico voxel a voxel.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

const VOXEL_SIZE: f32 = 0.1; // = core::constants::VOXEL_SIZE

// Rango tonal por material, indexado por el discriminante de VoxelType (llega en
// el vertex alpha). Cada entrada: (dark_mul, light_mul, steps, 0); steps < 1 =
// material plano. Hardcodeado aquí (no un uniform) porque el StandardMaterial
// bindless de Bevy 0.17 descarta bindings de extensión en el grupo 2.
// ESPEJO de `src/voxel/palette.rs::palette_of` — mantener en sync.
var<private> SPREADS: array<vec4<f32>, 16> = array<vec4<f32>, 16>(
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 0  Air
    vec4<f32>(0.80, 1.20, 4.0, 0.0), // 1  Dirt
    vec4<f32>(0.60, 1.35, 6.0, 0.0), // 2  Stone
    vec4<f32>(0.70, 1.25, 5.0, 0.0), // 3  Wood
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 4  Metal
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 5  Grass (color baked en CPU)
    vec4<f32>(0.88, 1.10, 4.0, 0.0), // 6  Sand
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 7  Leaves
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 8  Foliage
    vec4<f32>(0.80, 1.20, 4.0, 0.0), // 9  Bush
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 10 PineNeedles
    vec4<f32>(0.80, 1.15, 4.0, 0.0), // 11 SmallLeaves
    vec4<f32>(0.70, 1.25, 5.0, 0.0), // 12 PineWood
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 13
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 14
    vec4<f32>(0.0, 0.0, 0.0, 0.0),   // 15
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

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

#ifdef VERTEX_COLORS
    // El vertex alpha lleva el discriminante de VoxelType (id/255).
    let id = min(u32(round(in.color.a * 255.0)), 15u);
    let spread = SPREADS[id]; // (dark_mul, light_mul, steps, 0)
    let steps = u32(spread.z);
    if (steps >= 1u) {
        // Centro de la celda sólida: retrocede medio voxel por la normal, trunca.
        let cell = floor((in.world_position.xyz - in.world_normal * (VOXEL_SIZE * 0.5)) / VOXEL_SIZE);
        let h = hash01(i32(cell.x) + i32(cell.y) * 31, i32(cell.z));
        let idx = min(u32(h * f32(steps)), steps - 1u);
        let mul = step_multiplier(idx, steps, spread.x, spread.y);
        pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * mul, pbr_input.material.base_color.a);
    }
#endif

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

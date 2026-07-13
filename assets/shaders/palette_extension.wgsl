// Extensión de StandardMaterial: aplica una paleta tonal por voxel en el
// fragment shader. El color base llega en el vertex color (uniforme por quad, así
// el greedy meshing puede fusionar). El vertex ALPHA marca el material: >= 0.5 =
// material de paleta (variar tono), < 0.5 = plano (dejar el color tal cual).
//
// Para los voxels de paleta se deriva la CELDA sólida desde la posición mundial
// del fragment (retrocediendo medio voxel por la normal), se hashea a un índice
// de tono y se escala el brillo del color base. `hash01` y `step_multiplier`
// replican EXACTAMENTE las de `src/voxel/palette.rs` para que el resultado sea
// idéntico voxel a voxel.

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

// Parámetros de paleta. Hoy son constantes (fase 1): hardcodeadas aquí para
// evitar un binding de uniform, que choca con el StandardMaterial *bindless* de
// Bevy 0.17. Deben coincidir con `core::constants::VOXEL_SIZE` y
// `vegetation::config::{DARK_MUL, LIGHT_MUL}`. La fase 2 (paleta por material)
// las moverá a un buffer.
const VOXEL_SIZE: f32 = 0.1;
const DARK_MUL: f32 = 0.7;
const LIGHT_MUL: f32 = 1.25;
const PALETTE_STEPS: u32 = 5u;

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
    let flag = in.color.a;
#else
    let flag = 0.0;
#endif

    if (flag >= 0.5) {
        // Centro de la celda sólida: retrocede medio voxel por la normal, trunca.
        let cell = floor((in.world_position.xyz - in.world_normal * (VOXEL_SIZE * 0.5)) / VOXEL_SIZE);
        let h = hash01(i32(cell.x) + i32(cell.y) * 31, i32(cell.z));
        let idx = min(u32(h * f32(PALETTE_STEPS)), PALETTE_STEPS - 1u);
        let mul = step_multiplier(idx, PALETTE_STEPS, DARK_MUL, LIGHT_MUL);
        pbr_input.material.base_color = vec4<f32>(pbr_input.material.base_color.rgb * mul, pbr_input.material.base_color.a);
    }

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

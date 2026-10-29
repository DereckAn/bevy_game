// Extensión de StandardMaterial: aplica una paleta tonal por voxel en el
// fragment shader. El color base llega en el vertex color RGB (uniforme por quad,
// así el greedy meshing puede fusionar) y el vertex ALPHA lleva el discriminante
// de VoxelType (id/255), que indexa el rango tonal del material.
//
// Para cada fragment se deriva la CELDA sólida desde la posición mundial
// (retrocediendo medio voxel por la normal) y se le aplica la paleta compartida
// con el terreno lejano (`voxel_palette.wgsl`).

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}
#import voxel::palette::{palette_color, voxel_cell}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

#ifdef VERTEX_COLORS
    // El vertex alpha lleva el discriminante de VoxelType (id/255).
    let id = u32(round(in.color.a * 255.0));
    let cell = voxel_cell(in.world_position.xyz, in.world_normal);
    let base = pbr_input.material.base_color;
    pbr_input.material.base_color = vec4<f32>(palette_color(base.rgb, id, cell), base.a);
#endif

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

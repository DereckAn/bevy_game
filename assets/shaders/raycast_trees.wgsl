// Analytic tree LOD. A batch contains many boxes with per-tree flat attributes.
#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    pbr_types::{pbr_input_new, STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT},
    pbr_functions::{apply_pbr_lighting, calculate_view, main_pass_post_lighting_processing},
}
#import voxel::palette::{palette_color, voxel_cell}

struct TreeColors { wood: vec4<f32>, pine_wood: vec4<f32>, leaves: vec4<f32>, needles: vec4<f32> };
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> colors: TreeColors;

struct Vertex {
    @builtin(instance_index) instance: u32,
    @location(0) position: vec3<f32>,
    @location(1) tree: vec4<f32>,
    @location(2) species: vec2<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) @interpolate(flat) tree: vec4<f32>,
    @location(2) @interpolate(flat) oak: u32,
};
@vertex
fn vertex(v: Vertex) -> VertexOutput {
    let transform = mesh_functions::get_world_from_local(v.instance);
    let world = mesh_functions::mesh_position_local_to_world(transform, vec4<f32>(v.position, 1.0));
    let base = mesh_functions::mesh_position_local_to_world(transform, vec4<f32>(v.tree.xyz, 1.0));
    var out: VertexOutput;
    out.position = view.clip_from_world * world;
    out.world_position = world.xyz;
    out.tree = vec4<f32>(base.xyz, v.tree.w);
    out.oak = u32(v.species.x);
    return out;
}
struct Hit { t: f32, normal: vec3<f32>, id: u32 };
const MISS: f32 = 1e30;
fn closer(a: Hit, b: Hit) -> Hit {
    if (b.t < a.t) { return b; }
    return a;
}
// Slabs also handle origins inside the shape by returning the exit surface.
fn box_hit(o: vec3<f32>, d: vec3<f32>, lo: vec3<f32>, hi: vec3<f32>, id: u32) -> Hit {
    var near = -MISS;
    var far = MISS;
    var nn = vec3<f32>(0.0);
    var nf = nn;
    for (var a = 0; a < 3; a++) {
        if (abs(d[a]) < 1e-8) {
            if (o[a] < lo[a] || o[a] > hi[a]) { return Hit(MISS, nn, id); }
        } else {
            let t0 = (lo[a] - o[a]) / d[a];
            let t1 = (hi[a] - o[a]) / d[a];
            if (min(t0,t1) > near) {
                near = min(t0,t1); nn = vec3<f32>(0.0); nn[a] = -sign(d[a]);
            }
            if (max(t0,t1) < far) {
                far = max(t0,t1); nf = vec3<f32>(0.0); nf[a] = sign(d[a]);
            }
        }
    }
    if (near > far || far < 0.0) { return Hit(MISS, nn, id); }
    if (near >= 0.0) { return Hit(near, nn, id); }
    return Hit(far, nf, id);
}
// Finite upright cone, including its bottom cap. Solve the quadratic exactly.
fn cone_hit(o: vec3<f32>, d: vec3<f32>, bottom: f32, top: f32, radius: f32) -> Hit {
    let k = radius / (top-bottom);
    let k2 = k*k;
    let y = top-o.y;
    let a = dot(d.xz,d.xz)-k2*d.y*d.y;
    let b = 2.0*(dot(o.xz,d.xz)+k2*y*d.y);
    let c = dot(o.xz,o.xz)-k2*y*y;
    var roots = vec2<f32>(MISS);
    if (abs(a) < 1e-7) {
        if (abs(b) > 1e-7) { roots.x = -c/b; }
    } else {
        let discriminant = b*b-4.0*a*c;
        if (discriminant >= 0.0) {
            // Stable quadratic formula avoids cancellation for distant trees.
            let sign_b = select(-1.0, 1.0, b >= 0.0);
            let q = -0.5*(b+sign_b*sqrt(discriminant));
            if (abs(q) > 1e-12) { roots = vec2<f32>(q/a,c/q); }
            else { roots.x = -b/(2.0*a); }
        }
    }
    var hit = Hit(MISS, vec3<f32>(0.0,1.0,0.0), 10u);
    for (var i=0; i<2; i++) {
        let t=roots[i];
        let p=o+d*t;
        if (t >= 0.0 && t < hit.t && p.y >= bottom && p.y <= top) {
            let n=vec3<f32>(p.x,k2*(top-p.y),p.z);
            // At the exact apex the gradient vanishes.
            var normal=vec3<f32>(0.0,1.0,0.0);
            if (length(n)>1e-8) { normal=normalize(n); }
            hit=Hit(t,normal,10u);
        }
    }
    if (abs(d.y)>1e-8) {
        let t=(bottom-o.y)/d.y;
        let p=o.xz+d.xz*t;
        if (t>=0.0 && t<hit.t && dot(p,p)<=radius*radius) {
            hit=Hit(t,vec3<f32>(0.0,-1.0,0.0),10u);
        }
    }
    return hit;
}
struct FragmentOutput { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32 };
@fragment
fn fragment(v: VertexOutput) -> FragmentOutput {
    let d=normalize(v.world_position-view.world_position);
    // Rebase at the proxy entry to keep cone coefficients small at distance.
    let camera_o=(view.world_position-v.tree.xyz)/v.tree.w;
    let bounds=box_hit(camera_o,d,vec3<f32>(-0.23,0.0,-0.23),vec3<f32>(0.23,1.0,0.23),0u);
    var offset=bounds.t;
    if (all(camera_o>=vec3<f32>(-0.23,0.0,-0.23)) && all(camera_o<=vec3<f32>(0.23,1.0,0.23))) { offset=0.0; }
    if (offset>=MISS) { discard; }
    let o=camera_o+d*offset;
    var hit: Hit;
    if (v.oak==1u) {
        hit=box_hit(o,d,vec3<f32>(-0.04,0.0,-0.04),vec3<f32>(0.04,0.4,0.04),3u);
        hit=closer(hit,box_hit(o,d,vec3<f32>(-0.22,0.38,-0.22),vec3<f32>(0.22,0.78,0.22),7u));
    } else {
        hit=box_hit(o,d,vec3<f32>(-0.03,0.0,-0.03),vec3<f32>(0.03,0.35,0.03),12u);
        hit=closer(hit,cone_hit(o,d,0.25,0.58,0.22));
        hit=closer(hit,cone_hit(o,d,0.48,0.80,0.17));
        hit=closer(hit,cone_hit(o,d,0.70,1.00,0.11));
    }
    if (hit.t>=MISS) { discard; }
    let world_hit=v.tree.xyz+(o+d*hit.t)*v.tree.w;
    let clip=view.clip_from_world*vec4<f32>(world_hit,1.0);
    if (clip.w<=0.0 || clip.z>clip.w || clip.z<0.0) { discard; }
    // Base colours are supplied by the same voxel material table as near trees.
    var base=vec3<f32>(0.0);
    switch hit.id {
        case 3u: { base=colors.wood.rgb; }
        case 12u: { base=colors.pine_wood.rgb; }
        case 7u: { base=colors.leaves.rgb; }
        default: { base=colors.needles.rgb; }
    }
    var pbr=pbr_input_new();
    pbr.material.base_color=vec4<f32>(palette_color(base,hit.id,voxel_cell(world_hit,hit.normal)),1.0);
    pbr.material.flags |= STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    pbr.world_position=vec4<f32>(world_hit,1.0);
    pbr.world_normal=hit.normal;
    pbr.N=hit.normal;
    pbr.is_orthographic=false;
    pbr.V=calculate_view(pbr.world_position,false);
    pbr.frag_coord=vec4<f32>(v.position.xy,clip.z/clip.w,1.0/clip.w);
    var out: FragmentOutput;
    out.color=main_pass_post_lighting_processing(pbr,apply_pbr_lighting(pbr));
    out.depth=clip.z/clip.w;
    return out;
}

// Terreno lejano: traza el rayo de cada píxel por una tesela de 64×64 columnas.
// Port de `trace` en `src/voxel/raymarch.rs` (allí están los tests): misma
// cadena de máximos, mismo avance de nodo con índice ENTERO (recalcularlo desde
// la posición puede atascarse en un borde), mismas celdas semiabiertas [x, x+1).
// Se dibujan las caras traseras de la caja de la tesela; el shader calcula la
// entrada del rayo y escribe la profundidad real del impacto.

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_types::{pbr_input_new, STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT},
    pbr_functions::{apply_pbr_lighting, calculate_view, main_pass_post_lighting_processing},
}
#import voxel::palette::{palette_color, voxel_cell}

struct FarTile {
    origin: vec4<f32>, // xyz = esquina mínima (m), w = tamaño de celda (m)
    bounds: vec4<f32>, // x = fondo, y = techo de la caja (celdas)
    surface: vec4<f32>, // rgb = color base lineal, w = id de VoxelType
};

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> tile: FarTile;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var heights: texture_2d<f32>;
// Columnas de chunk que dibuja el terreno cercano: textura toroidal de
// MASK_SIZE², cada texel guarda su columna dueña (o i32::MIN si es del lejano).
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var near_mask: texture_2d<i32>;

const TILE_CELLS: i32 = 64;
const MIP_COUNT: i32 = 7;
const MAX_STEPS: i32 = 512;
const INF: f32 = 3.4e38;
const MASK_SIZE: i32 = 144; // = far_terrain::MASK_SIZE
const CHUNK_M: f32 = 3.2;   // BASE_CHUNK_SIZE × VOXEL_SIZE

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

struct Hit {
    found: bool,
    t: f32,
    normal: vec3<f32>,
};

fn miss() -> Hit {
    return Hit(false, 0.0, vec3<f32>(0.0));
}

fn height_at(level: i32, node: vec2<i32>) -> f32 {
    return textureLoad(heights, node, level).r;
}

// ¿Dibuja ya el terreno cercano la columna de chunk de esta celda? Exacto en
// los niveles cuya celda cabe en un chunk (0–3, hasta 3.2 m), que son los que
// pueden tocar el campo cercano, también mientras una tesela vieja de nivel
// 1–3 sigue viva esperando su reemplazo. Los niveles ≥ 4 empiezan a ≥ 1.6 km del
// jugador: solo llegarían al campo cercano (224 m) viajando ~1.4 km antes de
// que termine un intercambio.
fn is_near_owned(cell: vec2<i32>) -> bool {
    if (tile.origin.w > CHUNK_M) {
        return false;
    }
    let centre = tile.origin.xz + (vec2<f32>(cell) + 0.5) * tile.origin.w;
    let chunk = vec2<i32>(floor(centre / CHUNK_M));
    let texel = ((chunk % MASK_SIZE) + MASK_SIZE) % MASK_SIZE;
    return all(textureLoad(near_mask, texel, 0).xy == chunk);
}

fn cell_index(p: f32, d: f32) -> i32 {
    if (d < 0.0) {
        return i32(ceil(p)) - 1;
    }
    return i32(floor(p));
}

fn exit_t(n: i32, size: i32, o: f32, d: f32) -> f32 {
    if (d > 0.0) {
        return (f32((n + 1) * size) - o) / d;
    } else if (d < 0.0) {
        return (f32(n * size) - o) / d;
    }
    return INF;
}

fn trace(o: vec3<f32>, d: vec3<f32>) -> Hit {
    // Caja de la tesela: [0,64) × [fondo, techo] × [0,64) en celdas.
    let box_min = vec3<f32>(0.0, tile.bounds.x, 0.0);
    let box_max = vec3<f32>(f32(TILE_CELLS), tile.bounds.y, f32(TILE_CELLS));
    var t = 0.0;
    var t_far = INF;
    var axis = 1;
    for (var a = 0; a < 3; a++) {
        if (d[a] == 0.0) {
            let outside = o[a] < box_min[a] || o[a] > box_max[a];
            if (outside || (a != 1 && o[a] == box_max[a])) {
                return miss();
            }
            continue;
        }
        let t0 = (box_min[a] - o[a]) / d[a];
        let t1 = (box_max[a] - o[a]) / d[a];
        if (min(t0, t1) > t) {
            t = min(t0, t1);
            axis = a;
        }
        t_far = min(t_far, max(t0, t1));
    }
    if (t > t_far) {
        return miss();
    }

    let entry = o + d * t;
    let last = TILE_CELLS - 1;
    var cell = vec2<i32>(
        clamp(cell_index(entry.x, d.x), 0, last),
        clamp(cell_index(entry.z, d.z), 0, last),
    );
    // (d.x, d.z) y (o.x, o.z) indexados como la celda: 0 = x, 1 = z.
    let d2 = d.xz;
    let o2 = o.xz;
    var level = MIP_COUNT - 1;
    for (var i = 0; i < MAX_STEPS; i++) {
        let size = 1 << u32(level);
        let node = cell >> vec2<u32>(u32(level));
        let tx = exit_t(node.x, size, o.x, d.x);
        let tz = exit_t(node.y, size, o.z, d.z);
        var t_exit = tz;
        var exit_axis = 2;
        if (tx < tz) {
            t_exit = tx;
            exit_axis = 0;
        }
        if (t_far < t_exit) {
            t_exit = t_far;
            exit_axis = 1;
        }
        // Columna del cercano = aire (`trace_with_air`). Los mips siguen
        // contando su altura: el salto de nodos queda conservador.
        var h = height_at(level, node);
        if (level == 0 && is_near_owned(node)) {
            h = -INF;
        }
        let lowest = min(o.y + d.y * t, o.y + d.y * t_exit);
        if (lowest > h) {
            if (exit_axis == 1) {
                return miss();
            }
            let a = exit_axis / 2;
            let b = 1 - a;
            var step = -1;
            if (d2[a] > 0.0) {
                step = 1;
            }
            let next = node[a] + step;
            if (next < 0 || next >= (TILE_CELLS >> u32(level))) {
                return miss();
            }
            if (step > 0) {
                cell[a] = next * size;
            } else {
                cell[a] = (next + 1) * size - 1;
            }
            let pb = o2[b] + d2[b] * t_exit;
            cell[b] = clamp(cell_index(pb, d2[b]), node[b] * size, (node[b] + 1) * size - 1);
            t = max(t, t_exit);
            axis = exit_axis;
            level = min(level + 1, MIP_COUNT - 1);
        } else if (level > 0) {
            level -= 1;
        } else if (o.y + d.y * t <= h) {
            // Entra ya por debajo del tope: lateral de entrada.
            var n = vec3<f32>(0.0);
            n[axis] = -sign(d[axis]);
            return Hit(true, t, n);
        } else {
            return Hit(true, (h - o.y) / d.y, vec3<f32>(0.0, 1.0, 0.0));
        }
    }
    return miss();
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let cell_m = tile.origin.w;
    let cam = view.world_position;
    let dir = normalize(in.world_position.xyz - cam);
    // A unidades de celda: escala uniforme, así `dir` sigue unitaria y `t` sale en celdas.
    let hit = trace((cam - tile.origin.xyz) / cell_m, dir);
    if (!hit.found) {
        discard;
    }

    let world_hit = cam + dir * (hit.t * cell_m);
    let clip = view.clip_from_world * vec4<f32>(world_hit, 1.0);

    // Mismo color que el terreno cercano: base × sombreado por cara (cima 1.0,
    // lados 0.75, base 0.5) y paleta de la celda de voxel del impacto.
    var shade = 0.75;
    if (hit.normal.y > 0.5) {
        shade = 1.0;
    } else if (hit.normal.y < -0.5) {
        shade = 0.5;
    }
    let id = u32(tile.surface.w);
    var color = palette_color(tile.surface.rgb * shade, id, voxel_cell(world_hit, hit.normal));
    if (tile.bounds.w > 0.5) {
        // F6: tinte por nivel (rojo → amarillo → verde → cian → azul → magenta…).
        let hue = f32(u32(tile.bounds.z)) / 6.0;
        let tint = clamp(abs(fract(hue + vec3<f32>(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0 - 3.0) - 1.0, vec3<f32>(0.0), vec3<f32>(1.0));
        color = mix(color, tint * shade * 0.5, 0.6);
    }

    // Misma luz y niebla que un StandardMaterial por defecto (el del terreno
    // cercano es blanco + vertex color). Sin sombras: NotShadowReceiver, y a
    // esta distancia las cascadas (50 m) ya no llegan.
    var pbr_input = pbr_input_new();
    pbr_input.material.base_color = vec4<f32>(color, 1.0);
    pbr_input.material.flags |= STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = vec4<f32>(world_hit, 1.0);
    pbr_input.world_normal = hit.normal;
    pbr_input.N = hit.normal;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = calculate_view(pbr_input.world_position, pbr_input.is_orthographic);

    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr_input, apply_pbr_lighting(pbr_input));
    out.depth = clip.z / clip.w;
    return out;
}

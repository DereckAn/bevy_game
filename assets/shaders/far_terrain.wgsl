// Terreno lejano: traza el rayo de cada píxel por una tesela de 64×64 columnas.
// Port de `trace` en `src/voxel/raymarch.rs` (allí están los tests): misma
// cadena de máximos, mismo avance de nodo con índice ENTERO (recalcularlo desde
// la posición puede atascarse en un borde), mismas celdas semiabiertas [x, x+1).
// Se dibujan las caras traseras de la caja de la tesela; el shader calcula la
// entrada del rayo y escribe la profundidad real del impacto.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct FarTile {
    origin: vec4<f32>, // xyz = esquina mínima (m), w = tamaño de celda (m)
    bounds: vec4<f32>, // x = fondo, y = techo de la caja (celdas)
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> tile: FarTile;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var heights: texture_2d<f32>;

const TILE_CELLS: i32 = 64;
const MIP_COUNT: i32 = 7;
const MAX_STEPS: i32 = 512;
const INF: f32 = 3.4e38;
// Color plano de depuración (lineal) para distinguir la tesela del LOD.
const DEBUG_COLOR: vec3<f32> = vec3<f32>(0.45, 0.25, 0.6);

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
        let h = height_at(level, node);
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

    // Mismo sombreado por cara que el mallador cercano (cima 1.0, lados 0.75, base 0.5).
    var shade = 0.75;
    if (hit.normal.y > 0.5) {
        shade = 1.0;
    } else if (hit.normal.y < -0.5) {
        shade = 0.5;
    }

    var out: FragmentOutput;
    out.color = vec4<f32>(DEBUG_COLOR * shade, 1.0);
    out.depth = clip.z / clip.w;
    return out;
}

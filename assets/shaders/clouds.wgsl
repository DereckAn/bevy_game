// Domo de cielo: compone tres capas sobre la atmósfera, de atrás a delante:
// estrellas → luna → nubes. Todo se decide por la dirección de visión, así que
// se ve infinitamente lejos. Salida en alfa PREMULTIPLICADO (ver AlphaMode del
// material). Las nubes son FBM 2D proyectado sobre una capa horizontal.

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct CloudUniform {
    sun_direction: vec4<f32>, // xyz = dirección hacia el sol
    params: vec4<f32>,        // x = tiempo, y = coverage
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> cloud: CloudUniform;

fn hash2(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn hash31(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453123);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    let q = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 74.7)),
        dot(p, vec3<f32>(269.5, 183.3, 246.1)),
        dot(p, vec3<f32>(113.5, 271.9, 124.6)),
    );
    return fract(sin(q) * 43758.5453123);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i + vec2<f32>(0.0, 0.0));
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var value = 0.0;
    var amp = 0.5;
    for (var i = 0; i < 5; i = i + 1) {
        value = value + amp * value_noise(p);
        p = p * 2.0;
        amp = amp * 0.5;
    }
    return value;
}

// Campo de estrellas: una celda 3D por dirección, algunas con una estrella
// puntual jitterada. Devuelve brillo [0,1] con un ligero titileo.
fn stars(dir: vec3<f32>, time: f32) -> f32 {
    let scale = 130.0;
    let p = dir * scale;
    let cell = floor(p);
    let f = fract(p) - 0.5;
    let rnd = hash33(cell);
    let present = step(0.93, rnd.z); // ~7 % de celdas tienen estrella
    let center = (rnd.xy - 0.5) * 0.7;
    let d = length(vec3<f32>(f.x - center.x, f.y - center.y, f.z));
    let core = smoothstep(0.09, 0.0, d);
    let mag = mix(0.4, 1.4, hash31(cell + vec3<f32>(3.7)));
    let twinkle = 0.75 + 0.25 * sin(time * 3.0 + rnd.x * 6.28318);
    return present * core * mag * twinkle;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let cam = view.world_position;
    let dir = normalize(in.world_position.xyz - cam);

    // Por debajo del horizonte no hay cielo que dibujar (deja ver el terreno).
    if (dir.y <= 0.0) {
        discard;
    }

    let time = cloud.params.x;
    let coverage = cloud.params.y;
    let sun = normalize(cloud.sun_direction.xyz);

    // Desvanecido hacia el horizonte y factor de noche (sol bajo el horizonte).
    let horizon = smoothstep(0.0, 0.14, dir.y);
    let night = smoothstep(0.10, -0.08, sun.y);

    // --- Estrellas (capa más lejana) ---
    let star_a = clamp(stars(dir, time), 0.0, 1.0) * night * horizon;
    let star_rgb = vec3<f32>(0.9, 0.95, 1.0);

    // --- Luna: opuesta al sol, así que sale al anochecer sin lógica extra. ---
    let moon_dir = normalize(-sun);
    let md = dot(dir, moon_dir);
    let moon_core = smoothstep(0.9990, 0.9994, md);
    let moon_glow = smoothstep(0.985, 0.9990, md) * 0.25;
    let moon_a = clamp(moon_core + moon_glow, 0.0, 1.0) * horizon;
    let moon_rgb = vec3<f32>(2.6, 2.55, 2.4);                // HDR → brilla con bloom

    // --- Nubes (capa más cercana): FBM proyectado sobre una capa horizontal. ---
    let cloud_height = 250.0;
    let t = cloud_height / dir.y;
    let world = cam + dir * t;
    let uv = world.xz * 0.0022 + vec2<f32>(time * 0.006, time * 0.004);
    let base_n = fbm(uv);
    let detail = fbm(uv * 3.1 + vec2<f32>(5.2, 1.3));
    let dens_raw = base_n * 0.75 + detail * 0.25;
    let edge = 0.12;
    let low = 1.0 - coverage - edge;
    let density = smoothstep(low, low + edge * 2.0, dens_raw);
    let cloud_a = density * horizon;
    let toward_sun = clamp(dot(dir, sun) * 0.5 + 0.5, 0.0, 1.0);
    let daylight = clamp(sun.y * 1.5 + 0.15, 0.05, 1.0);
    let cloud_rgb = mix(vec3<f32>(0.55, 0.57, 0.62), vec3<f32>(1.0, 0.98, 0.95), toward_sun) * daylight;

    // Composición "over" en alfa premultiplicado: estrellas → luna → nubes.
    var col = star_rgb * star_a;
    var alpha = star_a;
    col = moon_rgb * moon_a + col * (1.0 - moon_a);
    alpha = moon_a + alpha * (1.0 - moon_a);
    col = cloud_rgb * cloud_a + col * (1.0 - cloud_a);
    alpha = cloud_a + alpha * (1.0 - cloud_a);

    if (alpha < 0.001) {
        discard;
    }
    return vec4<f32>(col, alpha);
}

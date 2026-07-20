//! Ciclo día/noche y configuración atmosférica de la cámara.
//!
//! El "sol" es una `DirectionalLight` marcada con [`Sun`]; su rotación, color e
//! intensidad los deriva [`update_day_night`] de la fracción del día. La cámara
//! recibe la atmósfera física de Bevy 0.17 ([`Atmosphere`]), que pinta el cielo
//! y su color según el ángulo del sol (azul de día, naranja al atardecer, oscuro
//! de noche) sin ninguna textura.

use std::f32::consts::TAU;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{Atmosphere, DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

/// Marca la luz direccional que actúa como sol (la mueve el ciclo día/noche).
#[derive(Component)]
pub struct Sun;

/// Estado del ciclo día/noche.
#[derive(Resource)]
pub struct TimeOfDay {
    /// Fracción del día en [0,1): 0 = medianoche, 0.25 = amanecer,
    /// 0.5 = mediodía, 0.75 = atardecer.
    pub fraction: f32,
    /// Duración de un ciclo completo en segundos.
    pub day_length_secs: f32,
    /// Si el ciclo avanza. En `false` el cielo queda congelado (útil para depurar).
    pub running: bool,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            fraction: 0.32,         // media mañana: la partida abre de día
            day_length_secs: 600.0, // 10 min por ciclo
            running: true,
        }
    }
}

/// Dirección hacia el sol para una fracción del día. Arco este→cenit→oeste con
/// una ligera inclinación en Z para que las sombras no sean perfectamente planas.
pub fn sun_direction(fraction: f32) -> Vec3 {
    let phi = (fraction - 0.25) * TAU;
    Vec3::new(phi.cos(), phi.sin(), 0.25).normalize()
}

/// Crea el sol al empezar partida. `teardown_world` (voxel) lo elimina al volver
/// al menú junto con el resto de `DirectionalLight`, así que no necesita limpieza
/// propia. Su transform/color/intensidad los fija `update_day_night` cada frame.
pub fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Sun,
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::default(),
    ));
}

/// Añade la atmósfera, la niebla y el bloom a la cámara del jugador en cuanto
/// aparece. `Atmosphere` exige HDR, que se añade solo vía sus componentes
/// requeridos (`Hdr`, `AtmosphereSettings` con `scene_units_to_m = 1.0`, que es
/// justo la escala del juego: 1 unidad = 1 metro).
pub fn setup_sky_camera(
    mut commands: Commands,
    cameras: Query<Entity, (With<Camera3d>, Without<Atmosphere>)>,
) {
    for entity in &cameras {
        commands.entity(entity).insert((
            Atmosphere::EARTH,
            Tonemapping::TonyMcMapface,
            Bloom::NATURAL,
            // La niebla oculta el pop-in de chunks (radio de carga ~205 m) y da
            // profundidad. Su color lo actualiza `update_day_night`.
            DistanceFog {
                color: Color::srgb(0.62, 0.72, 0.84),
                falloff: FogFalloff::Linear {
                    start: 90.0,
                    end: 190.0,
                },
                ..default()
            },
        ));
    }
}

/// Avanza el ciclo y actualiza sol, luz ambiental y color de niebla.
pub fn update_day_night(
    time: Res<Time>,
    mut tod: ResMut<TimeOfDay>,
    mut sun: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut ambient: ResMut<AmbientLight>,
    mut fog: Query<&mut DistanceFog>,
) {
    if tod.running {
        tod.fraction = (tod.fraction + time.delta_secs() / tod.day_length_secs).fract();
    }

    let sun_dir = sun_direction(tod.fraction);

    let Ok((mut transform, mut light)) = sun.single_mut() else {
        return;
    };
    // La luz viaja HACIA la escena: apunta desde la posición del sol al origen.
    *transform = Transform::from_translation(sun_dir * 100.0).looking_at(Vec3::ZERO, Vec3::Y);

    // Cuánto sol hay sobre el horizonte (0 de noche, 1 de día pleno).
    let day = smoothstep(-0.05, 0.20, sun_dir.y);
    light.illuminance = day * 20_000.0;

    // Cálido cerca del horizonte, blanco con el sol alto.
    let warmth = (sun_dir.y * 3.0).clamp(0.0, 1.0);
    light.color = Color::srgb(1.0, lerp(0.55, 1.0, warmth), lerp(0.32, 1.0, warmth));

    // Suelo mínimo de luz ambiental para que la noche no sea negra total.
    ambient.brightness = lerp(60.0, 550.0, day);

    if let Ok(mut fog) = fog.single_mut() {
        let c = |d: f32, n: f32| lerp(n, d, day);
        fog.color = Color::srgb(c(0.62, 0.02), c(0.72, 0.03), c(0.84, 0.06));
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

//! Roble realista: la misma idea que `oak.rs` (tronco que se ramifica) pero con
//! la geometría que hace que un roble PAREZCA un roble:
//!
//! - **Dominancia apical**: un eje dominante continúa y de él salen laterales
//!   más cortas y muy abiertas. `oak.rs` bifurca en 2–3 hijas IGUALES, lo que da
//!   una topología de coral, no de árbol.
//! - **Ramas curvas**: cada rama se recorre en sub-segmentos que se reorientan;
//!   la gravedad la hunde al principio y la luz endereza la punta.
//! - **Conservación de sección** (regla de Leonardo): el radio de las hijas sale
//!   de repartir el ÁREA del padre, no de un factor fijo.
//! - **Copa perforada por ruido**: mechones elipsoidales agujereados en vez de
//!   esferas macizas → silueta rota y claros de luz.
//! - **Envolvente de copa**: pasado [`CROWN_MAX_R`] las ramas se doblan hacia el
//!   eje, lo que ACOTA el alcance (ver la nota en esa constante).
//! - **Contrafuertes** en la base, como un roble viejo.
//!
//! Medido sobre 20 000 semillas con `trunk_height = 55`: 104 voxels de alto
//! (10.4 m), alcance máximo 46, radio de tronco 5.75 (~1.15 m de diámetro).
//!
//! La plantilla se genera RECORTADA a una caja (`clip_min`/`clip_max`, en
//! coordenadas relativas a la base del árbol). Una copa de 46 voxels de radio
//! cubre ~16 columnas de chunk y varios niveles en Y; sin el recorte se
//! construirían sus ~16k voxels una vez por chunk para tirar casi todos. Con él,
//! una rama fuera del chunk cuesta 0 iteraciones.

use super::voxelize::{next_rand, TreeVoxel};
use crate::voxel::VoxelType;
use bevy::prelude::*;
use fastnoise_lite::{FastNoiseLite, NoiseType};

// -- Escala y proporciones --------------------------------------------------

/// Radio de la base del tronco = `TRUNK_BASE_R + trunk_height * TRUNK_R_PER_H`.
/// Con `trunk_height` 40..=55 da 5.0..=5.75 voxels (~1.1 m de diámetro).
const TRUNK_BASE_R: f32 = 3.0;
const TRUNK_R_PER_H: f32 = 0.05;

/// Fracción de `trunk_height` que sube el tronco LIMPIO antes de la primera
/// rama. Los robles ramifican bajo, de ahí el 0.40.
const BOLE_FRACTION: f32 = 0.40;

/// Largo de la primera rama tras el tronco limpio, como fracción de la altura.
const FIRST_LIMB_FRACTION: f32 = 0.45;

/// Generaciones de ramificación tras el tronco limpio.
const BRANCH_DEPTH: u32 = 4;

// -- Curvatura de las ramas -------------------------------------------------

/// Sub-segmentos por rama. Más = curva más suave y más coste de rasterizado.
const STEPS: u32 = 5;

/// Desviación aleatoria de la dirección en cada sub-segmento.
const WOBBLE: f32 = 0.16;

/// Cuánto hunde la gravedad la rama al PRINCIPIO de su recorrido.
const DROOP: f32 = 0.30;

/// Cuánto endereza la PUNTA el fototropismo. Junto con `DROOP` produce la firma
/// del roble: ramas que caen y se levantan al final.
const UPTURN: f32 = 0.30;

/// Radio horizontal de la envolvente de la copa (voxels). Pasado ese radio la
/// rama se dobla hacia el eje del árbol.
///
/// No es cosmético: sin envolvente, el paseo aleatorio de las laterales compone a
/// lo largo de `BRANCH_DEPTH` generaciones y la cola de la distribución no está
/// acotada (medido sobre 4000 semillas: 73 voxels, y sigue creciendo con más
/// muestras). Cada semilla rara obligaría a subir [`MAX_CANOPY_RADIUS`], que
/// ensancha el escaneo de celdas de TODOS los biomas. Con la envolvente el
/// alcance queda acotado por construcción: una rama solo puede pasarse un
/// sub-segmento antes de virar.
///
/// [`MAX_CANOPY_RADIUS`]: super::MAX_CANOPY_RADIUS
const CROWN_MAX_R: f32 = 40.0;

/// Fuerza del tirón hacia dentro pasada la envolvente. Debe superar 1.0 (el
/// módulo de la dirección) para que la rama vire de verdad en un solo paso.
const ENVELOPE_PULL: f32 = 1.5;

// -- Dominancia apical ------------------------------------------------------

/// Parte del área de sección que se queda el eje dominante; el resto se lo
/// reparten las laterales.
const LEADER_WEIGHT: f32 = 0.60;

/// Largo de la hija dominante / de una lateral, respecto a la rama padre.
const LEADER_LENGTH: f32 = 0.78;
const LATERAL_LENGTH: f32 = 0.60;

/// Ángulo de salida de las laterales respecto al eje del padre (~62°): las
/// ramas del roble salen casi horizontales.
const LATERAL_ANGLE: f32 = 1.082;

/// Exponente de conservación de sección: `r_hija = r_padre * peso^(1/exp)`.
/// 2.0 sería área exacta; los árboles reales miden ~2.3.
const AREA_EXPONENT: f32 = 2.3;

/// Ángulo de oro (137.507°): reparte las laterales alrededor del eje padre en
/// vez de apelotonarlas a un lado, que es lo que hace el jitter XYZ de `oak.rs`.
const GOLDEN_ANGLE: f32 = 2.399_963_2;

// -- Copa -------------------------------------------------------------------

/// Radio horizontal y vertical del mechón de hojas en cada punta. Achatado
/// (`R_V < R_H`) porque la copa del roble es más ancha que alta.
const TIP_R_H: f32 = 5.0;
const TIP_R_V: f32 = 3.5;

/// Frecuencia del ruido que perfora la copa: periodo ~7 voxels → mechones del
/// tamaño de un manojo de hojas, no motas sueltas.
const CROWN_NOISE_FREQ: f32 = 0.15;

/// Umbral del ruido: por encima hay hoja. Subirlo = copa más rala y rota.
const CROWN_NOISE_THRESHOLD: f32 = -0.15;

// -- Contrafuertes (raíces) -------------------------------------------------

const MIN_ROOTS: u32 = 5;
const ROOT_COUNT_RANGE: u32 = 3; // 5..=7

/// Altura del tronco de la que arrancan los contrafuertes.
const ROOT_START_Y: f32 = 4.0;

/// Genera un ROBLE REALISTA, recortado a la caja `clip_min..=clip_max` (voxels
/// relativos a la base del árbol, ambos inclusive).
///
/// Pura: mismo `(rng_seed, trunk_height)` → misma forma. El recorte solo decide
/// QUÉ voxels se emiten, nunca cuáles existirían: dos chunks vecinos producen
/// rebanadas que encajan sin costura.
pub fn realistic_oak_template(
    rng_seed: u32,
    trunk_height: i32,
    clip_min: IVec3,
    clip_max: IVec3,
) -> Vec<TreeVoxel> {
    let h = trunk_height as f32;
    let base_radius = TRUNK_BASE_R + h * TRUNK_R_PER_H;

    // Ruido sembrado con la semilla del árbol → cada roble tiene su propio
    // patrón de claros en la copa.
    // ponytail: ruido por árbol, no un campo global de bosque. Si algún día se
    // quiere que los claros sean coherentes entre árboles vecinos, sembrar con
    // el seed del mundo y muestrear en coordenadas de MUNDO.
    let mut crown_noise = FastNoiseLite::with_seed(rng_seed as i32);
    crown_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
    crown_noise.set_frequency(Some(CROWN_NOISE_FREQ));

    let mut oak = Oak {
        out: Vec::new(),
        clip_min,
        clip_max,
        rng: rng_seed | 1, // el xorshift se queda muerto en 0
        crown_noise,
        azimuth: 0.0,
    };

    // Tronco limpio: sin gravedad (un tronco no se vence), solo el wobble le da
    // una leve torcedura.
    let (top, dir) = oak.limb(
        Vec3::ZERO,
        Vec3::Y,
        h * BOLE_FRACTION,
        base_radius,
        base_radius * 0.80,
        0.0,
    );
    oak.roots(base_radius);
    oak.grow(
        top,
        dir,
        h * FIRST_LIMB_FRACTION,
        base_radius * 0.80,
        BRANCH_DEPTH,
        DROOP,
    );

    oak.out
}

/// Estado de una construcción: el buffer de salida, la caja de recorte, el PRNG,
/// el ruido de la copa y el acumulador del ángulo de oro. Existe para que la
/// recursión no arrastre nueve parámetros.
struct Oak {
    out: Vec<TreeVoxel>,
    clip_min: IVec3,
    clip_max: IVec3,
    rng: u32,
    crown_noise: FastNoiseLite,
    azimuth: f32,
}

impl Oak {
    /// Aleatorio en [-1, 1].
    fn rand_sym(&mut self) -> f32 {
        (next_rand(&mut self.rng) % 1000) as f32 / 500.0 - 1.0
    }

    /// Recorre una rama en `STEPS` sub-segmentos reorientándose en cada paso:
    /// `droop` la hunde al principio (t→0) y `UPTURN` la endereza al final (t→1).
    ///
    /// Devuelve `(punta, dirección final)` para que las hijas HEREDEN la curva;
    /// si solo devolviera el punto, cada hija arrancaría con la dirección
    /// original y la curvatura se perdería en cada nivel.
    fn limb(
        &mut self,
        start: Vec3,
        dir: Vec3,
        length: f32,
        r_start: f32,
        r_end: f32,
        droop: f32,
    ) -> (Vec3, Vec3) {
        let step_len = length / STEPS as f32;
        let mut p = start;
        let mut d = dir;

        for i in 0..STEPS {
            let t = (i as f32 + 0.5) / STEPS as f32;
            d =
                (d + Vec3::new(
                    self.rand_sym() * WOBBLE,
                    self.rand_sym() * WOBBLE * 0.5 - droop * (1.0 - t) + UPTURN * t,
                    self.rand_sym() * WOBBLE,
                ) + envelope_pull(p))
                .normalize();

            let q = p + d * step_len;
            let a = r_start + (r_end - r_start) * (i as f32 / STEPS as f32);
            let b = r_start + (r_end - r_start) * ((i + 1) as f32 / STEPS as f32);
            self.tapered(p, q, a, b);
            p = q;
        }

        (p, d)
    }

    /// Dibuja una rama y RECURSIVAMENTE sus hijas: una dominante que sigue casi
    /// recta y 1–2 laterales cortas muy abiertas. En la última generación la
    /// punta recibe su mechón de hojas.
    fn grow(&mut self, start: Vec3, dir: Vec3, length: f32, radius: f32, depth: u32, droop: f32) {
        let (tip, tip_dir) = self.limb(start, dir, length, radius, radius * 0.75, droop);

        if depth == 0 {
            self.crown_cluster(tip);
            return;
        }

        let laterals = 1 + next_rand(&mut self.rng) % 2; // 1 o 2
        let lateral_weight = (1.0 - LEADER_WEIGHT) / laterals as f32;

        let leader_dir = (tip_dir
            + Vec3::new(
                self.rand_sym() * 0.18,
                self.rand_sym() * 0.09 + 0.10,
                self.rand_sym() * 0.18,
            ))
        .normalize();
        self.grow(
            tip,
            leader_dir,
            length * LEADER_LENGTH,
            radius * LEADER_WEIGHT.powf(1.0 / AREA_EXPONENT),
            depth - 1,
            droop,
        );

        let (u, v) = perpendicular_basis(tip_dir);
        for _ in 0..laterals {
            self.azimuth += GOLDEN_ANGLE;
            let side = u * self.azimuth.cos() + v * self.azimuth.sin();
            let lateral_dir =
                (tip_dir * LATERAL_ANGLE.cos() + side * LATERAL_ANGLE.sin()).normalize();
            self.grow(
                tip,
                lateral_dir,
                length * LATERAL_LENGTH,
                radius * lateral_weight.powf(1.0 / AREA_EXPONENT),
                depth - 1,
                droop,
            );
        }
    }

    /// Contrafuertes: ensanchan la base como un roble viejo. Salen del tronco a
    /// `ROOT_START_Y` y bajan en diagonal hasta clavarse justo bajo el suelo,
    /// repartidos en abanico (mismo patrón que las raíces zancudas del mangle).
    ///
    /// ponytail: no siguen el terreno. La plantilla es función pura de
    /// `(seed, altura)` y no puede muestrear la altura del suelo, así que en una
    /// pendiente fuerte alguna punta queda al aire. Para arreglarlo habría que
    /// pasar un sampler de altura a la plantilla y romper su pureza.
    fn roots(&mut self, base_radius: f32) {
        let count = MIN_ROOTS + next_rand(&mut self.rng) % (ROOT_COUNT_RANGE + 1);
        for k in 0..count {
            let jitter = (next_rand(&mut self.rng) % 100) as f32 / 100.0 - 0.5;
            let angle = std::f32::consts::TAU * k as f32 / count as f32 + jitter;
            let spread = base_radius + 1.0 + (next_rand(&mut self.rng) % 3) as f32;
            self.tapered(
                Vec3::new(0.0, ROOT_START_Y, 0.0),
                Vec3::new(angle.cos() * spread, -1.0, angle.sin() * spread),
                base_radius * 0.45,
                base_radius * 0.18,
            );
        }
    }

    /// Rasteriza un tramo de madera que se afina de `r_start` a `r_end`.
    ///
    /// Igual que `voxelize::voxelize_tapered`, pero los límites del bucle se
    /// INTERSECAN con la caja de recorte: si el tramo cae fuera del chunk, los
    /// rangos quedan vacíos y no se itera nada.
    fn tapered(&mut self, start: Vec3, end: Vec3, r_start: f32, r_end: f32) {
        let max_r = r_start.max(r_end);
        let lo = (start.min(end) - Vec3::splat(max_r))
            .floor()
            .as_ivec3()
            .max(self.clip_min);
        let hi = (start.max(end) + Vec3::splat(max_r))
            .ceil()
            .as_ivec3()
            .min(self.clip_max);

        let ab = end - start;
        let len_sq = ab.length_squared();

        for x in lo.x..=hi.x {
            for y in lo.y..=hi.y {
                for z in lo.z..=hi.z {
                    let p = Vec3::new(x as f32, y as f32, z as f32);
                    let t = if len_sq > 0.0 {
                        ((p - start).dot(ab) / len_sq).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    // Grosor local interpolado a lo largo del tramo.
                    let radius = r_start + (r_end - r_start) * t;
                    if (p - (start + ab * t)).length_squared() <= radius * radius {
                        self.out.push(TreeVoxel {
                            offset: IVec3::new(x, y, z),
                            voxel_type: VoxelType::Wood,
                        });
                    }
                }
            }
        }
    }

    /// Mechón de hojas en la punta de una rama: elipsoide achatado del que el
    /// ruido 3D quita los voxels que caen por debajo del umbral. Ese agujereado
    /// es lo que rompe la silueta; una esfera maciza se lee como una pelota.
    ///
    /// El ruido se muestrea en coordenadas RELATIVAS al árbol, así que dos
    /// chunks que comparten el mismo mechón calculan los mismos agujeros.
    fn crown_cluster(&mut self, center: Vec3) {
        let radii = Vec3::new(TIP_R_H, TIP_R_V, TIP_R_H);
        let lo = (center - radii).floor().as_ivec3().max(self.clip_min);
        let hi = (center + radii).ceil().as_ivec3().min(self.clip_max);

        for x in lo.x..=hi.x {
            for y in lo.y..=hi.y {
                for z in lo.z..=hi.z {
                    let p = Vec3::new(x as f32, y as f32, z as f32);
                    if ((p - center) / radii).length_squared() > 1.0 {
                        continue;
                    }
                    if self.crown_noise.get_noise_3d(p.x, p.y, p.z) > CROWN_NOISE_THRESHOLD {
                        self.out.push(TreeVoxel {
                            offset: IVec3::new(x, y, z),
                            voxel_type: VoxelType::Leaves,
                        });
                    }
                }
            }
        }
    }
}

/// Tirón hacia el eje del árbol si `p` ya está fuera de la envolvente de la copa,
/// `Vec3::ZERO` si está dentro. Se suma a la dirección como un término más, igual
/// que la gravedad y el fototropismo.
fn envelope_pull(p: Vec3) -> Vec3 {
    let radial = Vec3::new(p.x, 0.0, p.z);
    if radial.length() <= CROWN_MAX_R {
        return Vec3::ZERO;
    }
    -radial.normalize_or_zero() * ENVELOPE_PULL
}

/// Dos ejes unitarios perpendiculares a `d` (unitario) y entre sí: el plano en
/// el que el ángulo de oro reparte las ramas laterales.
fn perpendicular_basis(d: Vec3) -> (Vec3, Vec3) {
    let helper = if d.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    let u = helper.cross(d).normalize();
    (u, d.cross(u))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vegetation::trees::MAX_CANOPY_RADIUS;

    /// Caja lo bastante grande para no recortar nada.
    fn unclipped(seed: u32, trunk_height: i32) -> Vec<TreeVoxel> {
        realistic_oak_template(seed, trunk_height, IVec3::splat(-1000), IVec3::splat(1000))
    }

    /// Clave ordenable de un voxel. `VoxelType` no implementa `Hash`, así que
    /// comparamos vectores ordenados en vez de conjuntos (y de paso el test
    /// verifica también los duplicados, que la plantilla sí produce).
    fn key(v: &TreeVoxel) -> (i32, i32, i32, u8) {
        (v.offset.x, v.offset.y, v.offset.z, v.voxel_type as u8)
    }

    /// Semillas variadas: el alcance y la altura tienen COLA. Con una sola
    /// semilla medí 48 voxels de alcance, pero el peor caso de la distribución
    /// llegaba a 73 antes de añadir la envolvente — un test de una semilla habría
    /// dado luz verde a copas rebanadas en el borde del chunk.
    fn seeds() -> impl Iterator<Item = u32> {
        (0..256u32).map(|s| s.wrapping_mul(2_654_435_761))
    }

    /// Pasarse de [`MAX_CANOPY_RADIUS`] hace que `place_trees` no escanee la celda
    /// del árbol desde los chunks lejanos → copa cortada en plano por el borde.
    #[test]
    fn fits_declared_canopy_reach() {
        let worst = seeds()
            .flat_map(|s| unclipped(s, 55))
            .map(|v| v.offset.x.abs().max(v.offset.z.abs()))
            .max()
            .unwrap();
        assert!(worst <= MAX_CANOPY_RADIUS, "alcance {worst}");
    }

    /// El punto de todo esto es una copa ANCHA: si la geometría degenera a un
    /// árbol esbelto (se rompe el ángulo lateral o la dominancia apical) el
    /// alcance se hunde y esto lo pilla. El roble viejo llega a 24.
    #[test]
    fn crown_is_much_wider_than_the_legacy_oak() {
        let reach = unclipped(0x1234, 55)
            .iter()
            .map(|v| v.offset.x.abs().max(v.offset.z.abs()))
            .max()
            .unwrap();
        assert!(reach > 24, "alcance {reach}");
    }

    /// Quedarse por debajo de `height()` es lo que evita que el loader marque como
    /// aire el chunk donde vive la copa y DECAPITE el árbol.
    #[test]
    fn stays_below_declared_height() {
        let worst = seeds()
            .flat_map(|s| unclipped(s, 55))
            .map(|v| v.offset.y)
            .max()
            .unwrap();
        assert!(worst <= 55 * 2, "altura {worst}");
    }

    /// El recorte solo decide qué se emite: la rebanada recortada tiene que ser
    /// EXACTAMENTE los voxels de la plantilla completa que caen en la caja. Si
    /// esto falla, los chunks vecinos no encajan y aparecen costuras.
    #[test]
    fn clipping_yields_exactly_the_voxels_inside_the_box() {
        let lo = IVec3::new(-10, 20, -10);
        let hi = IVec3::new(21, 51, 21);

        let mut expected: Vec<_> = unclipped(0x1234, 55)
            .iter()
            .filter(|v| v.offset.cmpge(lo).all() && v.offset.cmple(hi).all())
            .map(key)
            .collect();
        expected.sort_unstable();

        let mut clipped: Vec<_> = realistic_oak_template(0x1234, 55, lo, hi)
            .iter()
            .map(key)
            .collect();
        clipped.sort_unstable();

        assert_eq!(clipped, expected);
    }

    #[test]
    fn buttress_roots_reach_below_the_base() {
        let voxels = unclipped(777, 55);
        assert!(voxels
            .iter()
            .any(|v| v.offset.y < 0 && v.voxel_type == VoxelType::Wood));
    }
}

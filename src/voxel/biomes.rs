//! Sistema de biomas para generación de terreno variado
//! Incluye montañas, llanuras, valles, colinas, etc.

use crate::core::{WorldKind, SEA_LEVEL_M};
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

// ============================================================================
// PARÁMETROS DE RELIEVE CONTINUO
// ============================================================================
// El terreno se controla con un campo de "continentalidad" suave en lugar de
// biomas discretos. Esto evita acantilados bruscos: la altura base y la
// amplitud se interpolan de forma continua entre tierras bajas y altas, así
// que la transición ocurre a lo largo de cientos de voxels, no de golpe.

/// Altura base en el extremo de valle (continentalidad mínima), en metros.
const VALLEY_BASE: f32 = -1.0;
/// Altura base en el extremo de montaña (continentalidad máxima), en metros.
const MOUNTAIN_BASE: f32 = 6.0;
/// Amplitud de variación mínima (tierras bajas).
const MIN_AMPLITUDE: f32 = 0.8;
/// Amplitud de variación máxima (tierras altas).
const MAX_AMPLITUDE: f32 = 4.0;
/// Intensidad del detalle extra de montaña, en metros.
const MOUNTAIN_DETAIL: f32 = 1.5;

// -- Desierto: dunas mas altasy onduladas que las llanras normales --
/// Altura base del valle en desierto (las dunas no bana tanto)
const DESERT_VALLEY_BASE: f32 = 0.0;
/// Altura base de las crestas de duna/montaña en desierto (más altas que las normales)
const DESERT_MOUNTAIN_BASE: f32 = 10.0;
/// Amplitud minima: incluso las zonas bajas ondulan (dunas).
const DESERT_MIN_AMPLITUDE: f32 = 2.0;
/// Amplitud maxima: dunas grandes
const DESERT_MAX_AMPLITUDE: f32 = 7.0;
/// Detalle de cresta mas marcado
const DESERT_MOUNTAIN_DETAIL: f32 = 2.5;

// -- Helado: montañas MUCHO más altas y escarpadas que los otros biomas --
/// Altura base en los valles helados (algo elevada, no baja al mar).
const ICE_VALLEY_BASE: f32 = 2.0;
/// Altura base de los picos: muy alta para montañas imponentes.
const ICE_MOUNTAIN_BASE: f32 = 40.0;
/// Amplitud mínima: incluso los valles ondulan un poco.
const ICE_MIN_AMPLITUDE: f32 = 2.0;
/// Amplitud máxima: gran relieve en las zonas altas.
const ICE_MAX_AMPLITUDE: f32 = 14.0;
/// Detalle de montaña fuerte para crestas escarpadas.
const ICE_MOUNTAIN_DETAIL: f32 = 5.0;

// -- Manglar: tierra PLANA claramente sobre el mar, cuencas de agua PROFUNDAS --
// La clave para que el agua se lea como agua (y no un charco a ras de suelo) es
// separar bien los dos regímenes: la tierra queda varios metros SOBRE el nivel
// del mar y las cuencas varios metros por DEBAJO, con poca amplitud local para
// que ninguno cruce el nivel del mar al azar (evita la costa "damero").
/// Fondo de las cuencas: bien hundido → agua profunda, nadable.
const MANGROVE_VALLEY_BASE: f32 = SEA_LEVEL_M - 3.5;
/// Tierra: claramente por encima del mar → islas planas secas.
const MANGROVE_MOUNTAIN_BASE: f32 = SEA_LEVEL_M + 2.0;
/// Amplitud mínima baja: el fondo de las cuencas es casi plano.
const MANGROVE_MIN_AMPLITUDE: f32 = 0.3;
/// Amplitud máxima baja: la tierra es casi plana (no vuelve a cruzar el mar).
const MANGROVE_MAX_AMPLITUDE: f32 = 0.7;
/// Sin detalle de montaña: el manglar no tiene picos.
const MANGROVE_MOUNTAIN_DETAIL: f32 = 0.0;
/// Ancho de la banda de río alrededor del contorno-cero de `river_noise`.
/// Cuanto mayor, más anchos los canales.
const MANGROVE_RIVER_BAND: f32 = 0.07;
/// Metros que se hunde el fondo del río por debajo del nivel del mar (canales
/// profundos que cortan la tierra plana).
const MANGROVE_RIVER_DEPTH: f32 = 4.0;

/// Interpolación lineal.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smoothstep clásico: 0 bajo `edge0`, 1 sobre `edge1`, suave entre medias.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Generador de biomas
pub struct BiomeGenerator {
    /// Continentalidad: campo suave que controla el relieve (valle ↔ montaña)
    biome_noise: FastNoiseLite,
    /// Detalle fractal compartido del terreno (mismo en todo el mundo)
    terrain_noise: FastNoiseLite,
    /// Detalle adicional para montañas (entra gradualmente con la altura)
    mountain_detail_noise: FastNoiseLite,
    /// Contorno para tallar ríos serpenteantes en el manglar (baja frecuencia).
    river_noise: FastNoiseLite,
    /// bioma que se esta generando (elige los parametros de relieve).
    kind: WorldKind,
}

impl BiomeGenerator {
    pub fn new(seed: i32, kind: WorldKind) -> Self {
        // Ruido para determinar tipo de bioma / continentalidad
        let mut biome_noise = FastNoiseLite::new();
        biome_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        biome_noise.set_frequency(Some(0.003)); // Biomas grandes, transiciones largas
        biome_noise.set_seed(Some(seed));

        // Detalle fractal del terreno: una sola capa para todo el mundo, así
        // el relieve local es uniforme y solo cambian base/amplitud.
        let mut terrain_noise = FastNoiseLite::new();
        terrain_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        terrain_noise.set_fractal_type(Some(FractalType::FBm));
        terrain_noise.set_fractal_octaves(Some(4));
        terrain_noise.set_frequency(Some(0.015));
        terrain_noise.set_seed(Some(seed.wrapping_add(500)));

        // Detalle de montaña (alta frecuencia)
        let mut mountain_detail_noise = FastNoiseLite::new();
        mountain_detail_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        mountain_detail_noise.set_frequency(Some(0.08));
        mountain_detail_noise.set_seed(Some(seed.wrapping_add(54321)));

        // Ríos del manglar: baja frecuencia → meandros largos. Se talla el
        // terreno a lo largo de su contorno-cero (ver `generate_height`).
        let mut river_noise = FastNoiseLite::new();
        river_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        river_noise.set_frequency(Some(0.004));
        river_noise.set_seed(Some(seed.wrapping_add(900)));

        Self {
            biome_noise,
            terrain_noise,
            mountain_detail_noise,
            river_noise,
            kind,
        }
    }

    /// Genera la altura del terreno de forma continua.
    ///
    /// La continentalidad (`biome_noise`) es un campo suave en [-1, 1]; de él
    /// derivamos base y amplitud interpoladas, por lo que el terreno pasa de
    /// llano a montañoso gradualmente y nunca de golpe.
    pub fn generate_height(&mut self, world_x: f32, world_z: f32) -> f32 {
        // Parámetros de relieve según el bioma.
        let (valley_base, mountain_base, min_amp, max_amp, mountain_detail) = match self.kind {
            WorldKind::Normal => (
                VALLEY_BASE,
                MOUNTAIN_BASE,
                MIN_AMPLITUDE,
                MAX_AMPLITUDE,
                MOUNTAIN_DETAIL,
            ),
            WorldKind::Desert => (
                DESERT_VALLEY_BASE,
                DESERT_MOUNTAIN_BASE,
                DESERT_MIN_AMPLITUDE,
                DESERT_MAX_AMPLITUDE,
                DESERT_MOUNTAIN_DETAIL,
            ),
            WorldKind::Ice => (
                ICE_VALLEY_BASE,
                ICE_MOUNTAIN_BASE,
                ICE_MIN_AMPLITUDE,
                ICE_MAX_AMPLITUDE,
                ICE_MOUNTAIN_DETAIL,
            ),
            WorldKind::Mangrove => (
                MANGROVE_VALLEY_BASE,
                MANGROVE_MOUNTAIN_BASE,
                MANGROVE_MIN_AMPLITUDE,
                MANGROVE_MAX_AMPLITUDE,
                MANGROVE_MOUNTAIN_DETAIL,
            ),
        };

        let continent = self.biome_noise.get_noise_2d(world_x, world_z);
        let t = ((continent + 1.0) * 0.5).clamp(0.0, 1.0); // [0, 1]
        let s = t * t * (3.0 - 2.0 * t); // smoothstep para suavizar aún más

        let base = lerp(valley_base, mountain_base, s);
        let amplitude = lerp(min_amp, max_amp, s);

        let detail = self.terrain_noise.get_noise_2d(world_x, world_z);
        let mut height = base + detail * amplitude;

        // El detalle de montaña entra con peso suave (sin escalón en el umbral)
        let mountain_weight = smoothstep(0.45, 0.9, t);
        height += self.mountain_detail_noise.get_noise_2d(world_x, world_z)
            * mountain_detail
            * mountain_weight;

        // Ríos del manglar: talla canales serpenteantes a lo largo del
        // contorno-cero de `river_noise`. Cerca del contorno, hunde el terreno
        // hasta por debajo del nivel del mar para que se llenen de agua.
        if self.kind == WorldKind::Mangrove {
            let r = self.river_noise.get_noise_2d(world_x, world_z).abs();
            let carve = smoothstep(MANGROVE_RIVER_BAND, 0.0, r); // 1 en el centro del canal, 0 fuera
            let river_bed = SEA_LEVEL_M - MANGROVE_RIVER_DEPTH;
            height = lerp(height, height.min(river_bed), carve);
        }

        height
    }

    /// Bioma que este generador está produciendo.
    pub fn kind(&self) -> WorldKind {
        self.kind
    }
}

/// Generador de terreno con múltiples capas de ruido
pub struct TerrainGenerator {
    pub biome_gen: BiomeGenerator,
}

impl TerrainGenerator {
    pub fn new(seed: i32, kind: WorldKind) -> Self {
        Self {
            biome_gen: BiomeGenerator::new(seed, kind),
        }
    }
}

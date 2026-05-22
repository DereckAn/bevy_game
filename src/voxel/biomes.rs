//! Sistema de biomas para generación de terreno variado
//! Incluye montañas, llanuras, valles, colinas, etc.

use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

/// Tipos de biomas disponibles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BiomeType {
    Plains,    // Llanuras planas
    Hills,     // Colinas suaves
    Mountains, // Montañas altas
    Valley,    // Valles profundos
    Plateau,   // Mesetas
}

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
    /// Humedad (para clasificación de bioma y, a futuro, vegetación)
    moisture_noise: FastNoiseLite,
    /// Detalle fractal compartido del terreno (mismo en todo el mundo)
    terrain_noise: FastNoiseLite,
    /// Detalle adicional para montañas (entra gradualmente con la altura)
    mountain_detail_noise: FastNoiseLite,
}

impl BiomeGenerator {
    pub fn new(seed: i32) -> Self {
        // Ruido para determinar tipo de bioma / continentalidad
        let mut biome_noise = FastNoiseLite::new();
        biome_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        biome_noise.set_frequency(Some(0.003)); // Biomas grandes, transiciones largas
        biome_noise.set_seed(Some(seed));

        // Ruido para humedad (futuro: árboles, agua, etc.)
        let mut moisture_noise = FastNoiseLite::new();
        moisture_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        moisture_noise.set_frequency(Some(0.005));
        moisture_noise.set_seed(Some(seed.wrapping_add(1000)));

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

        Self {
            biome_noise,
            moisture_noise,
            terrain_noise,
            mountain_detail_noise,
        }
    }

    /// Obtiene el bioma en una posición mundial (clasificación para materiales).
    pub fn get_biome(&mut self, world_x: f32, world_z: f32) -> BiomeType {
        let biome_value = self.biome_noise.get_noise_2d(world_x, world_z);
        let moisture = self.moisture_noise.get_noise_2d(world_x, world_z);

        match (biome_value, moisture) {
            (b, _) if b > 0.4 => BiomeType::Mountains,
            (b, _) if b < -0.4 => BiomeType::Valley,
            (b, m) if b > 0.1 && m < -0.2 => BiomeType::Plateau,
            (b, _) if b > -0.1 && b < 0.1 => BiomeType::Hills,
            _ => BiomeType::Plains,
        }
    }

    /// Genera la altura del terreno de forma continua.
    ///
    /// La continentalidad (`biome_noise`) es un campo suave en [-1, 1]; de él
    /// derivamos base y amplitud interpoladas, por lo que el terreno pasa de
    /// llano a montañoso gradualmente y nunca de golpe.
    pub fn generate_height(&mut self, world_x: f32, world_z: f32) -> f32 {
        let continent = self.biome_noise.get_noise_2d(world_x, world_z);
        let t = ((continent + 1.0) * 0.5).clamp(0.0, 1.0); // [0, 1]
        let s = t * t * (3.0 - 2.0 * t); // smoothstep para suavizar aún más

        let base = lerp(VALLEY_BASE, MOUNTAIN_BASE, s);
        let amplitude = lerp(MIN_AMPLITUDE, MAX_AMPLITUDE, s);

        let detail = self.terrain_noise.get_noise_2d(world_x, world_z);
        let mut height = base + detail * amplitude;

        // El detalle de montaña entra con peso suave (sin escalón en el umbral)
        let mountain_weight = smoothstep(0.45, 0.9, t);
        height += self.mountain_detail_noise.get_noise_2d(world_x, world_z)
            * MOUNTAIN_DETAIL
            * mountain_weight;

        height
    }
}

/// Generador de terreno con múltiples capas de ruido
pub struct TerrainGenerator {
    pub biome_gen: BiomeGenerator,
    cave_noise: FastNoiseLite,
}

impl TerrainGenerator {
    pub fn new(seed: i32) -> Self {
        let biome_gen = BiomeGenerator::new(seed);

        // Ruido para cuevas (futuro)
        let mut cave_noise = FastNoiseLite::new();
        cave_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        cave_noise.set_frequency(Some(0.05));
        cave_noise.set_seed(Some(seed.wrapping_add(2000)));

        Self {
            biome_gen,
            cave_noise,
        }
    }

    /// Calcula la densidad en un punto 3D
    pub fn get_density(&mut self, world_x: f32, world_y: f32, world_z: f32) -> f32 {
        // Obtener altura del terreno en esta posición XZ
        let terrain_height = self.biome_gen.generate_height(world_x, world_z);

        // Densidad básica: positivo bajo tierra, negativo en aire
        let density = terrain_height - world_y;

        density
    }
}

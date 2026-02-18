//! Sistema de biomas para generación de terreno variado
//! Incluye montañas, llanuras, valles, colinas, etc.

use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};

/// Tipos de biomas disponibles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BiomeType {
    Plains,      // Llanuras planas
    Hills,       // Colinas suaves
    Mountains,   // Montañas altas
    Valley,      // Valles profundos
    Plateau,     // Mesetas
}

impl BiomeType {
    /// Altura base del bioma (en metros)
    pub fn base_height(&self) -> f32 {
        match self {
            BiomeType::Plains => 1.0,
            BiomeType::Hills => 2.0,
            BiomeType::Mountains => 5.0,
            BiomeType::Valley => -1.0,
            BiomeType::Plateau => 3.0,
        }
    }
    
    /// Amplitud de variación (qué tan alto/bajo puede variar)
    pub fn amplitude(&self) -> f32 {
        match self {
            BiomeType::Plains => 0.5,      // Muy plano
            BiomeType::Hills => 2.0,       // Colinas moderadas
            BiomeType::Mountains => 8.0,   // Montañas muy altas
            BiomeType::Valley => 1.0,      // Valles suaves
            BiomeType::Plateau => 1.5,     // Mesetas con bordes
        }
    }
    
    /// Frecuencia del ruido (qué tan rápido cambia el terreno)
    pub fn frequency(&self) -> f32 {
        match self {
            BiomeType::Plains => 0.01,     // Cambios muy lentos
            BiomeType::Hills => 0.03,      // Cambios moderados
            BiomeType::Mountains => 0.02,  // Cambios lentos pero grandes
            BiomeType::Valley => 0.025,    // Cambios moderados
            BiomeType::Plateau => 0.015,   // Cambios lentos
        }
    }
    
    /// Octavas de ruido (detalle del terreno)
    pub fn octaves(&self) -> i32 {
        match self {
            BiomeType::Plains => 2,        // Poco detalle
            BiomeType::Hills => 3,         // Detalle moderado
            BiomeType::Mountains => 5,     // Mucho detalle
            BiomeType::Valley => 3,        // Detalle moderado
            BiomeType::Plateau => 4,       // Buen detalle
        }
    }
}

/// Generador de biomas
pub struct BiomeGenerator {
    biome_noise: FastNoiseLite,
    moisture_noise: FastNoiseLite,
    /// Ruido de terreno pre-construido para cada bioma (indexado por BiomeType)
    terrain_noises: [FastNoiseLite; 5],
    /// Ruido de detalle para montañas (pre-construido)
    mountain_detail_noise: FastNoiseLite,
}

impl BiomeGenerator {
    pub fn new(seed: i32) -> Self {
        // Ruido para determinar tipo de bioma
        let mut biome_noise = FastNoiseLite::new();
        biome_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        biome_noise.set_frequency(Some(0.003)); // Biomas grandes
        biome_noise.set_seed(Some(seed));

        // Ruido para humedad (futuro: árboles, agua, etc.)
        let mut moisture_noise = FastNoiseLite::new();
        moisture_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        moisture_noise.set_frequency(Some(0.005));
        moisture_noise.set_seed(Some(seed + 1000));

        // Pre-construir ruido de terreno para cada bioma
        let biomes = [
            BiomeType::Plains,
            BiomeType::Hills,
            BiomeType::Mountains,
            BiomeType::Valley,
            BiomeType::Plateau,
        ];
        let terrain_noises = biomes.map(|biome| {
            let mut noise = FastNoiseLite::new();
            noise.set_noise_type(Some(NoiseType::OpenSimplex2));
            noise.set_fractal_type(Some(FractalType::FBm));
            noise.set_fractal_octaves(Some(biome.octaves()));
            noise.set_frequency(Some(biome.frequency()));
            noise.set_seed(Some(12345));
            noise
        });

        // Pre-construir ruido de detalle para montañas
        let mut mountain_detail_noise = FastNoiseLite::new();
        mountain_detail_noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        mountain_detail_noise.set_frequency(Some(0.08));
        mountain_detail_noise.set_seed(Some(54321));

        Self {
            biome_noise,
            moisture_noise,
            terrain_noises,
            mountain_detail_noise,
        }
    }

    /// Obtiene el bioma en una posición mundial
    pub fn get_biome(&mut self, world_x: f32, world_z: f32) -> BiomeType {
        let biome_value = self.biome_noise.get_noise_2d(world_x, world_z);
        let moisture = self.moisture_noise.get_noise_2d(world_x, world_z);

        // Usar biome_value y moisture para determinar bioma
        match (biome_value, moisture) {
            // Montañas: valores altos de biome
            (b, _) if b > 0.4 => BiomeType::Mountains,

            // Valles: valores muy bajos
            (b, _) if b < -0.4 => BiomeType::Valley,

            // Mesetas: valores medios-altos con baja humedad
            (b, m) if b > 0.1 && m < -0.2 => BiomeType::Plateau,

            // Colinas: valores medios
            (b, _) if b > -0.1 && b < 0.1 => BiomeType::Hills,

            // Llanuras: por defecto
            _ => BiomeType::Plains,
        }
    }

    /// Índice en terrain_noises para cada bioma
    fn biome_index(biome: BiomeType) -> usize {
        match biome {
            BiomeType::Plains => 0,
            BiomeType::Hills => 1,
            BiomeType::Mountains => 2,
            BiomeType::Valley => 3,
            BiomeType::Plateau => 4,
        }
    }

    /// Genera altura del terreno con biomas
    pub fn generate_height(&mut self, world_x: f32, world_z: f32) -> f32 {
        let biome = self.get_biome(world_x, world_z);

        // Usar ruido pre-construido para este bioma
        let idx = Self::biome_index(biome);
        let noise_value = self.terrain_noises[idx].get_noise_2d(world_x, world_z);
        let height = biome.base_height() + noise_value * biome.amplitude();

        // Agregar capa adicional de detalle para montañas
        if matches!(biome, BiomeType::Mountains) {
            let detail = self.mountain_detail_noise.get_noise_2d(world_x, world_z) * 2.0;
            height + detail
        } else {
            height
        }
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
        cave_noise.set_seed(Some(seed + 2000));
        
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
        let mut density = terrain_height - world_y;
        
        // Agregar cuevas (opcional, comentado por ahora)
        // let cave_value = self.cave_noise.get_noise_3d(world_x, world_y, world_z);
        // if world_y < terrain_height - 0.5 && cave_value > 0.6 {
        //     density = -1.0; // Crear cavidad
        // }
        
        density
    }
}

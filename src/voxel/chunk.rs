//! Sistema de chunks columnares
//! 
//! Un chunk es una columna del mundo que contiene un campo de densidad 3D
//! Los valores de densidad determinan que es solido (>0) y que es aire (<=0)

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};
use crate::core::constants::{CHUNK_SIZE, VOXEL_SIZE, WORLD_HEIGHT};
use super::voxel_types::VoxelType;

/// Representa un chunk columnar del mundo con su campo de densidad y tipos de voxels.
/// 
/// # Diseño de Datos
/// 
/// El chunk mantiene DOS arrays paralelos:
/// 
/// 1. **densities**: Campo de densidad para Surface Nets (terreno suave)
///    - Tamaño: (CHUNK_SIZE + 1) × (WORLD_HEIGHT + 1) × (CHUNK_SIZE + 1)
///    - Positivo = sólido, Negativo = aire
///    - Usado por el sistema de meshing
/// 
/// 2. **voxel_types**: Tipo de material de cada voxel
///    - Tamaño: CHUNK_SIZE × WORLD_HEIGHT × CHUNK_SIZE
///    - Usado para gameplay (destrucción, drops, colores)
///    - Solo 1 byte por voxel gracias a #[repr(u8)]
/// 
/// # Memoria por Chunk
/// - Densities: (129 × 513 × 129) × 4 bytes = ~34 MB
/// - Types: (128 × 512 × 128) × 1 byte = ~8 MB
/// - **Total: ~42 MB por chunk**
#[derive(Component)]
pub struct Chunk {
    // Campo de densidad 3D. Positivo = solido, Negativo = aire
    // Tamaño +1 para permitir interpolación en bordes
    pub densities: [[[f32; CHUNK_SIZE + 1]; WORLD_HEIGHT + 1]; CHUNK_SIZE + 1],
    
    // Tipo de material de cada voxel
    // Usado para gameplay: destrucción, drops, colores
    pub voxel_types: [[[VoxelType; CHUNK_SIZE]; WORLD_HEIGHT]; CHUNK_SIZE],
    
    // Posicion del chunk en coordenadas de chunk (solo X,Z)
    pub position: IVec2
}

// ============================================================================
// IMPLEMENTACIÓN
// ============================================================================

impl Chunk {
    /// Crea un nuevo chunk columnar con terreno generado proceduralmente
    /// 
    /// Usa ruido Perlin para generar un terreno ondulado con una altura base.
    /// 
    /// # Parametros
    /// - 'position': Posicion del chunk en coordenadas del chunk (X,Z)
    /// 
    /// # Ejemplo
    /// ```ignore
    /// let chunk = Chunk::new(IVec2::new(0, 0));
    /// ```
    pub fn new(position: IVec2) -> Self {
        let perlin = Perlin::new(12345);
        let mut chunk = Self {
            densities: [[[0.0; CHUNK_SIZE + 1]; WORLD_HEIGHT + 1]; CHUNK_SIZE + 1],
            voxel_types: [[[VoxelType::Air; CHUNK_SIZE]; WORLD_HEIGHT]; CHUNK_SIZE],
            position
        };

        // Generar terreno para toda la columna
        for x in 0..=CHUNK_SIZE {
            for y in 0..=WORLD_HEIGHT { 
                for z in 0..=CHUNK_SIZE {
                    // Convierte coordenadas locales a mundiales
                    let world_x = (position.x * CHUNK_SIZE as i32 + x as i32) as f64 * VOXEL_SIZE as f64;
                    let world_y = y as f64 * VOXEL_SIZE as f64; 
                    let world_z = (position.y * CHUNK_SIZE as i32 + z as i32) as f64 * VOXEL_SIZE as f64;

                    // Terreno base + ruido
                    // Altura base + variacion con Perlin noise
                    // El factor 0.2 controla la frecuencia (colinas mas anchas)
                    // El factor 0.5 controla la amplitud (colinas mas suaves)
                    let height = 1.5 + perlin.get([world_x * 0.2, world_z * 0.2]) * 0.5;
                    let density = height - world_y;

                    // Densidad positiva debajo de la superficie, negativa arriba
                    chunk.densities[x][y][z] = density as f32;
                    
                    // Determinar tipo de voxel basado en densidad y altura
                    // Solo para voxels dentro del chunk (no el borde +1)
                    if x < CHUNK_SIZE && y < WORLD_HEIGHT && z < CHUNK_SIZE {
                        chunk.voxel_types[x][y][z] = VoxelType::from_density(density as f32, world_y);
                    }
                }
            }
        }

        chunk
    }

    /// Obtiene el valor de densidad en una posición local del chunk.
    /// 
    /// # Parámetros
    /// - `x`: Coordenada X local (0 a CHUNK_SIZE inclusive)
    /// - `y`: Coordenada Y local (0 a WORLD_HEIGHT inclusive)
    /// - `z`: Coordenada Z local (0 a CHUNK_SIZE inclusive)
    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[x][y][z]
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let chunk = Chunk::new(IVec2::ZERO);
        assert_eq!(chunk.position, IVec2::ZERO);
    }

    #[test]
    fn test_density_field_size() {
        let chunk = Chunk::new(IVec2::ZERO);
        assert_eq!(chunk.densities.len(), CHUNK_SIZE + 1);
        assert_eq!(chunk.densities[0].len(), WORLD_HEIGHT + 1);
        assert_eq!(chunk.densities[0][0].len(), CHUNK_SIZE + 1);
    }

    #[test]
    fn test_density_gradient() {
        let chunk = Chunk::new(IVec2::ZERO);
        // La densidad debe disminuir al subir (más aire arriba)
        let bottom = chunk.get_density(16, 0, 16);
        let top = chunk.get_density(16, WORLD_HEIGHT, 16);
        assert!(bottom > top, "Density should decrease with height");
    }

    #[test]
    fn test_chunk_position_offset() {
        let chunk1 = Chunk::new(IVec2::new(0, 0));
        let chunk2 = Chunk::new(IVec2::new(1, 0));
        
        // Los chunks adyacentes deben tener densidades diferentes debido al offset
        let d1 = chunk1.get_density(CHUNK_SIZE, 0, 0);
        let d2 = chunk2.get_density(0, 0, 0);
        // Deberían ser iguales en el borde compartido
        assert!((d1 - d2).abs() < 0.001, "Adjacent chunks should match at borders");
    }
}

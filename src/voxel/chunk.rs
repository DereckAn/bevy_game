//! 3 sistema de chunks
//! 
//! Un chunk es una unidad cubica del mundo que contiene un campo de densidad 3D
//! Los valores de densidad determinan que es solido (>0) y que es aire (<=0)

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

// ============================================================================
// CONSTANTES
// ============================================================================
// Tamaño del chunk en voxels por lado (32^3 = 32,768 voxels)
pub const CHUNK_SIZE: usize = 32;
// Escala de cada voxel en unidades del mundo 
pub const VOXEL_SIZE: f32 = 0.1;

// ============================================================================
// TIPOS
// ============================================================================

// Tipo de voxel (actualmente no usado, reservado para futuro)
#[derive(Copy, Clone, PartialEq, Default)]
pub enum Voxel {
    #[default]
    Air,
    Solid(f32), // f32 = densidad (-1.0 a 1.0)
}


/// Representa un chunk del mundo con su campo de densidad. 
/// 
/// El campo de densidad tiene tamaño 'CHUNK_SIZE + 1' para permitir
/// iterpolar correctamente en lo bordes del chunk.
#[derive(Component)]
pub struct Chunk{
    // Campo de densidad 3D. Poistivo = solido, Negativo = aire
    pub densities: [[[f32; CHUNK_SIZE + 1]; CHUNK_SIZE + 1]; CHUNK_SIZE + 1],
    // Posicion del chunk en coordenadas de chunk (no mundo)
    pub position: IVec3
}

// ============================================================================
// IMPLEMENTACIÓN
// ============================================================================

impl Chunk {
    /// Crea un nuevo chunk con terreno generado prodeduralmente
    /// 
    /// Usa ruido Perlin para generar un terreno ondulado con una altura base.
    /// 
    /// # Parametros
    /// - 'position': Posicion del chunk en coordenadas del chunk
    /// 
    /// # Ejemplo
    /// ```ignore
    /// let chunk = Chunk::new(IVec3::new(0, 0, 0));```
    pub fn new(position: IVec3) -> Self {
        let perlin = Perlin::new(12345);
        let mut chunk = Self {
            densities: [[[0.0; CHUNK_SIZE + 1]; CHUNK_SIZE + 1]; CHUNK_SIZE + 1],
            position
        };

        // Terreno simple: mitad inferior solida
        for x in 0..=CHUNK_SIZE  {
            for y in 0..=CHUNK_SIZE {
                for z in 0..=CHUNK_SIZE {
                    // Convierte coordenadas locales a mundiales
                    let world_x = (position.x * CHUNK_SIZE as i32 + x as i32) as f64 * VOXEL_SIZE as f64;
                    let world_y = (position.y * CHUNK_SIZE as i32 + y as i32) as f64 * VOXEL_SIZE as f64;
                    let world_z = (position.z * CHUNK_SIZE as i32 + z as i32) as f64 * VOXEL_SIZE as f64;

                    // Terreno base + ruido
                    // Altura base + variacion con Perlin noise
                    // El factor 0.2 controla la frecuencia (colinas mas anchas)
                    // El factor 0.5 controla la amplitud (colinas mas sueves)

                    let height = 1.5 + perlin.get([world_x * 0.2, world_z * 0.2]) * 0.5;
                    let density = height - world_y;

                    // Densidad positiva debajo de la superficie, negativa arriba
                    chunk.densities[x][y][z] = density as f32;
                }
            }
        }

        chunk
    }


    /// Obtiene el valor de densidad en una posición local del chunk.
    /// 
    /// # Parámetros
    /// - `x`, `y`, `z`: Coordenadas locales (0 a CHUNK_SIZE inclusive)
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
        let chunk = Chunk::new(IVec3::ZERO);
        assert_eq!(chunk.position, IVec3::ZERO);
    }

    #[test]
    fn test_density_field_size() {
        let chunk = Chunk::new(IVec3::ZERO);
        assert_eq!(chunk.densities.len(), CHUNK_SIZE + 1);
        assert_eq!(chunk.densities[0].len(), CHUNK_SIZE + 1);
        assert_eq!(chunk.densities[0][0].len(), CHUNK_SIZE + 1);
    }

    #[test]
    fn test_density_gradient() {
        let chunk = Chunk::new(IVec3::ZERO);
        // La densidad debe disminuir al subir (más aire arriba)
        let bottom = chunk.get_density(16, 0, 16);
        let top = chunk.get_density(16, CHUNK_SIZE, 16);
        assert!(bottom > top, "Density should decrease with height");
    }

    #[test]
    fn test_chunk_position_offset() {
        let chunk1 = Chunk::new(IVec3::new(0, 0, 0));
        let chunk2 = Chunk::new(IVec3::new(1, 0, 0));
        
        // Los chunks adyacentes deben tener densidades diferentes debido al offset
        let d1 = chunk1.get_density(CHUNK_SIZE, 0, 0);
        let d2 = chunk2.get_density(0, 0, 0);
        // Deberían ser iguales en el borde compartido
        assert!((d1 - d2).abs() < 0.001, "Adjacent chunks should match at borders");
    }
}
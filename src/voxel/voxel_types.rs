//! Sistema de tipos de voxels
//!
//! Define los diferentes materiales que pueden existir en el mundo,
//! sus propiedades físicas, y cómo se comportan.

use bevy::prelude::*;

// ============================================================================
// VOXEL TYPE ENUM
// ============================================================================

/// Tipo de voxel que representa diferentes materiales del mundo.
/// 
/// Cada tipo tiene propiedades únicas como dureza, color, y drops.
/// 
/// # Diseño
/// - `Copy + Clone`: Para copiar rápidamente sin allocaciones
/// - `PartialEq + Eq`: Para comparar tipos
/// - `Default`: Air es el valor por defecto
/// - `u8` repr: Optimización de memoria (1 byte por voxel en lugar de 8+)
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
#[repr(u8)] // Usa solo 1 byte en memoria
pub enum VoxelType {
    /// Aire - espacio vacío
    #[default]
    Air = 0,
    
    /// Tierra - material común, fácil de excavar
    Dirt = 1,
    
    /// Piedra - material duro, requiere pico
    Stone = 2,
    
    /// Madera - de árboles, requiere hacha
    Wood = 3,
    
    /// Metal - muy duro, requiere pico avanzado
    Metal = 4,
    
    /// Pasto - tierra con vegetación
    Grass = 5,
    
    /// Arena - material suave del desierto
    Sand = 6,
}

// ============================================================================
// VOXEL PROPERTIES
// ============================================================================

/// Propiedades físicas y de gameplay de un tipo de voxel.
/// 
/// # Campos
/// - `hardness`: Resistencia a destrucción (0.0 = instantáneo, 10.0 = muy duro)
/// - `color`: Color base para rendering
/// - `is_solid`: Si tiene colisión física
/// - `drops_self`: Si dropea el mismo material al destruirse
#[derive(Clone, Debug)]
pub struct VoxelProperties {
    /// Dureza del material (0.0 = muy suave, 10.0 = muy duro)
    /// 
    /// Esto afecta:
    /// - Tiempo para destruir
    /// - Qué herramienta se necesita
    /// - Cantidad de drops
    pub hardness: f32,
    
    /// Color base del voxel (usado para rendering)
    pub color: Color,
    
    /// Si el voxel es sólido (tiene colisión)
    pub is_solid: bool,
    
    /// Si dropea el mismo material al destruirse
    pub drops_self: bool,
    
    /// Nombre legible del material
    pub name: &'static str,
}

// ============================================================================
// IMPLEMENTACIÓN
// ============================================================================

impl VoxelType {
    /// Obtiene las propiedades de este tipo de voxel.
    /// 
    /// # Ejemplo
    /// ```ignore
    /// let stone = VoxelType::Stone;
    /// let props = stone.properties();
    /// println!("Hardness: {}", props.hardness); // 5.0
    /// ```
    pub fn properties(&self) -> VoxelProperties {
        match self {
            VoxelType::Air => VoxelProperties {
                hardness: 0.0,
                color: Color::srgba(0.0, 0.0, 0.0, 0.0), // Transparente
                is_solid: false,
                drops_self: false,
                name: "Air",
            },
            
            VoxelType::Dirt => VoxelProperties {
                hardness: 1.0, // Fácil de excavar
                color: Color::srgb(0.55, 0.35, 0.2), // Marrón tierra
                is_solid: true,
                drops_self: true,
                name: "Dirt",
            },
            
            VoxelType::Stone => VoxelProperties {
                hardness: 5.0, // Requiere pico
                color: Color::srgb(0.5, 0.5, 0.5), // Gris
                is_solid: true,
                drops_self: true,
                name: "Stone",
            },
            
            VoxelType::Wood => VoxelProperties {
                hardness: 2.0, // Requiere hacha (más eficiente)
                color: Color::srgb(0.4, 0.25, 0.1), // Marrón madera
                is_solid: true,
                drops_self: true,
                name: "Wood",
            },
            
            VoxelType::Metal => VoxelProperties {
                hardness: 10.0, // Muy duro, requiere pico avanzado
                color: Color::srgb(0.7, 0.7, 0.8), // Gris metálico
                is_solid: true,
                drops_self: true,
                name: "Metal",
            },
            
            VoxelType::Grass => VoxelProperties {
                hardness: 1.0, // Igual que tierra
                color: Color::srgb(0.3, 0.6, 0.2), // Verde pasto
                is_solid: true,
                drops_self: false, // Dropea tierra en su lugar
                name: "Grass",
            },
            
            VoxelType::Sand => VoxelProperties {
                hardness: 0.5, // Muy fácil de excavar
                color: Color::srgb(0.9, 0.85, 0.6), // Amarillo arena
                is_solid: true,
                drops_self: true,
                name: "Sand",
            },
        }
    }
    
    /// Verifica si este voxel es sólido (tiene colisión).
    /// 
    /// Útil para optimización: evita llamar a `properties()` completo.
    #[inline]
    pub fn is_solid(&self) -> bool {
        !matches!(self, VoxelType::Air)
    }
    
    /// Verifica si este voxel es aire.
    #[inline]
    pub fn is_air(&self) -> bool {
        matches!(self, VoxelType::Air)
    }
    
    /// Convierte un valor de densidad a un tipo de voxel.
    /// 
    /// Esta función es temporal para mantener compatibilidad con el sistema
    /// de generación actual basado en densidad.
    /// 
    /// # Lógica
    /// - Densidad > 0.0 = Sólido (elegimos tipo según altura)
    /// - Densidad <= 0.0 = Aire
    /// 
    /// # Parámetros
    /// - `density`: Valor de densidad del voxel
    /// - `world_y`: Altura en el mundo (para elegir tipo)
    pub fn from_density(density: f32, world_y: f64) -> Self {
        if density <= 0.0 {
            VoxelType::Air
        } else {
            // Elegir tipo basado en altura
            // Esto es temporal - en el futuro usaremos biomas
            if world_y < 0.5 {
                VoxelType::Stone // Profundo = piedra
            } else if world_y < 1.5 {
                VoxelType::Dirt // Medio = tierra
            } else if world_y < 1.6 {
                VoxelType::Grass // Superficie = pasto
            } else {
                VoxelType::Dirt // Por defecto
            }
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voxel_type_size() {
        // Verificar que VoxelType usa solo 1 byte
        assert_eq!(std::mem::size_of::<VoxelType>(), 1);
    }

    #[test]
    fn test_air_is_not_solid() {
        assert!(!VoxelType::Air.is_solid());
        assert!(VoxelType::Air.is_air());
    }

    #[test]
    fn test_stone_is_solid() {
        assert!(VoxelType::Stone.is_solid());
        assert!(!VoxelType::Stone.is_air());
    }

    #[test]
    fn test_hardness_values() {
        assert_eq!(VoxelType::Air.properties().hardness, 0.0);
        assert_eq!(VoxelType::Dirt.properties().hardness, 1.0);
        assert_eq!(VoxelType::Stone.properties().hardness, 5.0);
        assert_eq!(VoxelType::Metal.properties().hardness, 10.0);
    }

    #[test]
    fn test_from_density() {
        // Aire
        assert_eq!(VoxelType::from_density(-1.0, 2.0), VoxelType::Air);
        
        // Piedra (profundo)
        assert_eq!(VoxelType::from_density(1.0, 0.0), VoxelType::Stone);
        
        // Tierra (medio)
        assert_eq!(VoxelType::from_density(1.0, 1.0), VoxelType::Dirt);
        
        // Pasto (superficie)
        assert_eq!(VoxelType::from_density(1.0, 1.55), VoxelType::Grass);
    }
}

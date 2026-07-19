//! Sistema de herramientas
//!
//! Define las herramientas que el jugador puede usar para destruir voxels.
//! Y sus efectividades contra diferentes materiales.

use crate::voxel::voxel_types::VoxelType;
use bevy::prelude::*;

// ============================================================================
// TOOL TYPE ENUM
// ============================================================================

/// Tipo de herramienta que el jugador puede usar.
///
/// Cada herramienta tienen efeciencia diferente contra diferentes materiales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolType {
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    None,
}

// ============================================================================
// TOOL PROPERTIES
// ============================================================================

#[derive(Clone, Debug)]
pub struct ToolProperties {
    /// Nombre de la herramienta.
    // El HUD usa iconos, no el nombre; reservado para tooltips/menús.
    #[allow(dead_code)]
    pub name: &'static str,

    /// Durabilidad maxima de la herramienta.
    pub max_durability: u32,

    /// Eficiencia de la herramienta.
    pub speed_multiplier: f32,
}

// ============================================================================
// IMPLEMENTACIÓN
// ============================================================================

impl ToolType {
    /// Obtiene las propiedades de esta herramienta.
    pub fn properties(&self) -> ToolProperties {
        match self {
            ToolType::Pickaxe => ToolProperties {
                name: "Pickaxe",
                max_durability: 100,
                speed_multiplier: 1.0,
            },
            ToolType::Axe => ToolProperties {
                name: "Axe",
                max_durability: 100,
                speed_multiplier: 1.0,
            },
            ToolType::Shovel => ToolProperties {
                name: "Shovel",
                max_durability: 100,
                speed_multiplier: 1.0,
            },
            ToolType::Hoe => ToolProperties {
                name: "Hoe",
                max_durability: 100,
                speed_multiplier: 1.0,
            },
            ToolType::None => ToolProperties {
                name: "Hands",
                max_durability: 0,     // Infinito
                speed_multiplier: 0.5, // Muy lento
            },
        }
    }

    /// Calcula la efictividad de esta herramienta contra  un tipo de voxel
    ///
    /// REtorna un multiplicador:
    ///
    /// - 1.0: La herramienta es eficiente
    /// - 0.5: La herramienta es poco eficiente
    /// - 0.0: La herramienta es ineficiente

    pub fn effectiveness_against(&self, voxel_type: VoxelType) -> f32 {
        match (self, voxel_type) {
            // Aire no necesita herramienta
            (_, VoxelType::Air) => 1.0,

            // Pico es bueno contra piedra y metal
            (ToolType::Pickaxe, VoxelType::Stone) => 1.5,
            (ToolType::Pickaxe, VoxelType::Metal) => 1.5,

            // Hacha es buena contra madera
            (ToolType::Axe, VoxelType::Wood | VoxelType::PineWood) => 1.5,

            // Pala es buena contra tierra, pasto y arena
            (ToolType::Shovel, VoxelType::Dirt) => 1.5,
            (ToolType::Shovel, VoxelType::Grass) => 1.5,
            (ToolType::Shovel, VoxelType::Sand) => 1.5,

            // Manos desnudas son malas contra todo
            (ToolType::None, _) => 0.3,

            // Herramienta incorrecta
            _ => 0.3,
        }
    }

    /// Obtiene el patrón de destrucción para esta herramienta
    ///
    /// Retorna una lista de posiciones relativas que se destruirán.
    ///
    /// Esfera de radio `BREAK_RADIUS` (igual para todas las herramientas): todos
    /// los offsets cuya distancia al centro cabe en el radio. Sube el radio para
    /// romper cráteres más grandes.
    pub fn get_destruction_pattern(&self) -> Vec<IVec3> {
        /// Radio de la esfera de destrucción, en voxels.
        const BREAK_RADIUS: i32 = 2;

        let mut pattern = Vec::new();
        for x in -BREAK_RADIUS..=BREAK_RADIUS {
            for y in -BREAK_RADIUS..=BREAK_RADIUS {
                for z in -BREAK_RADIUS..=BREAK_RADIUS {
                    if x * x + y * y + z * z <= BREAK_RADIUS * BREAK_RADIUS {
                        pattern.push(IVec3::new(x, y, z));
                    }
                }
            }
        }
        pattern
    }
}

// ============================================================================
// TOOL COMPONENT
// ============================================================================

/// Componente que representa una herramienta equipada
///
/// Se adjuna a una entidad (jugador) para indicar que herramienta esta usando.
#[derive(Component, Debug)]
pub struct Tool {
    /// Tipo de herramienta equipada
    pub tool_type: ToolType,

    /// Durabilidad actual (0 = rota    )
    pub current_durability: u32,
}

impl Tool {
    /// Crea una herramienta con durabilidad maxima.
    pub fn new(tool_type: ToolType) -> Self {
        let max_durability = tool_type.properties().max_durability;
        Self {
            tool_type,
            current_durability: max_durability,
        }
    }

    /// Reduce la durabilidad de la herramienta.
    ///
    /// Retorna "true" si la herramienta se rompio.
    pub fn damage(&mut self, amount: u32) -> bool {
        // Manos no se rompen
        if self.tool_type == ToolType::None {
            return false;
        }

        self.current_durability = self.current_durability.saturating_sub(amount);
        self.current_durability == 0
    }

    /// Verifica si la herramienta esta rota.
    // Consultas de durabilidad para el HUD; el sistema de durabilidad aún no está conectado.
    #[allow(dead_code)]
    pub fn is_broken(&self) -> bool {
        self.tool_type != ToolType::None && self.current_durability == 0
    }

    /// Obtiene el porcentaje de durabilidad restante (0.0 - 1.0)
    #[allow(dead_code)]
    pub fn get_durability_percentage(&self) -> f32 {
        if self.tool_type == ToolType::None {
            return 1.0; // Manos nunca se rompen
        }

        let max = self.tool_type.properties().max_durability;
        if max == 0 {
            return 1.0; // Evita division por cero
        }

        self.current_durability as f32 / max as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destruction_pattern_is_a_sphere() {
        let pattern = ToolType::Pickaxe.get_destruction_pattern();
        assert!(pattern.contains(&IVec3::ZERO)); // centro
        assert!(pattern.contains(&IVec3::new(2, 0, 0))); // borde sobre eje (dist²=4)
        assert!(!pattern.contains(&IVec3::new(2, 2, 2))); // esquina (dist²=12) fuera
    }
}

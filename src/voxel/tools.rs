//! Sistema de herramientas 
//! 
//! Define las herramientas que el jugador puede usar para destruir voxels.
//! Y sus efectividades contra diferentes materiales. 

use bevy::prelude::*;
use crate::voxel::voxel_types::VoxelType;

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
    pub name: &'static str,

    /// Durabilidad maxima de la herramienta.
    pub max_durability: u32,

    /// Eficiencia de la herramienta.
    pub speed_multiplier: f32,
}


// ============================================================================
// IMPLEMENTACIÓN
// ============================================================================


impl ToolType{
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
                max_durability: 0, // Infinito 
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
            (ToolType::Axe, VoxelType::Wood) => 1.5,
            
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
    pub fn is_broken(&self) -> bool {
        self.tool_type != ToolType::None && self.current_durability == 0
    }

    /// Obtiene el porcentaje de durabilidad restante (0.0 - 1.0)
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
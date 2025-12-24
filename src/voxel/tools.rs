//! Sistema de herramientas 
//! 
//! Define las herramientas que el jugador puede usar para destruir voxels.
//! Y sus efectividades contra diferentes materiales. 

use bevy::prelude::*;
use crate::voxel::voxel_types::VoxelType;
use rand::Rng;

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

    /// Calcula cuántos voxels se obtienen al destruir con esta herramienta
    /// 
    /// Retorna un rango aleatorio basado en la herramienta y tipo de voxel
    pub fn calculate_drops(&self, voxel_type: VoxelType) -> u32 {
        let mut rng = rand::thread_rng();
        
        let (min, max) = match (self, voxel_type) {
            // Aire no da drops
            (_, VoxelType::Air) => (0, 0),
            
            // Manos desnudas (muy poco eficiente)
            (ToolType::None, VoxelType::Stone) => (0, 1),
            (ToolType::None, VoxelType::Metal) => (0, 0),
            (ToolType::None, VoxelType::Dirt | VoxelType::Grass | VoxelType::Sand) => (2, 3),
            (ToolType::None, VoxelType::Wood) => (1, 2),
            
            // Pala (buena para tierra/arena)
            (ToolType::Shovel, VoxelType::Dirt | VoxelType::Grass | VoxelType::Sand) => (8, 15),
            (ToolType::Shovel, VoxelType::Stone) => (2, 4),
            (ToolType::Shovel, VoxelType::Wood) => (3, 5),
            (ToolType::Shovel, VoxelType::Metal) => (0, 1),
            
            // Pico (bueno para piedra/metal)
            (ToolType::Pickaxe, VoxelType::Stone) => (8, 15),
            (ToolType::Pickaxe, VoxelType::Metal) => (3, 8),
            (ToolType::Pickaxe, VoxelType::Dirt | VoxelType::Grass | VoxelType::Sand) => (5, 8),
            (ToolType::Pickaxe, VoxelType::Wood) => (4, 6),
            
            // Hacha (excelente para madera)
            (ToolType::Axe, VoxelType::Wood) => (10, 30),
            (ToolType::Axe, VoxelType::Dirt | VoxelType::Grass | VoxelType::Sand) => (6, 10),
            (ToolType::Axe, VoxelType::Stone) => (3, 6),
            (ToolType::Axe, VoxelType::Metal) => (1, 3),
            
            // Azada (herramienta especial, por ahora como pala)
            (ToolType::Hoe, voxel) => {
                // Reutilizar lógica de pala
                return ToolType::Shovel.calculate_drops(voxel);
            }
        };
        
        if min >= max {
            min
        } else {
            rng.gen_range(min..=max)
        }
    }

    /// Obtiene el patrón de destrucción para esta herramienta
    /// 
    /// Retorna una lista de posiciones relativas que se destruirán
    pub fn get_destruction_pattern(&self) -> Vec<IVec3> {
        match self {
            // Manos: solo 1 voxel
            ToolType::None => vec![IVec3::ZERO],
            
            // Pala: cráter horizontal (excavación)
            ToolType::Shovel => vec![
                IVec3::new(0, 0, 0),   // Centro
                IVec3::new(1, 0, 0),   // Derecha
                IVec3::new(-1, 0, 0),  // Izquierda
                IVec3::new(0, 0, 1),   // Adelante
                IVec3::new(0, 0, -1),  // Atrás
                IVec3::new(0, -1, 0),  // Abajo (simula excavación)
            ],
            
            // Pico: cráter cónico (picotazo)
            ToolType::Pickaxe => vec![
                IVec3::new(0, 0, 0),   // Centro
                IVec3::new(1, 0, 0),   // Derecha
                IVec3::new(-1, 0, 0),  // Izquierda
                IVec3::new(0, 1, 0),   // Arriba
                IVec3::new(0, -1, 0),  // Abajo
                IVec3::new(0, 0, 1),   // Adelante
                IVec3::new(0, 0, -1),  // Atrás
            ],
            
            // Hacha: cráter vertical (cortar tronco)
            ToolType::Axe => vec![
                IVec3::new(0, 0, 0),   // Centro
                IVec3::new(0, 1, 0),   // Arriba
                IVec3::new(0, -1, 0),  // Abajo
                IVec3::new(1, 0, 0),   // Derecha
                IVec3::new(-1, 0, 0),  // Izquierda
                IVec3::new(0, 2, 0),   // Más arriba
                IVec3::new(0, -2, 0),  // Más abajo
                IVec3::new(1, 1, 0),   // Diagonal
                IVec3::new(-1, 1, 0),  // Diagonal
                IVec3::new(1, -1, 0),  // Diagonal
                IVec3::new(-1, -1, 0), // Diagonal
            ],
            
            // Azada: como pala por ahora
            ToolType::Hoe => ToolType::Shovel.get_destruction_pattern(),
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
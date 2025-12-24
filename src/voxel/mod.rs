//! # Modulo de Voxels
//! 
//! Sistema de terreno basado en voxels usando el algoritmo Surface Nets
//! para generar meshessuaves a partir de campo de densidad. 
//! 
//! ## Estructura
//! - 'chunk': Define la estructura de datos del chunk y generacion de terreno
//! - 'meshing': Contiene las funciones para generar mallas suaves a partir de los datos de densidad
//! - 'voxel_types': Define los tipos de materiales y sus propiedades


pub mod chunk;
pub mod meshing;
pub mod voxel_types;
pub mod tools;
pub mod destruction;

pub use chunk::*;
pub use meshing::*;
pub use voxel_types::*;
pub use tools::*;
pub use destruction::*;
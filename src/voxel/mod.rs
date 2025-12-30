//! # Modulo de Voxels
//! 
//! Sistema de terreno basado en voxels con greedy meshing optimizado
//! 
//! ## Estructura
//! - 'greedy_meshing': Algoritmo de meshing optimizado que reduce triangulos 70-95%
//! - 'voxel_types': Define los tipos de materiales y sus propiedades
//! - 'tools': Herramientas para interactuar con voxels
//! - 'destruction': Sistema de destruccion de voxels
//! - 'lod_system': Sistema de nivel de detalle (LOD) para chunks
//! - 'dynamic_chunks': Chunks base de 32³ con generacion de terreno

pub mod greedy_meshing;
pub mod voxel_types;
pub mod tools;
pub mod destruction;
pub mod lod_system;
pub mod dynamic_chunks;
pub mod chunk_loading;

pub use greedy_meshing::*;
pub use voxel_types::*;
pub use tools::*;
pub use destruction::*;
pub use lod_system::*;
pub use dynamic_chunks::BaseChunk;
pub use chunk_loading::*;
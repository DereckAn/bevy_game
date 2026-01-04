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

pub mod biomes;
pub mod chunk_cache;
pub mod chunk_loading;
pub mod destruction;
pub mod downsampling;
pub mod dynamic_chunks;
pub mod frustum_culling;
pub mod greedy_meshing;
pub mod lod_chunks;
pub mod lod_system;
pub mod octree;
pub mod tools;
pub mod voxel_types;

pub use biomes::*;
pub use chunk_cache::*;
pub use chunk_loading::*;
pub use destruction::*;
pub use downsampling::*;
pub use dynamic_chunks::BaseChunk;
pub use frustum_culling::*;
pub use greedy_meshing::*;
pub use lod_chunks::*;
pub use lod_system::*;
pub use octree::*;
pub use tools::*;
pub use voxel_types::*;

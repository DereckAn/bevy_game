// Exporta todos los módulos para tests y reutilización
pub mod core;
pub mod voxel;
pub mod player;

// Re-exporta los plugins principales para facilitar el uso
pub use player::PlayerPlugin;
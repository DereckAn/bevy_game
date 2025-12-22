// Exporta todos los módulos para tests y reutilización
pub mod core;
pub mod voxel;
pub mod player;
pub mod physics;
pub mod debug;

// Re-exporta los plugins principales para facilitar el uso
pub use player::PlayerPlugin;
pub use physics::PhysicsPlugin;
pub use debug::DebugPlugin;
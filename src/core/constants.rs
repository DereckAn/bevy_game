// Constantes globales del juego

// Voxel system - Dynamic chunk system inspired by "Lay of the Land"
pub const BASE_CHUNK_SIZE: usize = 32; // Base chunk size (like Lay of the Land)
pub const MAX_WORLD_HEIGHT: usize = 2048; // 64 vertical chunks (32 * 64)
pub const VOXEL_SIZE: f32 = 0.1; // 10cm voxels for detailed construction

// Dynamic chunk merging
pub const MAX_MERGE_SIZE: usize = 16; // Up to 16x16x16 chunks can merge (512³ voxels)
pub const LOD_DISTANCES: [f32; 5] = [50.0, 100.0, 200.0, 400.0, 800.0]; // LOD transition distances

// Physics
pub const GRAVITY: f32 = -9.81;
pub const JUMP_FORCE: f32 = 5.0;
pub const GROUND_CHECK_DISTANCE: f32 = 0.1;

// Player
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_RADIUS: f32 = 0.3;

// World architecture - NEW
pub const MAX_LOADED_WORLDS: usize = 10; // Maximum worlds loaded simultaneously
pub const MEMORY_BUDGET_GB: usize = 4; // Total memory budget for all worlds
pub const WORLD_LOAD_TIMEOUT_SECONDS: f32 = 5.0; // Maximum time to load a world
pub const TELEPORT_TIMEOUT_SECONDS: f32 = 1.0; // Maximum time for teleportation
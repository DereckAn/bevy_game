// Constantes globales del juego

// ============================================================================
// SISTEMA DE CHUNKS DINÁMICOS
// ============================================================================

// Chunk base de 32³ como "Lay of the Land"
pub const BASE_CHUNK_SIZE: usize = 32;

// Tamaño de voxel en metros
pub const VOXEL_SIZE: f32 = 0.1;

// Altura máxima del mundo (2048 voxels = 204.8 metros)
pub const MAX_WORLD_HEIGHT: usize = 2048;

// Número de chunks verticales (2048 ÷ 32 = 64 chunks de altura)
pub const VERTICAL_CHUNKS: usize = MAX_WORLD_HEIGHT / BASE_CHUNK_SIZE;

// Distancias para cada nivel de LOD (Level of Detail)
// Ajustadas para chunks de 3.2m (32 voxels * 0.1m)
pub const LOD_DISTANCES: [f32; 5] = [
    16.0,  // Ultra: 0-16 metros (~5 chunks)
    32.0,  // High: 16-32 metros (~10 chunks)
    64.0,  // Medium: 32-64 metros (~20 chunks)
    128.0, // Low: 64-128 metros (~40 chunks)
    256.0, // Minimal: 128+ metros (~80+ chunks)
];

// Tamaño máximo cuando se combinan chunks (16x16x16 = 512³)
pub const MAX_MERGED_SIZE: usize = 512;

// ============================================================================
// CONSTANTES LEGACY (mantener por compatibilidad)
// ============================================================================

// Alias para compatibilidad con código existente
pub const CHUNK_SIZE: usize = BASE_CHUNK_SIZE;

// Physics
pub const GRAVITY: f32 = -9.81;
pub const JUMP_FORCE: f32 = 5.0;
pub const GROUND_CHECK_DISTANCE: f32 = 0.1;

// Player
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_RADIUS: f32 = 0.3;

// Constantes globales del juego

// ============================================================================
// SISTEMA DE CHUNKS DINÁMICOS
// ============================================================================

// Chunk base de 32³ como "Lay of the Land"
pub const BASE_CHUNK_SIZE: usize = 32;

// Radio del mundo en chunks desde el origen (mapa FINITO).
// 128 → mapa de ~256×256 chunks (~820 m de lado con VOXEL_SIZE=0.1).
// Más allá de este límite no se generan chunks.
pub const WORLD_CHUNK_RADIUS: i32 = 128;

// Tamaño de voxel en metros
pub const VOXEL_SIZE: f32 = 0.1;

// Distancias para cada nivel de LOD (Level of Detail)
// Ajustadas para Distant Horizons style rendering (64 chunk radius)
pub const LOD_DISTANCES: [f32; 5] = [
    32.0,  // Ultra: 0-32 metros (~10 chunks)
    64.0,  // High: 32-64 metros (~20 chunks)
    128.0, // Medium: 64-128 metros (~40 chunks)
    192.0, // Low: 128-192 metros (~60 chunks)
    256.0, // Minimal: 192+ metros (~80+ chunks)
];

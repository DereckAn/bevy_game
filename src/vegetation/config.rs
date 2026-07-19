//! Configuración de vegetación — el único lugar para ajustar colores y activar/
//! desactivar tipos de vegetación. Son constantes: **edita y recompila** para
//! aplicar los cambios.
//!
//! Colores en sRGB como `[r, g, b]` con cada canal en 0.0..=1.0.

// ============================================================================
// ACTIVAR / DESACTIVAR (por tipo)
// ============================================================================

/// Árboles grandes y pequeños (pinos, robles, arbustitos tipo árbol).
pub const ENABLE_TREES: bool = true;

/// Arbustos (montículos de follaje atravesable).
pub const ENABLE_BUSHES: bool = true;

/// Tufos de pasto (follaje atravesable sobre el suelo).
pub const ENABLE_GRASS: bool = true;

// ============================================================================
// COLORES (sRGB, 0.0..=1.0)
// ============================================================================

/// Tronco y ramas de roble (madera). Color base #733805; el pintado genera una
/// paleta tonal (más oscuro↔más claro) a partir de él, ver [`DARK_MUL`]/[`LIGHT_MUL`].
pub const WOOD_COLOR: [f32; 3] = [0.451, 0.220, 0.020];

// Tronco y ramas de pino (madera más oscura).
pub const PINE_WOOD_COLOR: [f32; 3] = [1.15, 0.56, 0.05];

/// Paleta tonal de la madera: multiplicadores de brillo del color base para el
/// extremo oscuro (sombra) y el claro (luz). El pintado interpola entre ambos por
/// voxel/quad, así cada árbol y cada banda del tronco varía sin cambiar el tono.
pub const DARK_MUL: f32 = 0.7;
pub const LIGHT_MUL: f32 = 1.25;

/// Copas de los robles (hojas genéricas).
pub const LEAVES_COLOR: [f32; 3] = [0.2, 0.8, 0.2];

/// Acículas de los pinos (verde oscuro).
pub const PINE_COLOR: [f32; 3] = [0.08, 0.30, 0.12];

/// Hojas de los árboles pequeños (verde más claro).
pub const SMALL_LEAVES_COLOR: [f32; 3] = [0.45, 0.80, 0.35];

/// Tufos de pasto.
pub const GRASS_COLOR: [f32; 3] = [0.20, 0.55, 0.15];

/// Arbustos (verde más oscuro para distinguirlos del pasto).
pub const BUSH_COLOR: [f32; 3] = [0.10, 0.32, 0.10];

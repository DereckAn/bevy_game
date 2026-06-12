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

/// Tronco y ramas (madera).
pub const WOOD_COLOR: [f32; 3] = [0.4, 0.25, 0.1];

/// Copas de los árboles (hojas de pinos/robles/pequeños).
pub const LEAVES_COLOR: [f32; 3] = [0.2, 0.8, 0.2];

/// Tufos de pasto.
pub const GRASS_COLOR: [f32; 3] = [0.20, 0.55, 0.15];

/// Arbustos (verde más oscuro para distinguirlos del pasto).
pub const BUSH_COLOR: [f32; 3] = [0.10, 0.32, 0.10];

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

/// Copas de los robles (hojas genéricas).
pub const LEAVES_COLOR: [f32; 3] = [0.2, 0.8, 0.2];

/// Acículas de los pinos (verde oscuro).
pub const PINE_COLOR: [f32; 3] = [0.08, 0.30, 0.12];

/// Hojas de los árboles pequeños (verde más claro).
pub const SMALL_LEAVES_COLOR: [f32; 3] = [0.45, 0.80, 0.35];

/// Tufos de pasto.
pub const GRASS_COLOR: [f32; 3] = [0.20, 0.55, 0.15];

/// Tufos de pasto seco del desierto (#E49E49).
pub const DESERT_GRASS_COLOR: [f32; 3] = [0.894, 0.620, 0.286];

/// Arbustos (verde más oscuro para distinguirlos del pasto).
pub const BUSH_COLOR: [f32; 3] = [0.10, 0.32, 0.10];

/// Arbustos secos del desierto (#9C5906).
pub const DESERT_BUSH_COLOR: [f32; 3] = [0.612, 0.349, 0.024];

/// Cactus (verde saguaro, algo azulado/apagado).
pub const CACTUS_COLOR: [f32; 3] = [0.24, 0.45, 0.26];

// Paleta pizarra-azulada del bioma helado (de claro a oscuro):
// #d5dbe2 nieve · #b3bfcb hielo · #778ca4 copas · #4f6271 troncos · (#313c45 libre)

/// Nieve de superficie: el tono más claro de la paleta (#d5dbe2).
pub const SNOW_COLOR: [f32; 3] = [0.835, 0.859, 0.886];

/// Hielo bajo la nieve: gris-azul claro, algo más oscuro que la nieve (#b3bfcb).
pub const ICE_COLOR: [f32; 3] = [0.702, 0.749, 0.796];

/// Tronco/ramas de los árboles del bioma helado: pizarra oscura, contrasta
/// contra la nieve (#4f6271).
pub const WHITE_WOOD_COLOR: [f32; 3] = [0.310, 0.384, 0.443];

/// Copas de los árboles del bioma helado: pizarra media azulada (#778ca4).
pub const WHITE_LEAVES_COLOR: [f32; 3] = [0.467, 0.549, 0.643];

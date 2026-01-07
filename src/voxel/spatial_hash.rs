//! Spatial Hash Grid para búsqueda rápida de chunks en mundos voxel
//!
//! Este sistema divide el mundo en una cuadrícula 2D (ignorando Y) donde cada celda
//! contiene una lista de chunks. Esto permite búsquedas O(1) en lugar de O(log n).
//!
//! Perfecto para mundos tipo Minecraft donde la búsqueda es principalmente horizontal.

use bevy::prelude::*;
use std::collections::HashMap;

/// Tamaño de cada celda del grid en chunks
/// 16 significa que cada celda contiene hasta 16x16 chunks horizontales
const CELL_SIZE: i32 = 16;

/// Spatial Hash Grid para búsqueda rápida de chunks
///
/// Divide el mundo en celdas 2D (solo X y Z) y almacena qué chunks
/// están en cada celda. Cada celda puede contener chunks de cualquier altura Y.
#[derive(Resource)]
pub struct SpatialHashGrid {
    /// Tamaño de cada celda del grid (en chunks)
    cell_size: i32,

    /// HashMap: cell_key (IVec2) -> Vec<IVec3> (chunks en esa celda)
    /// La key es la posición de la celda, el valor es la lista de chunks
    cells: HashMap<IVec2, Vec<IVec3>>,

    /// Contador de chunks totales (para debug/stats)
    total_chunks: usize,
}

impl Default for SpatialHashGrid {
    fn default() -> Self {
        Self::new(CELL_SIZE)
    }
}

impl SpatialHashGrid {
    /// Crea un nuevo Spatial Hash Grid con el tamaño de celda especificado
    pub fn new(cell_size: i32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            total_chunks: 0,
        }
    }

    /// Convierte una posición de chunk (3D) a una celda del grid (2D)
    /// Solo usa X y Z, ignorando Y
    #[inline]
    fn chunk_to_cell(&self, chunk_pos: IVec3) -> IVec2 {
        IVec2::new(
            chunk_pos.x.div_euclid(self.cell_size),
            chunk_pos.z.div_euclid(self.cell_size),
        )
    }

    /// Inserta un chunk en el grid - O(1)
    pub fn insert(&mut self, chunk_pos: IVec3) {
        let cell = self.chunk_to_cell(chunk_pos);
        let chunks = self.cells.entry(cell).or_insert_with(Vec::new);

        // Evitar duplicados
        if !chunks.contains(&chunk_pos) {
            chunks.push(chunk_pos);
            self.total_chunks += 1;
        }
    }

    /// Remueve un chunk del grid - O(1) amortizado
    pub fn remove(&mut self, chunk_pos: IVec3) -> bool {
        let cell = self.chunk_to_cell(chunk_pos);

        if let Some(chunks) = self.cells.get_mut(&cell) {
            if let Some(index) = chunks.iter().position(|&pos| pos == chunk_pos) {
                chunks.swap_remove(index);
                self.total_chunks -= 1;

                // Limpiar celda vacía para ahorrar memoria
                if chunks.is_empty() {
                    self.cells.remove(&cell);
                }

                return true;
            }
        }

        false
    }

    /// Busca todos los chunks dentro de un radio HORIZONTAL desde el centro
    /// Ignora la distancia en Y - perfecto para chunk loading
    ///
    /// Complejidad: O(c + k) donde c = celdas a verificar, k = chunks en resultado
    /// Para radio 64 con cell_size 16: c ≈ 25 celdas
    pub fn query_radius_horizontal(&self, center: IVec3, radius: i32) -> Vec<IVec3> {
        let mut results = Vec::new();

        // Calcular qué celdas pueden intersectar el radio
        let cell_center = self.chunk_to_cell(center);
        let cell_radius = (radius / self.cell_size) + 1;

        let radius_sq = radius * radius;

        // Solo iterar celdas que pueden contener chunks en el radio
        for cx in -cell_radius..=cell_radius {
            for cz in -cell_radius..=cell_radius {
                let cell = cell_center + IVec2::new(cx, cz);

                // Si la celda existe, verificar sus chunks
                if let Some(chunks) = self.cells.get(&cell) {
                    for &chunk_pos in chunks {
                        // Calcular distancia HORIZONTAL (solo X y Z)
                        let dx = chunk_pos.x - center.x;
                        let dz = chunk_pos.z - center.z;
                        let distance_sq = dx * dx + dz * dz;

                        if distance_sq <= radius_sq {
                            results.push(chunk_pos);
                        }
                    }
                }
            }
        }

        results
    }

    /// Obtiene el número total de chunks en el grid
    pub fn len(&self) -> usize {
        self.total_chunks
    }

    /// Verifica si el grid está vacío
    pub fn is_empty(&self) -> bool {
        self.total_chunks == 0
    }

    /// Obtiene el número de celdas activas (con al menos un chunk)
    pub fn active_cells(&self) -> usize {
        self.cells.len()
    }

    /// Limpia todas las celdas vacías para liberar memoria
    /// Útil para ejecutar periódicamente
    pub fn cleanup_empty_cells(&mut self) {
        self.cells.retain(|_, chunks| !chunks.is_empty());
    }

    /// Limpia completamente el grid
    pub fn clear(&mut self) {
        self.cells.clear();
        self.total_chunks = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_query() {
        let mut grid = SpatialHashGrid::new(16);

        // Insertar algunos chunks
        grid.insert(IVec3::new(0, 0, 0));
        grid.insert(IVec3::new(1, 5, 1));
        grid.insert(IVec3::new(50, 10, 50));

        assert_eq!(grid.len(), 3);

        // Query cerca del origen
        let results = grid.query_radius_horizontal(IVec3::ZERO, 5);
        assert!(results.contains(&IVec3::new(0, 0, 0)));
        assert!(results.contains(&IVec3::new(1, 5, 1)));
        assert!(!results.contains(&IVec3::new(50, 10, 50)));
    }

    #[test]
    fn test_remove() {
        let mut grid = SpatialHashGrid::new(16);

        let pos = IVec3::new(10, 20, 30);
        grid.insert(pos);
        assert_eq!(grid.len(), 1);

        assert!(grid.remove(pos));
        assert_eq!(grid.len(), 0);

        // Remover de nuevo debería retornar false
        assert!(!grid.remove(pos));
    }

    #[test]
    fn test_horizontal_distance_only() {
        let mut grid = SpatialHashGrid::new(16);

        // Chunks a diferentes alturas pero misma posición horizontal
        grid.insert(IVec3::new(0, 0, 0));
        grid.insert(IVec3::new(0, 100, 0)); // Muy alto
        grid.insert(IVec3::new(0, -50, 0)); // Muy bajo

        // Todos deberían estar en el radio porque Y no importa
        let results = grid.query_radius_horizontal(IVec3::ZERO, 1);
        assert_eq!(results.len(), 3);
    }
}

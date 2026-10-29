//! Trazado de rayos sobre una tesela de columnas (terreno lejano).
//!
//! Referencia en Rust del shader de terreno lejano (fase 1 de
//! `docs/raymarch_far_terrain_plan.md`): el WGSL será un port de `trace`.
//! Una tesela = 64×64 columnas; cada columna es sólida desde el fondo de la caja
//! hasta su altura. Todo en unidades de celda: x/z en [0, 64), y en las mismas
//! unidades (el shader convierte de mundo a tesela antes de trazar).

use bevy::math::{IVec2, IVec3, UVec2, Vec3};

pub const TILE_CELLS: usize = 64;
/// Niveles de la cadena de máximos: 64², 32², …, 1².
pub const MIP_COUNT: usize = TILE_CELLS.trailing_zeros() as usize + 1;
/// Tope de iteraciones por rayo: el shader necesita acotar el coste por píxel.
const MAX_STEPS: u32 = 512;
/// Holgura (celdas) bajo la superficie más baja del borde: un vecino de nivel
/// más grueso redondea su altura hasta 1 celda propia, más 1 de margen.
const BORDER_MARGIN: f32 = 2.0;

/// Alturas de una tesela con su cadena de máximos.
pub struct TileHeights {
    /// `mips[m]` tiene (64>>m)² alturas; cada una es el máximo de los 2^m×2^m
    /// columnas de debajo. Si el rayo pasa por encima de ese máximo, el nodo
    /// entero está vacío para él y se salta de un paso.
    pub mips: Vec<Vec<f32>>,
    /// Fondo de la caja: altura mínima − 1 (las columnas llegan hasta aquí).
    pub bottom: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Distancia a lo largo de `dir` (normalizada) desde el origen.
    pub t: f32,
    pub cell: UVec2,
    /// +Y si pega en la cima; ±X/±Z si pega en un lateral.
    pub normal: IVec3,
}

impl TileHeights {
    /// `heights` en orden fila a fila: índice `x + z * 64`.
    pub fn new(heights: Vec<f32>) -> Self {
        assert_eq!(heights.len(), TILE_CELLS * TILE_CELLS);
        let bottom = heights.iter().copied().fold(f32::INFINITY, f32::min) - 1.0;
        let mut mips = vec![heights];
        for m in 1..MIP_COUNT {
            let prev = &mips[m - 1];
            let n = TILE_CELLS >> m;
            let pn = n * 2;
            let next = (0..n * n)
                .map(|i| {
                    let (x, z) = (i % n * 2, i / n * 2);
                    prev[x + z * pn]
                        .max(prev[x + 1 + z * pn])
                        .max(prev[x + (z + 1) * pn])
                        .max(prev[x + 1 + (z + 1) * pn])
                })
                .collect();
            mips.push(next);
        }
        Self { mips, bottom }
    }

    /// Baja el fondo de la caja hasta `border_min` (altura más baja de los
    /// vecinos junto al borde) − margen. Así los laterales de las columnas de
    /// esta tesela llegan hasta la superficie del vecino y no queda una rendija
    /// visible entre teselas o niveles a distinta altura.
    pub fn lower_bottom_to(&mut self, border_min: f32) {
        self.bottom = self.bottom.min(border_min - BORDER_MARGIN);
    }

    fn top(&self) -> f32 {
        self.mips[MIP_COUNT - 1][0]
    }

    fn height(&self, level: usize, node: IVec2) -> f32 {
        self.mips[level][node.x as usize + node.y as usize * (TILE_CELLS >> level)]
    }
}

/// Traza un rayo (`dir` normalizada) por la tesela con la cadena de máximos:
/// sube de nivel tras cada salto por encima de un nodo y baja al no poder
/// descartarlo, hasta resolver una columna concreta en el nivel 0.
///
/// El nodo actual se lleva como índice ENTERO y se avanza ±1 por el eje de
/// salida: recalcularlo desde la posición en coma flotante puede dejarlo en el
/// mismo nodo cuando el rayo sale justo por un borde (bucle infinito).
pub fn trace(tile: &TileHeights, origin: Vec3, dir: Vec3) -> Option<Hit> {
    let (mut t, t_far, mut axis) = box_span(tile, origin, dir)?;
    let last = TILE_CELLS as i32 - 1;
    let entry = origin + dir * t;
    // Celda fina (nivel 0) en la que está el rayo; su nodo de nivel L es cell >> L.
    let mut cell = IVec2::new(
        cell_index(entry.x, dir.x).clamp(0, last),
        cell_index(entry.z, dir.z).clamp(0, last),
    );
    let mut level = MIP_COUNT - 1;
    for _ in 0..MAX_STEPS {
        let size = 1i32 << level;
        let node = cell >> level as i32;
        let (t_exit, exit_axis) = node_exit(origin, dir, node, size, t_far);
        let h = tile.height(level, node);
        if lowest_y(origin, dir, t, t_exit) > h {
            if exit_axis == 1 {
                return None;
            }
            // Ejes de la tesela (x, z) ↔ ejes del mundo (0, 2).
            let a = exit_axis / 2;
            let b = 1 - a;
            let next = node[a] + if dir[exit_axis] > 0.0 { 1 } else { -1 };
            if next < 0 || next >= (TILE_CELLS >> level) as i32 {
                return None;
            }
            // Celda fina pegada al borde que se cruza; en el otro eje, la de la
            // posición de salida, acotada al nodo (no cambia por ese eje).
            cell[a] = if next > node[a] {
                next * size
            } else {
                (next + 1) * size - 1
            };
            let p = origin + dir * t_exit;
            cell[b] =
                cell_index(p[b * 2], dir[b * 2]).clamp(node[b] * size, (node[b] + 1) * size - 1);
            t = t.max(t_exit);
            axis = exit_axis;
            level = (level + 1).min(MIP_COUNT - 1);
        } else if level > 0 {
            level -= 1;
        } else {
            return Some(column_hit(origin, dir, t, axis, h, cell.as_uvec2()));
        }
    }
    None
}

/// Tramo `[t_near, t_far]` del rayo dentro de la caja de la tesela y el eje por
/// el que entra. Con el origen dentro de la caja, entra "por Y" en t = 0.
fn box_span(tile: &TileHeights, origin: Vec3, dir: Vec3) -> Option<(f32, f32, usize)> {
    let min = Vec3::new(0.0, tile.bottom, 0.0);
    let max = Vec3::new(TILE_CELLS as f32, tile.top(), TILE_CELLS as f32);
    let (mut t_near, mut t_far, mut axis) = (0.0f32, f32::INFINITY, 1);
    for a in 0..3 {
        if dir[a] == 0.0 {
            // En x/z la tesela es semiabierta [0, 64): el borde 64 es de la vecina.
            let outside = origin[a] < min[a] || origin[a] > max[a];
            if outside || (a != 1 && origin[a] == max[a]) {
                return None;
            }
            continue;
        }
        let t0 = (min[a] - origin[a]) / dir[a];
        let t1 = (max[a] - origin[a]) / dir[a];
        if t0.min(t1) > t_near {
            t_near = t0.min(t1);
            axis = a;
        }
        t_far = t_far.min(t0.max(t1));
    }
    (t_near <= t_far).then_some((t_near, t_far, axis))
}

/// Índice de celda de la coordenada `p` para un rayo que avanza con `d`: justo
/// en un borde, la celda hacia la que va el rayo.
fn cell_index(p: f32, d: f32) -> i32 {
    if d < 0.0 {
        p.ceil() as i32 - 1
    } else {
        p.floor() as i32
    }
}

/// `t` en que el rayo sale del nodo (lado `size` celdas) y eje de salida; eje 1
/// si antes sale de la caja por arriba o por abajo (`t_far`).
fn node_exit(origin: Vec3, dir: Vec3, node: IVec2, size: i32, t_far: f32) -> (f32, usize) {
    let exit = |n: i32, o: f32, d: f32| {
        if d > 0.0 {
            (((n + 1) * size) as f32 - o) / d
        } else if d < 0.0 {
            ((n * size) as f32 - o) / d
        } else {
            f32::INFINITY
        }
    };
    let (tx, tz) = (exit(node.x, origin.x, dir.x), exit(node.y, origin.z, dir.z));
    let (t_exit, axis) = if tx < tz { (tx, 0) } else { (tz, 2) };
    if t_far < t_exit {
        (t_far, 1)
    } else {
        (t_exit, axis)
    }
}

/// Altura mínima del rayo en `[t0, t1]` (es una recta: está en un extremo).
fn lowest_y(origin: Vec3, dir: Vec3, t0: f32, t1: f32) -> f32 {
    (origin.y + dir.y * t0).min(origin.y + dir.y * t1)
}

/// Impacto con la columna `cell` (altura `h`), sabiendo que el rayo baja de `h`
/// dentro de ella: si ya entra por debajo pega en el lateral de entrada; si no,
/// en la cima.
fn column_hit(origin: Vec3, dir: Vec3, t: f32, axis: usize, h: f32, cell: UVec2) -> Hit {
    if origin.y + dir.y * t <= h {
        let mut normal = IVec3::ZERO;
        normal[axis] = -(dir[axis].signum() as i32);
        Hit { t, cell, normal }
    } else {
        Hit {
            t: (h - origin.y) / dir.y,
            cell,
            normal: IVec3::Y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL_X: usize = 40;
    const WALL_HEIGHT: f32 = 30.0;

    /// Relieve ondulado entre ~2 y ~17 celdas con un muro alto en x = 40.
    fn test_tile() -> TileHeights {
        let heights = (0..TILE_CELLS * TILE_CELLS)
            .map(|i| {
                let (x, z) = (i % TILE_CELLS, i / TILE_CELLS);
                if x == WALL_X {
                    WALL_HEIGHT
                } else {
                    9.0 + 6.0 * (x as f32 * 0.3).sin() * (z as f32 * 0.2).cos()
                        + ((x * z) % 7) as f32 * 0.3
                }
            })
            .collect();
        TileHeights::new(heights)
    }

    /// Oráculo independiente de `trace`: intersección rayo-caja contra CADA
    /// columna (sin caja de tesela, sin recorrido, sin helpers compartidos);
    /// devuelve el `t` del primer impacto.
    fn first_hit_by_column_boxes(tile: &TileHeights, origin: Vec3, dir: Vec3) -> Option<f32> {
        (0..TILE_CELLS * TILE_CELLS)
            .filter_map(|i| {
                let (x, z) = ((i % TILE_CELLS) as f32, (i / TILE_CELLS) as f32);
                let min = [x, tile.bottom, z];
                let max = [x + 1.0, tile.mips[0][i], z + 1.0];
                let (mut near, mut far) = (0.0f32, f32::INFINITY);
                for a in 0..3 {
                    if dir[a] == 0.0 {
                        // Celdas semiabiertas [x, x+1) en x/z, como `trace`: un
                        // rayo justo sobre una arista es de la celda superior.
                        let outside = origin[a] < min[a] || origin[a] > max[a];
                        if outside || (a != 1 && origin[a] == max[a]) {
                            return None;
                        }
                    } else {
                        let t0 = (min[a] - origin[a]) / dir[a];
                        let t1 = (max[a] - origin[a]) / dir[a];
                        near = near.max(t0.min(t1));
                        far = far.min(t0.max(t1));
                    }
                }
                (near <= far).then_some(near)
            })
            .min_by(f32::total_cmp)
    }

    /// ¿`trace` encuentra la misma primera superficie que el oráculo? Tolerancia
    /// absoluta + relativa (a 10⁴ celdas el ulp de f32 ya es ~10⁻³).
    fn agrees_with_column_boxes(tile: &TileHeights, origin: Vec3, dir: Vec3) -> bool {
        match (
            trace(tile, origin, dir),
            first_hit_by_column_boxes(tile, origin, dir),
        ) {
            (None, None) => true,
            (Some(hit), Some(t)) => (hit.t - t).abs() <= 1e-3 + 1e-5 * t,
            _ => false,
        }
    }

    /// Rayos desde un anillo alrededor de la tesela (cenitales, oblicuos y
    /// rasantes, en todas las direcciones) hacia una rejilla de puntos del suelo.
    fn sweep_rays() -> Vec<(Vec3, Vec3)> {
        let center = Vec3::new(32.0, 0.0, 32.0);
        (0..16)
            .flat_map(|i| [1.5f32, 12.3, 40.7].map(move |y| (i, y)))
            .flat_map(|(i, y)| {
                let angle = i as f32 * 0.3927 + 0.071;
                let origin = center + Vec3::new(angle.cos() * 50.0, y, angle.sin() * 50.0);
                (0..81).map(move |k| {
                    let target = Vec3::new(
                        3.37 + (k % 9) as f32 * 7.13,
                        0.0,
                        2.91 + (k / 9) as f32 * 7.07,
                    );
                    (origin, (target - origin).normalize())
                })
            })
            .collect()
    }

    #[test]
    fn ray_straight_down_hits_column_top() {
        let tile = test_tile();
        let hit = trace(&tile, Vec3::new(10.5, 100.0, 20.5), Vec3::NEG_Y);
        assert_eq!(
            hit.map(|h| (h.cell, h.normal)),
            Some((UVec2::new(10, 20), IVec3::Y))
        );
    }

    #[test]
    fn ray_above_all_terrain_misses() {
        let tile = test_tile();
        assert_eq!(
            trace(&tile, Vec3::new(-5.0, WALL_HEIGHT + 1.0, 10.5), Vec3::X),
            None
        );
    }

    #[test]
    fn low_horizontal_ray_hits_side_of_wall() {
        let tile = test_tile();
        let hit = trace(&tile, Vec3::new(-5.0, 20.0, 10.5), Vec3::X);
        assert_eq!(
            hit.map(|h| (h.cell, h.normal)),
            Some((UVec2::new(WALL_X as u32, 10), IVec3::NEG_X))
        );
    }

    #[test]
    fn ray_travelling_negative_x_hits_far_side_of_wall() {
        let tile = test_tile();
        let hit = trace(&tile, Vec3::new(70.0, 20.0, 10.5), Vec3::NEG_X);
        assert_eq!(
            hit.map(|h| (h.cell, h.normal)),
            Some((UVec2::new(WALL_X as u32, 10), IVec3::X))
        );
    }

    #[test]
    fn ray_down_the_far_tile_edge_belongs_to_the_neighbour() {
        let flat = TileHeights::new(vec![10.0; TILE_CELLS * TILE_CELLS]);
        let edge = TILE_CELLS as f32;
        assert_eq!(trace(&flat, Vec3::new(edge, 20.0, 10.5), Vec3::NEG_Y), None);
    }

    #[test]
    fn high_tile_side_reaches_down_to_lower_neighbour() {
        let mut high = TileHeights::new(vec![50.0; TILE_CELLS * TILE_CELLS]);
        high.lower_bottom_to(10.0);
        // Rayo horizontal justo sobre la superficie del vecino bajo (x < 0).
        let hit = trace(&high, Vec3::new(-5.0, 10.5, 20.5), Vec3::X);
        assert_eq!(hit.map(|h| h.normal), Some(IVec3::NEG_X));
    }

    #[test]
    fn ray_along_cell_boundary_finds_first_surface() {
        let tile = test_tile();
        let dir = Vec3::new(1.0, -0.05, 0.0).normalize();
        assert!(agrees_with_column_boxes(
            &tile,
            Vec3::new(-5.0, 14.0, 20.0),
            dir
        ));
    }

    #[test]
    fn ray_starting_inside_tile_box_finds_first_surface() {
        let tile = test_tile();
        let dir = Vec3::new(0.3, -1.0, 0.2).normalize();
        assert!(agrees_with_column_boxes(
            &tile,
            Vec3::new(10.5, 25.0, 20.5),
            dir
        ));
    }

    #[test]
    fn distant_shallow_ray_finds_first_surface() {
        let tile = test_tile();
        let origin = Vec3::new(-10_000.0, 400.0, 31.7);
        let dir = (Vec3::new(32.0, 5.0, 33.1) - origin).normalize();
        assert!(agrees_with_column_boxes(&tile, origin, dir));
    }

    #[test]
    fn grazing_diagonal_ray_resolves_within_step_cap() {
        let tile = test_tile();
        let origin = Vec3::new(-1.0, 17.5, -0.7);
        let dir = (Vec3::new(64.5, 2.0, 63.8) - origin).normalize();
        assert!(agrees_with_column_boxes(&tile, origin, dir));
    }

    #[test]
    fn skipping_nodes_finds_same_first_surface_as_every_column_box() {
        let tile = test_tile();
        let mismatches: Vec<_> = sweep_rays()
            .into_iter()
            .filter(|&(o, d)| !agrees_with_column_boxes(&tile, o, d))
            .collect();
        assert!(
            mismatches.is_empty(),
            "{} rays differ, first: {:?}",
            mismatches.len(),
            mismatches.first()
        );
    }
}

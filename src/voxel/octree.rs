//! Octree espacial para búsquedas eficientes de chunks
//! Reduce la complejidad de O(n) a O(log n)

use bevy::prelude::*;

/// Bounding box 3D para representar regiones espaciales
#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: IVec3,
    pub max: IVec3,
}

impl BoundingBox {
    /// Crea un nuevo bounding box
    pub fn new(min: IVec3, max: IVec3) -> Self {
        Self { min, max }
    }

    /// Verifica si un punto está dentro del bounding box
    pub fn contains(&self, point: IVec3) -> bool {
        point.x >= self.min.x && point.x < self.max.x &&
        point.y >= self.min.y && point.y < self.max.y &&
        point.z >= self.min.z && point.z < self.max.z
    }

    /// Verifica si dos bounding boxes se intersectan
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }

    /// Calcula el centro del bounding box
    pub fn center(&self) -> IVec3 {
        (self.min + self.max) / 2
    }

    /// Calcula el tamaño del bounding box
    pub fn size(&self) -> i32 {
        (self.max.x - self.min.x)
            .max(self.max.y - self.min.y)
            .max(self.max.z - self.min.z)
    }

    /// Verifica si una esfera intersecta con el bounding box
    pub fn intersects_sphere(&self, center: IVec3, radius: i32) -> bool {
        let closest = IVec3::new(
            center.x.clamp(self.min.x, self.max.x),
            center.y.clamp(self.min.y, self.max.y),
            center.z.clamp(self.min.z, self.max.z),
        );
        (closest - center).length_squared() <= radius * radius
    }
}

/// Nodo del Octree para búsqueda espacial eficiente
#[derive(Clone, Debug)]
pub struct OctreeNode {
    pub bounds: BoundingBox,
    pub chunks: Vec<IVec3>,
    pub children: Option<Box<[OctreeNode; 8]>>,
}

impl OctreeNode {
    /// Crea un nuevo nodo del octree
    pub fn new(bounds: BoundingBox) -> Self {
        Self {
            bounds,
            chunks: Vec::new(),
            children: None,
        }
    }

    /// Inserta un chunk en el octree
    pub fn insert(&mut self, chunk_pos: IVec3, max_depth: usize, current_depth: usize) {
        // Si llegamos a profundidad máxima o el nodo es pequeño, guardar aquí
        if current_depth >= max_depth || self.bounds.size() <= 2 {
            self.chunks.push(chunk_pos);
            return;
        }

        // Subdividir si aún no se ha hecho
        if self.children.is_none() {
            self.subdivide();
        }

        // Insertar en el octante correspondiente
        let octant = self.get_octant(chunk_pos);
        if let Some(children) = &mut self.children {
            children[octant].insert(chunk_pos, max_depth, current_depth + 1);
        }
    }

    /// Elimina un chunk del octree
    pub fn remove(&mut self, chunk_pos: IVec3) -> bool {
        // Buscar en este nodo
        if let Some(index) = self.chunks.iter().position(|&pos| pos == chunk_pos) {
            self.chunks.swap_remove(index);
            return true;
        }

        // Buscar en hijos
        if let Some(children) = &mut self.children {
            let octant = Self::get_octant_static(&self.bounds, chunk_pos);
            return children[octant].remove(chunk_pos);
        }

        false
    }

    /// Busca chunks en un radio desde una posición - O(log n)
    pub fn query_radius(&self, center: IVec3, radius: i32, results: &mut Vec<IVec3>) {
        // Early exit si el radio no intersecta este nodo
        if !self.bounds.intersects_sphere(center, radius) {
            return;
        }

        // Agregar chunks de este nodo que estén en el radio
        for &chunk_pos in &self.chunks {
            let distance_sq = (chunk_pos - center).length_squared();
            if distance_sq <= radius * radius {
                results.push(chunk_pos);
            }
        }

        // Buscar recursivamente en hijos
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.query_radius(center, radius, results);
            }
        }
    }

    /// Busca chunks en una región rectangular - O(log n)
    pub fn query_region(&self, region: BoundingBox, results: &mut Vec<IVec3>) {
        // Early exit si no hay intersección
        if !self.bounds.intersects(&region) {
            return;
        }

        // Agregar chunks de este nodo que estén en la región
        for &chunk_pos in &self.chunks {
            if region.contains(chunk_pos) {
                results.push(chunk_pos);
            }
        }

        // Buscar recursivamente en hijos
        if let Some(children) = &self.children {
            for child in children.iter() {
                child.query_region(region, results);
            }
        }
    }

    /// Busca el chunk más cercano a una posición - O(log n)
    pub fn find_nearest(&self, position: IVec3) -> Option<IVec3> {
        let mut nearest: Option<(IVec3, i32)> = None;

        self.find_nearest_recursive(position, &mut nearest);

        nearest.map(|(pos, _)| pos)
    }

    /// Búsqueda recursiva del chunk más cercano
    fn find_nearest_recursive(&self, position: IVec3, nearest: &mut Option<(IVec3, i32)>) {
        // Verificar chunks en este nodo
        for &chunk_pos in &self.chunks {
            let dist_sq = (chunk_pos - position).length_squared();
            
            match nearest {
                None => *nearest = Some((chunk_pos, dist_sq)),
                Some((_, current_dist)) if dist_sq < *current_dist => {
                    *nearest = Some((chunk_pos, dist_sq));
                }
                _ => {}
            }
        }

        // Buscar en hijos, ordenados por distancia
        if let Some(children) = &self.children {
            // Calcular distancia a cada octante
            let mut octant_distances: Vec<(usize, i32)> = children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let center = child.bounds.center();
                    let dist_sq = (center - position).length_squared();
                    (i, dist_sq)
                })
                .collect();

            // Ordenar por distancia
            octant_distances.sort_by_key(|(_, dist)| *dist);

            // Buscar en octantes cercanos primero
            for (octant_idx, _) in octant_distances {
                children[octant_idx].find_nearest_recursive(position, nearest);
            }
        }
    }

    /// Subdivide el nodo en 8 octantes
    fn subdivide(&mut self) {
        let center = self.bounds.center();
        let mut children = Vec::with_capacity(8);

        for i in 0..8 {
            let min = IVec3::new(
                if i & 1 == 0 { self.bounds.min.x } else { center.x },
                if i & 2 == 0 { self.bounds.min.y } else { center.y },
                if i & 4 == 0 { self.bounds.min.z } else { center.z },
            );

            let max = IVec3::new(
                if i & 1 == 0 { center.x } else { self.bounds.max.x },
                if i & 2 == 0 { center.y } else { self.bounds.max.y },
                if i & 4 == 0 { center.z } else { self.bounds.max.z },
            );

            children.push(OctreeNode::new(BoundingBox::new(min, max)));
        }

        self.children = Some(children.into_boxed_slice().try_into().unwrap());
    }

    /// Determina en qué octante cae una posición
    fn get_octant(&self, pos: IVec3) -> usize {
        Self::get_octant_static(&self.bounds, pos)
    }

    /// Versión estática de get_octant para evitar problemas de borrowing
    fn get_octant_static(bounds: &BoundingBox, pos: IVec3) -> usize {
        let center = bounds.center();
        let mut octant = 0;
        if pos.x >= center.x { octant |= 1; }
        if pos.y >= center.y { octant |= 2; }
        if pos.z >= center.z { octant |= 4; }
        octant
    }

    /// Cuenta el número total de chunks en el octree
    pub fn count_chunks(&self) -> usize {
        let mut count = self.chunks.len();
        
        if let Some(children) = &self.children {
            for child in children.iter() {
                count += child.count_chunks();
            }
        }
        
        count
    }

    /// Obtiene la profundidad máxima del octree
    pub fn max_depth(&self) -> usize {
        if let Some(children) = &self.children {
            1 + children.iter().map(|c| c.max_depth()).max().unwrap_or(0)
        } else {
            0
        }
    }
}

/// Recurso que mantiene el octree de chunks
#[derive(Resource)]
pub struct ChunkOctree {
    pub root: OctreeNode,
    pub chunk_count: usize,
}

impl ChunkOctree {
    /// Crea un nuevo octree con los límites especificados
    pub fn new(world_bounds: BoundingBox) -> Self {
        Self {
            root: OctreeNode::new(world_bounds),
            chunk_count: 0,
        }
    }

    /// Inserta un chunk en el octree
    pub fn insert(&mut self, chunk_pos: IVec3) {
        self.root.insert(chunk_pos, 10, 0); // max_depth = 10
        self.chunk_count += 1;
    }

    /// Elimina un chunk del octree
    pub fn remove(&mut self, chunk_pos: IVec3) -> bool {
        if self.root.remove(chunk_pos) {
            self.chunk_count -= 1;
            true
        } else {
            false
        }
    }

    /// Busca chunks en un radio
    pub fn query_radius(&self, center: IVec3, radius: i32) -> Vec<IVec3> {
        let mut results = Vec::new();
        self.root.query_radius(center, radius, &mut results);
        results
    }

    /// Busca chunks en una región
    pub fn query_region(&self, region: BoundingBox) -> Vec<IVec3> {
        let mut results = Vec::new();
        self.root.query_region(region, &mut results);
        results
    }

    /// Encuentra el chunk más cercano a una posición
    pub fn find_nearest(&self, position: IVec3) -> Option<IVec3> {
        self.root.find_nearest(position)
    }

    /// Reconstruye el octree desde cero con una lista de chunks
    pub fn rebuild(&mut self, chunks: &[IVec3]) {
        // Crear nuevo octree con los mismos límites
        let bounds = self.root.bounds;
        self.root = OctreeNode::new(bounds);
        self.chunk_count = 0;

        // Insertar todos los chunks
        for &chunk_pos in chunks {
            self.insert(chunk_pos);
        }
    }

    /// Obtiene estadísticas del octree
    pub fn stats(&self) -> OctreeStats {
        OctreeStats {
            total_chunks: self.chunk_count,
            max_depth: self.root.max_depth(),
            actual_chunks: self.root.count_chunks(),
        }
    }
}

/// Estadísticas del octree
#[derive(Debug)]
pub struct OctreeStats {
    pub total_chunks: usize,
    pub max_depth: usize,
    pub actual_chunks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_octree_insert_and_query() {
        let bounds = BoundingBox::new(IVec3::new(-100, -10, -100), IVec3::new(100, 10, 100));
        let mut octree = ChunkOctree::new(bounds);

        // Insertar algunos chunks
        octree.insert(IVec3::new(0, 0, 0));
        octree.insert(IVec3::new(5, 0, 5));
        octree.insert(IVec3::new(50, 0, 50));

        // Buscar chunks cerca del origen
        let results = octree.query_radius(IVec3::ZERO, 10);
        assert!(results.contains(&IVec3::new(0, 0, 0)));
        assert!(results.contains(&IVec3::new(5, 0, 5)));
        assert!(!results.contains(&IVec3::new(50, 0, 50)));
    }

    #[test]
    fn test_octree_remove() {
        let bounds = BoundingBox::new(IVec3::new(-100, -10, -100), IVec3::new(100, 10, 100));
        let mut octree = ChunkOctree::new(bounds);

        octree.insert(IVec3::new(0, 0, 0));
        assert_eq!(octree.chunk_count, 1);

        octree.remove(IVec3::new(0, 0, 0));
        assert_eq!(octree.chunk_count, 0);
    }

    #[test]
    fn test_find_nearest() {
        let bounds = BoundingBox::new(IVec3::new(-100, -10, -100), IVec3::new(100, 10, 100));
        let mut octree = ChunkOctree::new(bounds);

        octree.insert(IVec3::new(10, 0, 10));
        octree.insert(IVec3::new(50, 0, 50));

        let nearest = octree.find_nearest(IVec3::ZERO);
        assert_eq!(nearest, Some(IVec3::new(10, 0, 10)));
    }
}

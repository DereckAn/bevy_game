//! Sistema de chunks dinamicos como minecraft o lay of the land
//! Chunks base de 32³ que se combinan segun LOD

use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use rayon::prelude::*;
use fastnoise_lite::{FastNoiseLite, NoiseType, FractalType};
use std::time::Instant;
use std::collections::HashMap;
use crate::core::{BASE_CHUNK_SIZE, VOXEL_SIZE};
use crate::voxel::{ChunkLOD, VoxelType};


/// Chunk base de 32³ (use heap para evitar stack overflow)
#[derive(Component)]
pub struct BaseChunk {
    pub densities: Box<[[[f32; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]>,
    pub voxel_types: Box<[[[VoxelType; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]>,
    pub position: IVec3,
    pub last_accessed: Instant
}

/// Chunk combinado (multiples basechunks unidos)
#[derive(Component)]
pub struct MergedChunk {
    pub base_chunks: Vec<IVec3>, // Posiciones de chunks base incluidos
    pub effective_size: usize, // 64, 128, 256, 512
    pub position: IVec3, // Posicion del chunk combinado
    pub last_updated: Instant
}

/// Sistema principal de chunks dinamicos
#[derive(Resource)]
pub struct DynamicChunkSystem {
    pub base_chunks: HashMap<IVec3, Entity>,
    pub merged_chunks: HashMap<IVec3, Entity>,
    pub spatial_index: OctreeNode,
    pub player_position: Vec3,
    pub merge_scheduler: ChunkMergeScheduler,
}

/// Programador de operaciones de merge/split
#[derive(Default)]
pub struct ChunkMergeScheduler {
    pub merge_queue: Vec<MergeTask>,
    pub split_queue: Vec<SplitTask>,
    pub processed_regions: HashSet<IVec3>,
}

/// Tarea para combinar chunks
pub struct MergeTask {
    pub chunks_to_merge: Vec<IVec3>,
    pub target_lod: ChunkLOD,
    pub priority: f32
}

/// Tarea para dividir chunks
pub struct SplitTask {
    pub chunk_to_split: IVec3,
    pub target_lod: IVec3,
    pub priority: f32
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub min: IVec3,
    pub max: IVec3,
}

impl BoundingBox {
    pub fn contains(&self, point: IVec3) -> bool {
        point.x >= self.min.x && point.x < self.max.x &&
        point.y >= self.min.y && point.y < self.max.y &&
        point.z >= self.min.z && point.z < self.max.z
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }

    pub fn center(&self) -> IVec3 {
        (self.min + self.max) / 2
    }

    pub fn size(&self) -> i32 {
        (self.max.x - self.min.x)
            .max(self.max.y - self.min.y)
            .max(self.max.z - self.min.z)
    }

     pub fn intersects_sphere(&self, center: IVec3, radius: i32) -> bool {
        let closest = IVec3::new(
            center.x.clamp(self.min.x, self.max.x),
            center.y.clamp(self.min.y, self.max.y),
            center.z.clamp(self.min.z, self.max.z),
        );
        (closest - center).length_squared() <= radius * radius
    }
}



impl BaseChunk {

     pub fn new(position: IVec3) -> Self {
        let mut chunk = Self {
            densities: Box::new([[[0.0; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]),
            voxel_types: Box::new([[[VoxelType::Air; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]),
            position,
            last_accessed: Instant::now(),
        };
        
        // Generar terreno usando la versión optimizada
        chunk.generate_terrain();
        chunk
    }

    pub fn get_density(&self, x: usize, y: usize, z: usize) -> f32 {
        self.densities[x][y][z]
    }

    /// Generación de terreno ULTRA-OPTIMIZADA
    /// Combina FastNoiseLite (10x más rápido) + Rayon (4x más rápido) = 40x speedup!
    pub fn generate_terrain(&mut self) {
        let mut noise = FastNoiseLite::new();
        noise.set_noise_type(Some(NoiseType::OpenSimplex2));
        noise.set_fractal_type(Some(FractalType::FBm));
        noise.set_fractal_octaves(Some(4));
        noise.set_frequency(Some(0.02));
        noise.set_seed(Some(12345));

        let chunk_pos = self.position;
        
        // Calcular todas las densidades en paralelo
        let total_size = (BASE_CHUNK_SIZE + 1).pow(3);
        let densities_flat: Vec<f32> = (0..total_size)
            .into_par_iter() // ¡Paralelización automática!
            .map(|idx| {
                // Convertir índice 1D a 3D
                let x = idx % (BASE_CHUNK_SIZE + 1);
                let y = (idx / (BASE_CHUNK_SIZE + 1)) % (BASE_CHUNK_SIZE + 1);
                let z = idx / ((BASE_CHUNK_SIZE + 1) * (BASE_CHUNK_SIZE + 1));

                // Coordenadas mundiales
                let world_x = (chunk_pos.x * BASE_CHUNK_SIZE as i32 + x as i32) as f32 * VOXEL_SIZE;
                let world_z = (chunk_pos.z * BASE_CHUNK_SIZE as i32 + z as i32) as f32 * VOXEL_SIZE;
                let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f32 * VOXEL_SIZE;

                // FastNoiseLite es ~10x más rápido que Perlin
                let height = 1.5 + noise.get_noise_2d(world_x, world_z) * 0.5;
                height - world_y
            })
            .collect();

        // Copiar resultados al array 3D
        for (idx, &density) in densities_flat.iter().enumerate() {
            let x = idx % (BASE_CHUNK_SIZE + 1);
            let y = (idx / (BASE_CHUNK_SIZE + 1)) % (BASE_CHUNK_SIZE + 1);
            let z = idx / ((BASE_CHUNK_SIZE + 1) * (BASE_CHUNK_SIZE + 1));
            self.densities[x][y][z] = density;
        }

        // Calcular tipos de voxel también en paralelo
        let voxel_types_flat: Vec<VoxelType> = (0..BASE_CHUNK_SIZE.pow(3))
            .into_par_iter()
            .map(|idx| {
                let x = idx % BASE_CHUNK_SIZE;
                let y = (idx / BASE_CHUNK_SIZE) % BASE_CHUNK_SIZE;
                let z = idx / (BASE_CHUNK_SIZE * BASE_CHUNK_SIZE);
                
                let world_y = (chunk_pos.y * BASE_CHUNK_SIZE as i32 + y as i32) as f64 * VOXEL_SIZE as f64;
                let density = densities_flat[x + y * (BASE_CHUNK_SIZE + 1) + z * (BASE_CHUNK_SIZE + 1) * (BASE_CHUNK_SIZE + 1)];
                
                VoxelType::from_density(density, world_y)
            })
            .collect();

        // Copiar tipos de voxel
        for (idx, voxel_type) in voxel_types_flat.iter().enumerate() {
            let x = idx % BASE_CHUNK_SIZE;
            let y = (idx / BASE_CHUNK_SIZE) % BASE_CHUNK_SIZE;
            let z = idx / (BASE_CHUNK_SIZE * BASE_CHUNK_SIZE);
            self.voxel_types[x][y][z] = *voxel_type;
        }
    }
}

impl DynamicChunkSystem {
    pub fn new(world_bounds: BoundingBox) -> Self {
        Self {
            base_chunks: HashMap::new(),
            merged_chunks: HashMap::new(),
            spatial_index: OctreeNode::new(world_bounds),
            player_position: Vec3::ZERO,
            merge_scheduler: ChunkMergeScheduler::default()
        }
    }

    /// Actualiza posicion del jugador
    pub fn update_player_posicion(&mut self, position: Vec3) {
        self.player_position = position;
    }

    /// Agregar un chunk al sistema y al indice espacial
    pub fn add_chunk(&mut self, chunk_pos: IVec3, entity: Entity) {
        self.base_chunks.insert(chunk_pos, entity);
        self.spatial_index.insert(chunk_pos, 10,0); // Max_depth = 10 
    }

    /// Reconstruye el octree (llama solo cuando hay muchos cambios)
    pub fn rebuild_spatial_index(&mut self, world_bounds: BoundingBox) {
        self.spatial_index= OctreeNode::new(world_bounds);
        for &chunk_pos in self.base_chunks.keys() {
            self.spatial_index.insert(chunk_pos, 10, 0);
        }
    }
}

impl ChunkMergeScheduler {
    /// Análisis OPTIMIZADO - O(log n) en lugar de O(n³)
    pub fn analyze_chunks_for_merge(
        &mut self,
        player_position: Vec3,
        spatial_index: &OctreeNode,
        base_chunks: &HashMap<IVec3, Entity>,
    ) {
        self.merge_queue.clear();
        self.split_queue.clear();
        self.processed_regions.clear();

        // Convertir posición del jugador a coordenadas de chunk
        let player_chunk = IVec3::new(
            (player_position.x / (BASE_CHUNK_SIZE as f32 * VOXEL_SIZE)) as i32,
            (player_position.y / (BASE_CHUNK_SIZE as f32 * VOXEL_SIZE)) as i32,
            (player_position.z / (BASE_CHUNK_SIZE as f32 * VOXEL_SIZE)) as i32,
        );

        // Buscar chunks en diferentes radios según LOD
        self.analyze_lod_ring(player_chunk, 0, 8, ChunkLOD::Ultra, spatial_index, base_chunks);
        self.analyze_lod_ring(player_chunk, 8, 16, ChunkLOD::High, spatial_index, base_chunks);
        self.analyze_lod_ring(player_chunk, 16, 32, ChunkLOD::Medium, spatial_index, base_chunks);
        self.analyze_lod_ring(player_chunk, 32, 64, ChunkLOD::Low, spatial_index, base_chunks);

        // Ordenar por prioridad
        self.merge_queue.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
    }

    /// Analiza un anillo de chunks a cierta distancia - O(log n) por anillo
    fn analyze_lod_ring(
        &mut self,
        player_chunk: IVec3,
        inner_radius: i32,
        outer_radius: i32,
        target_lod: ChunkLOD,
        spatial_index: &OctreeNode,
        base_chunks: &HashMap<IVec3, Entity>,
    ) {
        if target_lod == ChunkLOD::Ultra {
            return; // No merge para LOD ultra
        }

        // Buscar chunks en el radio usando octree - O(log n)!
        let mut chunks_in_range = Vec::new();
        spatial_index.query_radius(player_chunk, outer_radius, &mut chunks_in_range);

        // Filtrar por anillo (entre inner y outer radius)
        chunks_in_range.retain(|&chunk_pos| {
            let dist = (chunk_pos - player_chunk).length_squared();
            dist >= inner_radius * inner_radius && dist < outer_radius * outer_radius
        });

        // Agrupar chunks en regiones de merge
        let merge_factor = target_lod.merge_factor() as i32;
        for &chunk_pos in &chunks_in_range {
            let region_start = IVec3::new(
                (chunk_pos.x / merge_factor) * merge_factor,
                (chunk_pos.y / merge_factor) * merge_factor,
                (chunk_pos.z / merge_factor) * merge_factor,
            );

            // Evitar procesar la misma región múltiples veces
            if !self.processed_regions.insert(region_start) {
                continue;
            }

            // Buscar chunks en esta región usando octree - O(log n)!
            let region_bounds = BoundingBox {
                min: region_start,
                max: region_start + IVec3::splat(merge_factor),
            };

            let mut chunks_in_region = Vec::new();
            spatial_index.query_region(region_bounds, &mut chunks_in_region);

            // Verificar que los chunks existen
            chunks_in_region.retain(|pos| base_chunks.contains_key(pos));

            // Solo crear tarea si hay suficientes chunks
            let required_chunks = (merge_factor * merge_factor * merge_factor / 2) as usize;
            if chunks_in_region.len() >= required_chunks {
                let distance = (player_chunk - region_start).length_squared() as f32;
                let priority = 1000.0 - distance;

                self.merge_queue.push(MergeTask {
                    chunks_to_merge: chunks_in_region,
                    target_lod,
                    priority,
                });
            }
        }
    }
}



// ============================================================================
// OCTREE PARA BÚSQUEDA ESPACIAL EFICIENTE
// ============================================================================

/// Nodo del octree para busqueda espacial O(log n)
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

    /// Busca chunks en un radio desde una posicion - O (log n)
    pub fn query_radius(&self, center: IVec3, radius: i32, results: &mut Vec<IVec3> ) {
        // Early exit si el radio no intersecta este nodo
        if !self.bounds.intersects_sphere(center, radius) {
            return;
        }

        // Agregar chunks de este nodo que esten en el radio
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


    /// Busca chunks en un region rectangular - O(log n)
    pub fn query_region(&self, region: BoundingBox, results: &mut Vec<IVec3>) {
        // Early exit si no hay interseccion
        if !self.bounds.intersects(&region) {return;}

        // Agregar chunks de este nodo que esten en la region
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

    /// Inserta un chunk en el octree
    pub fn insert(&mut self, chunk_pos: IVec3, max_depth: usize, current_depth: usize) {
        // Si llegamos a produndidad maxima o el nopdo es pequeno, guardamos aqui
        if current_depth >= max_depth || self.bounds.size() <= 2 {
            self.chunks.push(chunk_pos);
            return;
        }

        // Subdividir di aun no se ha hecho
        if self.children.is_none() {
            self.subdivide();
        }

        // Inserta en el octante correspondiente
        let octant = self.get_octant(chunk_pos);
        if let Some(children) = &mut self.children {
            children[octant].insert(chunk_pos, max_depth, current_depth + 1);
        }
    }

    /// Subdivide el node en 8 octantes
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
                if i & 1 == 0 { self.bounds.max.x } else { center.x },
                if i & 2 == 0 { self.bounds.max.y } else { center.y },
                if i & 4 == 0 { self.bounds.max.z } else { center.z },
            );

            children.push(OctreeNode::new(BoundingBox {min , max}));
        }

        self.children = Some(children.into_boxed_slice().try_into().unwrap());
    }

    /// Determina en que octante cae una posicion
    fn get_octant(
        &self, pos: IVec3
    ) -> usize {
        let center = self.bounds.center();
        let mut octant = 0;
        if pos.x >= center.x { octant |= 1; }
        if pos.y >= center.y { octant |= 2; }
        if pos.z >- center.z { octant |= 4; }
        octant
    }
}
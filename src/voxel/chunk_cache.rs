//! Sistema de caché persistente para chunks
//! Guarda y carga chunks del disco para evitar regeneración

use bevy::prelude::*;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write};
use crate::{
    core::BASE_CHUNK_SIZE,
    voxel::{BaseChunk, VoxelType},
};

/// Directorio donde se guardan los chunks
const CACHE_DIR: &str = "world_cache";

/// Versión del formato de caché (cambiar si el formato cambia)
const CACHE_VERSION: u32 = 1;

/// Obtiene la ruta del archivo de caché para un chunk
fn get_cache_path(chunk_pos: IVec3) -> PathBuf {
    PathBuf::from(CACHE_DIR)
        .join(format!("chunk_{}_{}_{}_{}.dat", 
            CACHE_VERSION,
            chunk_pos.x, 
            chunk_pos.y, 
            chunk_pos.z
        ))
}

/// Inicializa el directorio de caché
pub fn init_cache_dir() -> std::io::Result<()> {
    fs::create_dir_all(CACHE_DIR)?;
    info!("Cache directory initialized at: {}", CACHE_DIR);
    Ok(())
}

/// Guarda un chunk en el disco
pub fn save_chunk_to_disk(chunk: &BaseChunk) -> std::io::Result<()> {
    let path = get_cache_path(chunk.position);
    
    // Crear directorio si no existe
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let mut file = File::create(&path)?;
    
    // Escribir header
    file.write_all(&CACHE_VERSION.to_le_bytes())?;
    file.write_all(&chunk.position.x.to_le_bytes())?;
    file.write_all(&chunk.position.y.to_le_bytes())?;
    file.write_all(&chunk.position.z.to_le_bytes())?;
    
    // Escribir densidades (comprimido - solo valores únicos)
    for x in 0..=BASE_CHUNK_SIZE {
        for y in 0..=BASE_CHUNK_SIZE {
            for z in 0..=BASE_CHUNK_SIZE {
                file.write_all(&chunk.densities[x][y][z].to_le_bytes())?;
            }
        }
    }
    
    // Escribir tipos de voxel (1 byte por voxel)
    for x in 0..BASE_CHUNK_SIZE {
        for y in 0..BASE_CHUNK_SIZE {
            for z in 0..BASE_CHUNK_SIZE {
                file.write_all(&[chunk.voxel_types[x][y][z] as u8])?;
            }
        }
    }
    
    Ok(())
}

/// Carga un chunk desde el disco
pub fn load_chunk_from_disk(chunk_pos: IVec3) -> std::io::Result<BaseChunk> {
    let path = get_cache_path(chunk_pos);
    let mut file = File::open(&path)?;
    
    // Leer header
    let mut version_bytes = [0u8; 4];
    file.read_exact(&mut version_bytes)?;
    let version = u32::from_le_bytes(version_bytes);
    
    if version != CACHE_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Cache version mismatch: expected {}, got {}", CACHE_VERSION, version)
        ));
    }
    
    let mut pos_x_bytes = [0u8; 4];
    let mut pos_y_bytes = [0u8; 4];
    let mut pos_z_bytes = [0u8; 4];
    file.read_exact(&mut pos_x_bytes)?;
    file.read_exact(&mut pos_y_bytes)?;
    file.read_exact(&mut pos_z_bytes)?;
    
    let stored_pos = IVec3::new(
        i32::from_le_bytes(pos_x_bytes),
        i32::from_le_bytes(pos_y_bytes),
        i32::from_le_bytes(pos_z_bytes),
    );
    
    if stored_pos != chunk_pos {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Position mismatch: expected {:?}, got {:?}", chunk_pos, stored_pos)
        ));
    }
    
    // Crear chunk vacío
    let mut chunk = BaseChunk {
        densities: Box::new([[[0.0; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]; BASE_CHUNK_SIZE + 1]),
        voxel_types: Box::new([[[VoxelType::Air; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]; BASE_CHUNK_SIZE]),
        position: chunk_pos,
    };
    
    // Leer densidades
    for x in 0..=BASE_CHUNK_SIZE {
        for y in 0..=BASE_CHUNK_SIZE {
            for z in 0..=BASE_CHUNK_SIZE {
                let mut density_bytes = [0u8; 4];
                file.read_exact(&mut density_bytes)?;
                chunk.densities[x][y][z] = f32::from_le_bytes(density_bytes);
            }
        }
    }
    
    // Leer tipos de voxel
    for x in 0..BASE_CHUNK_SIZE {
        for y in 0..BASE_CHUNK_SIZE {
            for z in 0..BASE_CHUNK_SIZE {
                let mut voxel_type_byte = [0u8; 1];
                file.read_exact(&mut voxel_type_byte)?;
                chunk.voxel_types[x][y][z] = match voxel_type_byte[0] {
                    0 => VoxelType::Air,
                    1 => VoxelType::Stone,
                    2 => VoxelType::Dirt,
                    3 => VoxelType::Wood,
                    4 => VoxelType::Metal,
                    5 => VoxelType::Grass,
                    6 => VoxelType::Sand,
                    _ => VoxelType::Air,
                };
            }
        }
    }
    
    Ok(chunk)
}

/// Verifica si un chunk existe en caché
pub fn chunk_exists_in_cache(chunk_pos: IVec3) -> bool {
    get_cache_path(chunk_pos).exists()
}

/// Elimina un chunk del caché
pub fn delete_chunk_from_cache(chunk_pos: IVec3) -> std::io::Result<()> {
    let path = get_cache_path(chunk_pos);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Limpia todo el caché (útil para regenerar el mundo)
pub fn clear_cache() -> std::io::Result<()> {
    if Path::new(CACHE_DIR).exists() {
        fs::remove_dir_all(CACHE_DIR)?;
        fs::create_dir_all(CACHE_DIR)?;
        info!("Cache cleared");
    }
    Ok(())
}

/// Obtiene estadísticas del caché
pub fn get_cache_stats() -> std::io::Result<CacheStats> {
    let mut stats = CacheStats::default();
    
    if !Path::new(CACHE_DIR).exists() {
        return Ok(stats);
    }
    
    for entry in fs::read_dir(CACHE_DIR)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            stats.chunk_count += 1;
            stats.total_size_bytes += metadata.len();
        }
    }
    
    Ok(stats)
}

#[derive(Default, Debug)]
pub struct CacheStats {
    pub chunk_count: usize,
    pub total_size_bytes: u64,
}

impl CacheStats {
    pub fn total_size_mb(&self) -> f64 {
        self.total_size_bytes as f64 / (1024.0 * 1024.0)
    }
}

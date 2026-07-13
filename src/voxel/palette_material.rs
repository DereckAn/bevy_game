//! Material de los chunks: `StandardMaterial` (PBR) + una extensión que aplica la
//! paleta tonal por voxel en el fragment shader (ver `assets/shaders/
//! palette_extension.wgsl`). Así el color varía por voxel SIN coste de geometría:
//! el greedy meshing vuelve a fusionar por `VoxelType` y el color se calcula por
//! fragmento en la GPU.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

/// Ruta del shader de la extensión (relativa a `assets/`).
const SHADER_PATH: &str = "shaders/palette_extension.wgsl";

/// Material de los chunks reales.
pub type ChunkMaterial = ExtendedMaterial<StandardMaterial, PaletteExtension>;

/// Extensión de paleta. **Sin bindings** a propósito: solo intercambia el
/// fragment shader. Los parámetros de paleta están hardcodeados en el WGSL (ver
/// `palette_extension.wgsl`) porque el `StandardMaterial` bindless de Bevy 0.17
/// no admite añadir un uniform de extensión en el grupo 2. La fase 2 (paleta por
/// material) los moverá a un buffer/material propio.
#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
pub struct PaletteExtension {}

impl MaterialExtension for PaletteExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

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

/// Extensión de paleta. **Sin bindings**: solo intercambia el fragment shader.
/// El rango tonal por material vive en el WGSL (`SPREADS`), no en un uniform,
/// porque el `StandardMaterial` bindless de Bevy 0.17 descarta los bindings de
/// extensión en el grupo 2. El vertex alpha lleva el discriminante de `VoxelType`
/// y el shader indexa `SPREADS` con él.
#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
pub struct PaletteExtension {}

impl MaterialExtension for PaletteExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }
}

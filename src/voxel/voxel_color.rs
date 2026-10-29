//! Coloreado de voxels (vertex colors).
//!
//! El meshing escribe un color por vértice; este módulo decide ese color a
//! partir del tipo de voxel. Toda la variación (tonos de paleta y los parches
//! del pasto) la pinta el shader por fragmento (`palette_extension.wgsl`), así
//! que el color es uniforme por material y el greedy meshing puede fusionar.
//! Los colores se devuelven en RGB LINEAL (el shader PBR los interpreta así).

use crate::voxel::VoxelType;

/// Color (RGB lineal) de un vértice. El **alpha lleva el discriminante de
/// `VoxelType`** (`id/255`): el shader lo usa para leer el rango tonal de ese
/// material en `SPREADS` y aplicar la variación por fragmento (o dejar el color
/// plano si ese material no tiene paleta).
///
/// El pasto va en blanco: su verde entero (ruido + altura) lo calcula el shader
/// desde la posición mundial, y el blanco deja pasar el sombreado por cara.
pub fn voxel_color(voxel_type: VoxelType) -> [f32; 4] {
    let id = voxel_type as u8 as f32 / 255.0;
    if voxel_type == VoxelType::Grass {
        return [1.0, 1.0, 1.0, id];
    }
    let l = voxel_type.properties().color.to_linear();
    [l.red, l.green, l.blue, id]
}

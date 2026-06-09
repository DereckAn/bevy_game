//! Árboles: geometría compartida, colocación determinista y una plantilla por
//! especie. Para añadir una especie nueva: crea `<especie>.rs` con su
//! `*_template`, declárala abajo, y añádela al `match` de `place_trees` (y al
//! `TreeKind`) en `placement.rs`.

mod oak;
mod pine;
mod placement;
mod small;
mod voxelize;

// API pública del módulo (los paths `crate::vegetation::trees::*` siguen válidos).
// `tree_in_cell` / `TreeInstance` / `TreeKind` se re-exportarán cuando los use el
// sistema LOD; por ahora solo lo que consumen el terreno y el loader.
pub use placement::{place_trees, tree_ceiling_for_chunk};

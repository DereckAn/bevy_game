# Palette Color System — Implementation Plan (Option 4b: GPU Shader)

## Goal

Almost every voxel material (trees, leaves, rocks, terrain, …) will eventually
use a **tonal palette** (tints & shades of a base color) instead of one flat
color, for a richer look. This must stay **efficient**: palette variation cannot
cost geometry.

## Decision: GPU palette shader (Option 4b)

Color is decoupled from geometry. The mesh carries only *material identity* +
position; the **fragment shader** computes the palette color per pixel. Greedy
meshing merges purely by `VoxelType` again — full 70–95% triangle reduction
restored for every material — and color variation becomes free (per-fragment).

### Why not the alternatives

- **Vertex colors + no-merge (current state).** Per-voxel color forces each face
  into its own quad. Fine for thin trunks; fatal once "almost everything" does
  it — greedy meshing stops working, triangle counts explode.
- **Option 3 (color-aware CPU merge).** Merge key = `(VoxelType, colorIndex)`.
  Efficiency depends entirely on color **coherence**: random per-voxel colors →
  average run length ~1.2 → merging basically stops. Only works with
  noise-coherent *patches*, and never merges as well as 4b. Kept as a documented
  fallback, not the path.
- **Hard limit that rules out "just merge the colors".** A quad has 4 corner
  vertices and vertex colors *interpolate*; a merged quad of N different colors
  renders as a gradient, not N blocks. Distinct per-voxel colors need either one
  quad per color change (no-merge) or a shader/texture. Hence 4b.

## Target architecture (as built)

- **`ExtendedMaterial<StandardMaterial, PaletteExtension>`** ([palette_material.rs](../src/voxel/palette_material.rs)).
  Keeps PBR lighting; the extension only swaps in a fragment shader
  ([palette_extension.wgsl](../assets/shaders/palette_extension.wgsl)).
- **Base color rides the vertex color (RGB), Rust-driven.** `voxel_color` writes
  each material's flat base (from `config.rs`/`properties`), uniform per quad so
  greedy meshing still merges. The **vertex alpha carries the `VoxelType`
  discriminant** (`id/255`) — the material tag the shader needs.
- **Per-material tonal range lives in the shader**, in a `var<private> SPREADS`
  table indexed by that id: `(dark_mul, light_mul, steps)`, `steps < 1` = flat.
  The fragment derives the voxel cell from world position, hashes to a tone, and
  scales the base. `hash01`/`step_multiplier` mirror [palette.rs](../src/voxel/palette.rs).
- **Greedy meshing merges by `VoxelType`** again (wood no-merge removed); only
  grass tops stay unmerged (CPU-baked noise).

### Why the spreads are in the shader, not a uniform (bindless limitation)

The clean design was a `spreads` uniform filled from the Rust registry. It does
**not work**: Bevy 0.17's `StandardMaterial` is `#[bindless]`, and an extension's
group-2 binding is dropped from the pipeline layout (wgpu validation error
`binding 100 not available`) — even though `force_non_bindless` *should* apply.
So the per-material spreads are hardcoded in the WGSL `SPREADS` table (mirror of
`palette_of`, kept in sync by hand). Base colors stay Rust-driven; only the
*variation amounts* live in WGSL. Fully removing that split would require
replacing `ExtendedMaterial` with a from-scratch non-bindless `Material` — a big
job, deferred until the two-place sync actually hurts.

### Where to change colors
- **Base color of a material** → Rust (`config.rs` constants, or `properties`).
- **How much it varies** (`dark/light/steps`) → WGSL `SPREADS` (+ `palette.rs` mirror).

## Phased plan

Each phase is independently shippable and verifiable.

### Phase 0 — Unify palettes into a data registry
Collapse `WOOD_COLOR` / `PINE_WOOD_COLOR` / `DARK_MUL` / `LIGHT_MUL` / the
`LazyLock` palettes into one `palette_of(VoxelType) -> Palette` table. CPU path
keeps working unchanged (still bakes vertex colors from the table).
- **Verify:** trees look identical to now; `cargo build` + existing tests pass.

### Phase 1 — Prototype the shader on ONE material (wood)
Add `PaletteExt` extended material, the custom `matId` vertex attribute, and the
WGSL palette + AO. Apply to chunk meshes but only wood reads the palette;
everything else falls back to its flat color. Re-enable wood merging.
- **Verify:** trunks show per-fragment palette variation **and** trunk triangle
  count drops vs current (profiler / mesh stats). Run the app, look at a forest.

### Phase 2 — More materials + per-material spread (DONE)
Added palette entries for Stone, Leaves, PineNeedles, SmallLeaves, Bush, Dirt,
Sand (`palette_of`). Per-material `(dark, light, steps)` in the WGSL `SPREADS`
table. **Kept `ATTRIBUTE_COLOR`** (deliberately): it's cheap, merge-compatible,
and carries base+id — removing it would need a per-vertex material-id + the
bindless buffer for no real gain. **Grass stays CPU** (its color needs terrain
slope, not cheaply available per-fragment); its top-face no-merge remains the one
open optimization.

### Phase 3 — Reconcile the side systems (DONE)
- **LOD impostors** — `linear_rgba` now carries the id in alpha, so impostor
  trunks/canopies get the palette from the shader.
- **Destruction re-meshing** — no change; rebuilds via the same `voxel_color`
  path and keeps the `ChunkMaterial` handle. Stateless shader just works.
- **Drop items** — fixed the latent OOB panic: `DropAssets.materials` now sized
  to all `VoxelType` variants (`VOXEL_TYPE_COUNT`). Drops stay flat
  `StandardMaterial` (tiny cubes, no shader).
- **Foliage / Bush** — render opaque; Bush varies via its palette entry, Foliage
  stays flat. Nothing to reconcile.
- **Verify:** break a pine (trunk + needles drop — was the panic), walk to LOD
  distance, cross a chunk border — no color pops or panics.

## Open / deferred
- Grass top-face no-merge → shader (needs slope per-fragment). Biggest remaining
  triangle win.
- WGSL `SPREADS` ↔ `palette.rs` two-place sync. Removing it = custom non-bindless
  `Material` rewrite. Deferred.

## Key interactions to watch
- LOD trees have a separate color path.
- Destruction rebuilds meshes (fine; shader is stateless).
- `Foliage` / `Bush` transparency.
- Pre-existing `DropAssets.materials` stale-array bug (fix in Phase 3).

## Fallback (Option 3), if shader work is ever deferred
Store `colorIndex` in the greedy mask, merge only equal `(type, index)`, and
switch palettes from per-voxel-random to noise-coherent patches (reuse the grass
fbm). Simpler CPU pipeline; gives up true per-voxel randomness (patches instead);
never merges as well as 4b.

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

## Target architecture

- **Palette as data.** One registry: `palette_of(VoxelType) -> Palette`, where
  `Palette = { base_color, dark_mul, light_mul, steps, coherence }`. Replaces the
  scattered `config.rs` constants and the `_PALETTE` `LazyLock`s. Uploaded to the
  GPU as a storage/uniform buffer.
- **Material identity per vertex.** A custom mesh attribute (a `u32` palette/
  material id) written during meshing, instead of baking `ATTRIBUTE_COLOR`.
- **`ExtendedMaterial<StandardMaterial, PaletteExt>`.** Keep PBR lighting; the
  extension's fragment shader computes
  `base_color = palette[matId][ hash(worldPos) % steps ]`, then applies the
  face-orientation shading (current `face_shade` AO) from the normal. WGSL
  reimplements the existing `hash01`.
- **Greedy meshing simplifies.** Remove the wood/grass no-merge special cases and
  per-quad color sampling. Merge by `VoxelType` only.

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

### Phase 2 — Migrate all materials to the palette shader
Give rocks / leaves / etc. palette entries (coherent noise for large surfaces,
finer for detail). Remove `ATTRIBUTE_COLOR`; grass's noise moves into WGSL.
Delete every no-merge special case.
- **Verify:** full-scene triangle count meaningfully lower than today; visual
  parity or better; FPS up.

### Phase 3 — Reconcile the side systems
- LOD impostors (`lod_chunks.rs` uses its own `linear_rgba`).
- Destruction re-meshing (stateless shader — should just work).
- Drop-item materials (`rapier_integration.rs` — the `materials` array is already
  too short and panics for `VoxelType as usize >= 10`; fix here).
- Transparency (`Foliage` / `Bush`).
Decide per-system: use the shader or keep flat.
- **Verify:** break a trunk, walk to LOD distance, cross a chunk border — no
  color pops or panics.

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

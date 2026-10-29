# Raymarched Far Terrain — Plan

Branch: `opt/raymarch-far-terrain` (from `opt/meshing`).

Goal: render terrain out to kilometres (and later real-size mountains) by
**raycasting** the far field instead of meshing it. Near terrain stays as it is
today (greedy-meshed Real chunks + LOD heightmaps). The far field is drawn by a
shader that walks each pixel's ray through a heightfield, **skipping whole empty
regions in one step**, with cells that get **bigger with distance**.

---

## 1. Design

### 1.1 What is raymarched

The terrain is a heightfield (`TerrainGenerator::generate_height(x, z)`), so the
far field is stored as a **2D grid of columns**: each cell = one blocky column
(height + colour). A ray hitting a column hits either its flat top or one of its
4 sides, so the far field keeps the voxel look.

Caves/overhangs don't exist in the far field (they're invisible at that
distance anyway). Player edits aren't reflected (same as today's LOD).

### 1.2 Tiles and levels (bigger cubes further away)

- A **tile** = 64×64 cells, always. One entity, one proxy box, one material.
- **Level k** cell size = `0.4 m × 2^k`, tile size = `25.6 m × 2^k`
  (level 0 tile = 8×8 chunk columns, aligned to the chunk grid).
- Each level is a square **ring** of tiles around the player, ±10 tiles, minus
  the area covered by level k−1 (classic clipmap). Level k's centre snaps to the
  tile grid of level k+1, so each ring's hole is exactly a union of its tiles.
- Level 0 is a full square (no hole) of ±256 m, larger than the whole near
  field (LODs unload at 70 chunks = 224 m), so far terrain exists under every
  near column as a fallback (§1.6), and only level-0 cells (0.4 m, 8×8 per
  chunk column) ever meet near terrain.

| Level | Cell | Tile | Ring reaches |
|---|---|---|---|
| 0 | 0.4 m | 25.6 m | ~256 m |
| 2 | 1.6 m | 102 m | ~1 km |
| 4 | 6.4 m | 410 m | ~4.1 km |
| 6 | 25.6 m | 1.6 km | ~16 km |
| 7 | 51.2 m | 3.3 km | ~33 km |

~8 levels × ≤ ~320 tiles (level 0: 441) ≈ 2,700 tiles, 64² height samples each.

### 1.3 Per-tile data (GPU)

- `heights`: `R32Float` 64×64 **with a max-mip chain** (64→32→…→1). Mip m texel
  = max height of the 2^m×2^m cells below it. Built on the CPU.
- `materials`: `R8Uint` 64×64, the surface `VoxelType` of each cell. **Not**
  baked colours: `voxel_color` returns white for grass (its green comes from
  `grass_color` in the palette shader), returns linear RGB, and uses alpha for
  the material id. The far shader evaluates the palette at the hit cell instead,
  sharing the WGSL (`SPREADS`, `hash01`, `grass_color`) with
  `palette_extension.wgsl` via a common import, so near and far match.
- Uniform: tile origin, cell size, level. Shared by all tiles: the near-coverage
  mask (§1.6).

Own `Material` (not `ExtendedMaterial<StandardMaterial, _>`): Bevy 0.17's
bindless `StandardMaterial` drops extension bindings in group 2 (see
`palette_material.rs`). Pattern to follow: `CloudMaterial` in `sky/clouds.rs`.

### 1.4 The ray (empty-space skipping)

Proxy geometry = the tile's AABB (min height − margin … max height). Rasterizing
the box already skips every pixel whose ray misses the tile (sky, other tiles).
Render **back faces** (`cull_mode = Front`) and compute the entry point
analytically in the shader (`t_enter = max(0, ray∩box)`), so it also works if
the camera is ever inside a box.

Inside the box, **maximum-mipmap heightfield tracing** (Tevs et al. 2008, the
standard hierarchical DDA over a max quadtree):

```text
level = top mip; p = entry point (cell coords)
loop (capped at N iterations):
    node   = cell containing p at mip `level`
    h_max  = heights.mip(level)[node]
    y_ray  = lowest ray height while inside node
    if y_ray > h_max:            # ray is above everything in this node
        p = exit of node         # skip the whole node in ONE step
        level = min(level + 1, top)   # try a bigger step next
        if p left the tile: discard
    elif level > 0:
        level -= 1               # descend into the node
    else:
        hit: column top (normal +Y) or side (normal ±X/±Z) → done
```

That's the "skip the whole chunk because it's empty" step from the request,
applied at every scale: one 64×64-cell node, then 32×32, … down to 1 cell.

### 1.5 Output: depth, lighting, fog

- `@builtin(frag_depth)` = hit point through `view.clip_from_world`
  (reverse-Z), so far terrain occludes and is occluded correctly by meshed
  terrain, water, clouds.
- Build a `PbrInput` from (hit position, face normal, colour) and call
  `apply_pbr_lighting` + `main_pass_post_lighting_processing` → same sun,
  ambient, `DistanceFog` and atmosphere as the meshed terrain.
- `NotShadowCaster` (the proxy box would cast box shadows; shadows already end
  at 50 m). No depth prepass exists in the project today; if one is added
  (SSAO/TAA), the material needs a matching prepass shader.

### 1.6 Seam with the near field: one owner per chunk column

No cutoff radius and no depth offset. A radius can't match near coverage
(whole chunks around a snapped player chunk, loaded asynchronously, stopping at
`WORLD_CHUNK_RADIUS`), and a fixed drop can't keep flat far columns under
sloped LOD triangles (their height difference grows with slope). Instead every
chunk column has **exactly one owner**:

- **Near owns a column** when it is render-ready there: the column has a
  *visible* LOD in `ColumnLods`, or its surface Real chunk is meshed (shared
  helper `column_surface_chunk_y`, also used by `retire_covered_lods_system`),
  for **two consecutive frames** (its mesh has been uploaded). Losing
  ownership is immediate (the safe direction: far draws).
- **Far owns every other column**, including ones still loading and everything
  beyond the world edge, so there is never a hole.
- **Coverage mask:** an `Rg32Sint` texture of 144² chunk columns indexed
  toroidally (`chunk.rem_euclid(144)`), each texel storing its owning column
  (or `i32::MIN`), so no origin uniform and no aliasing. Computed every frame
  after the chunk systems, extracted with the frame (`ExtractResource`) so it
  matches that frame's near entities, written to the GPU only when it changes.
- **In the shader:** at level 0 a cell in a near-owned column is **air** (the
  ray passes through it). The max-mip ignores coverage, so skipping stays
  conservative (it may descend more than needed, never skips a surface). The
  Rust `trace` gets the same `is_air(cell)` hook, with tests.
- No overlap → no depth fight. At an ownership border the two surfaces can sit
  at different heights; the far side's column walls close the step when far is
  higher; when near is higher, the open step faces away from the camera
  (the camera is always inside the near field).
- **Cost:** writing `frag_depth` disables early-z, so far boxes behind near
  terrain still run the shader. Tiles whose 8×8 columns are all near-owned are
  set `Visibility::Hidden`, and shown again as soon as one column loses
  coverage.
- **Ready before retire:** far tiles are swapped only when the replacement is
  render-ready (textures uploaded, material prepared), not merely spawned.

**Gaps between tiles.** A tile's own `min height − margin` can sit above a
lower neighbour's surface (sharp drop across a tile or level border), leaving
a see-through slit. The box bottom must be conservative: the min over the tile
**and a one-cell border ring** sampled at the coarser neighbour's cell size,
minus one coarse cell. Phase 4 adds a test with sharp height differences
between levels.

### 1.7 Streaming

- Key: `(level, IVec2 tile)` in a `FarTiles` resource.
- When the player crosses a level's snap cell, compute the new ring; spawn
  missing tiles **before** despawning old ones (same rule as
  `chunk_lod_system.md` §4).
- Generation async on `AsyncComputeTaskPool` (one `TerrainGenerator` per task,
  `generate_height` is `&mut self`), budgeted per frame like chunks.
- Ignores `WORLD_CHUNK_RADIUS` (noise is infinite); revisit when the world
  grows.

### 1.8 Things that must change around it

- Camera `PerspectiveProjection.far` (default 1000 m → e.g. 50 km): it drives
  frustum culling.
- Cloud dome radius (800 m, chosen to sit inside the old far plane) and fog
  distances.

---

## 2. Phases

Each phase ends with a check before moving on.

0. **Baseline.** Tracy capture on this branch before any change, same route and
   duration as later captures. → verify: `.tracy` saved, FPS/frame time noted.
1. **Reference traversal in Rust + test** (`src/voxel/raymarch.rs`). Pure
   function `trace(tile, origin, dir) -> Option<Hit>` (max-mip DDA, integer
   node stepping so it always makes forward progress). Oracle: an
   **independent** ray-vs-box test against every column (shares no helpers
   with `trace`). Cases: sweep of ~3,900 rays in all directions, exact cell
   boundaries, negative directions, origin inside the tile box, a ray from
   10⁴ cells away, a grazing diagonal (step cap). The WGSL is a port of this.
   → verify: `cargo test --lib raymarch`.
2. **One static tile.** Spawn a single level-0 tile at a fixed spot with the
   material, box, shader (traversal + flat colour + `frag_depth`).
   → verify: the far surface stays within one far cell (0.4 m) vertically of
   the LOD mesh there (LOD uses sloped triangles, far uses flat columns, so
   they can't match exactly); placed overlapping meshed terrain, occlusion is
   correct both ways (no box edges, no see-through).
3. **Lighting + fog.** PBR lighting and fog in the shader.
   → verify: tile shading matches neighbouring meshed terrain at the same
   distance; day/night cycle works.
4. **Rings + streaming.** All levels, async generation, spawn-before-despawn,
   debug key to tint tiles by level.
   → verify: walk a long route — no holes, no flicker at ring boundaries;
   tile count per level stays roughly constant; Tracy zone `far_tile_generate`.
5. **Hook up at distance.** Coverage mask + `is_air` (§1.6), camera far
   plane, cloud dome, fog.
   → verify: Rust tests that `trace` treats near-owned cells as air; walking
   in and out of the near field, unloading/reloading columns and standing at
   the world edge show no holes and no far terrain poking through near
   terrain (frame-step around transitions); Tracy + GPU pass timing vs
   baseline (phase 0); view distance in km with stable FPS.
6. **Later (separate decisions):** replace the `LodChunk` band (102–205 m) with
   level 0 and raise the near-field start; taller mountains in `biomes.rs`.

---

## 3. Risks / open questions

- **Performance is unproven** until measured on the GPU (phase 5); nothing
  here guarantees it beats the meshed LOD.
- **Draw calls:** one material per tile (~1,600, ~half visible) breaks
  batching. Fine for the prototype; the upgrade is one toroidally-updated
  texture per level (8 materials total).
- **Pixel cost at grazing angles:** rays skimming flat terrain take many steps.
  Iteration cap + measure GPU time per pass (Bevy `RenderDiagnosticsPlugin`
  needs `TIMESTAMP_QUERY`).
- **`generate_height` cost** is unmeasured; 1,600 × 4,096 samples on first load.
  Measure in phase 4; coarse levels may need a cheaper noise path.
- **Trees:** today's cone impostors stop at 205 m; the far field has no trees
  (could tint/bump cells later).
- **MSAA:** `frag_depth` edges alias; acceptable at distance.

# LOD Trees — Step-by-Step Implementation Plan

Goal: show **distant trees** in the LOD terrain (out past the ~32-chunk Real band),
as cheap **impostors** (a few triangles each), placed by the *same* deterministic
`tree_in_cell` so they line up exactly with the detailed voxel trees when a chunk
converts LOD → Real (no popping/shifting).

This builds on the voxel trees in `docs/trees_implementation.md`.

---

## How LOD works here (the constraints)

- A `LodChunk` (`src/voxel/lod_chunks.rs`) is a **heightmap**, not voxels:
  `surface_heights` grid (16/8/4 per axis by `LodLevel`) + `surface_types`.
- `mesh_lod_chunk(lod_chunk)` turns it into a surface mesh (top + side skirts).
  It currently emits **positions + normals only — no vertex colors** — and the
  chunk uses a **debug-colored material** (orange→red by distance).
- Only the **y=0 column** LOD chunk exists per (x,z); absolute heights stand in
  for the whole vertical column.
- LOD reaches ~200 chunks — a **huge** area. Volume control is mandatory.
- Built/used in two places in `src/voxel/chunk_loading.rs`: the `ChunkType::Lod`
  branch of `load_chunks_system`, and `convert_real_to_lod_system`.

Because LOD is a mesh, trees are added as **impostor geometry merged into the LOD
mesh**, NOT by stamping voxels.

---

## Decisions to lock first (recommended defaults)

1. **Which trees at distance** — only **pines** (big), skip small bushes (invisible
   far away + far cheaper). → **Default: pines only.**
2. **Impostor shape** — low-poly **cone (canopy) + thin box (trunk)**, sized from the
   tree's height; vs. "crossed quads." → **Default: cone + trunk box** (fits pines).
3. **LOD bands that show trees** — Medium only / Medium+Low / all. Further = more
   trees = more cost. → **Default: Medium + Low; none on Minimal (128+).**
4. **Color upgrade** — to render green trees vs terrain, the LOD mesh needs vertex
   colors + a white material (it's debug-colored now). → **Default: yes, do it.**

---

## Step 1 — LOD vertex-color upgrade (prerequisite)

Trees need to be green against the terrain, but the LOD mesh has no colors and uses
a debug material. So first give LOD meshes vertex colors.

- [ ] In `mesh_lod_chunk`, add a `colors: Vec<[f32; 4]>` buffer and
      `mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors)`, mirroring the Real
      mesher. Push a color per vertex (terrain greens/browns — can reuse the ideas
      from `voxel_color.rs`, or a simple per-`surface_type` color to start).
- [ ] Switch LOD chunks to a **white** material so vertex colors show
      (`ChunkMaterials`: add a white `lod` handle, or reuse the real white one).
      Update `lod_handle` usage at the two call sites.

**Verify:** `cargo run --release` — distant terrain is naturally colored (green/brown)
instead of orange/red. (Bonus: distant terrain finally matches near terrain.)

> If you'd rather not recolor terrain yet, an alternative is to keep terrain on the
> debug material and give *only the tree impostors* their own white-material mesh —
> but a single colored LOD mesh is simpler and looks better.

---

## Step 2 — Thread the world `seed` into the LOD mesher

`tree_in_cell` needs the seed, but `mesh_lod_chunk` doesn't have it.

- [ ] Change `mesh_lod_chunk(lod_chunk)` → `mesh_lod_chunk(lod_chunk, seed)`
      (or store `seed` on `LodChunk` at creation — pick one; param is simplest).
- [ ] Update both call sites in `chunk_loading.rs` (`load_chunks_system` Lod branch,
      `convert_real_to_lod_system`) to pass `world_seed.0`.

**Verify:** `cargo build` (no behavior change yet).

---

## Step 3 — The pine impostor geometry

A helper that appends a cheap pine impostor (cone + trunk box) into the mesh
buffers at a world position, sized by the tree's height.

- [ ] Add a function (in `lod_chunks.rs`) like
      `add_pine_impostor(base: Vec3, height_m: f32, positions, normals, colors, indices)`:
  - **Trunk**: a thin tall box (brown), ~lower third of the height.
  - **Canopy**: a cone (green) — a ring of base vertices + an apex, fanned into
      triangles. ~6–8 sides is plenty at distance.
- [ ] Keep it tiny: a handful of triangles per tree. No per-voxel detail.

**Verify:** unit-testable in isolation (e.g. it pushes a non-zero number of indices),
or just visually in Step 4.

---

## Step 4 — Place impostors from deterministic trees

Reuse the exact placement logic so LOD and Real agree.

- [ ] In `mesh_lod_chunk`, after building the terrain surface, scan the chunk's cell
      footprint (expanded by `MAX_CANOPY_RADIUS`) exactly like `place_trees` /
      `tree_ceiling_for_chunk` — same `div_euclid` cell range.
- [ ] For each `tree_in_cell(...)` that is `Some` **and** `kind == Pine`:
  - compute its surface height via `biome.generate_height` (meters) → world Y,
  - call `add_pine_impostor` at that position, sized by `tree.height()`.
  - **Skip `TreeKind::Small`.**
- [ ] **Band limit:** only add impostors for `LodLevel::Medium` (and maybe `Low`);
      return early for `Minimal`. `log()`/comment what's dropped so it's not silent.

Note: `mesh_lod_chunk` doesn't currently have a `BiomeGenerator`. Either pass one in
(alongside `seed`) or construct a `TerrainGenerator::new(seed)` inside — match how the
call sites already create one.

**Verify:** `cargo run --release` — distant pines appear as simple green cones; as you
walk toward them and the chunk converts LOD → Real, each cone is replaced by the
detailed voxel pine **at the same spot** (no jump). Bushes do NOT appear at distance.

---

## Notes / gotchas

- **Volume is the killer.** LOD spans ~200 chunks; drawing every tree would be
  hundreds of thousands of impostors. Pines-only + band-limit (Steps 4) are not
  optional — they're what makes this viable.
- **Determinism = no pop.** Impostors use the same `tree_in_cell`, so positions match
  the Real voxel trees. The visual changes detail on conversion, not location.
- **Consistency of base height.** Use the same base as `place_trees`
  (`surface_voxel_y + 1`) so the cone sits where the real trunk will.
- **y=0 column only.** LOD trees live in the single y=0 LOD chunk per (x,z); their
  height is absolute, like the LOD terrain — no vertical chunk juggling needed.
- **Cost lives on the main thread.** LOD chunks are meshed synchronously in
  `load_chunks_system` / conversions, so heavy impostor work there causes hitches —
  keep impostors cheap and band-limited.
- **No collision** — LOD chunks have no colliders; impostors are purely visual. Real
  collision only exists once the chunk becomes Real. That's expected.

---

## Suggested order

1. **Step 1** (LOD color upgrade) — verifiable on its own; also improves distant terrain.
2. **Step 2** (thread `seed`) — tiny, unblocks the rest.
3. **Step 3** (impostor geometry) — build the cone/trunk in isolation.
4. **Step 4** (deterministic placement + band limit) — see distant pine forests.

Later: fade/scale impostors near the LOD→Real boundary to soften the swap; add an
oak impostor once the oak voxel tree exists.

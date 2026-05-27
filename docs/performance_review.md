# Performance Review — Code Analysis

**Date**: June 2026
**Scope**: Full read of the voxel pipeline (generation, meshing, chunk loading, LOD, destruction, physics, culling). Analysis only — no code was changed.

## TL;DR

The biggest wins are not in the algorithms (greedy meshing, DDA raycast, and the heightmap cache are all solid) — they are in **data duplication and redundant work**. Three findings stand out:

1. A bug that generates **5 identical stacked LOD meshes per column**.
2. A **density array that is 82% of chunk memory and fully derivable from the heightmap**.
3. **Backface culling disabled on every material**, roughly doubling GPU fragment work.

---

## Related feature — Voxel persistence (diffs) — ✅ IMPLEMENTED (June 2026)

Not a performance finding, but a **correctness** fix discovered while reviewing the chunk lifecycle (and prompted by planning for multiplayer). It is recorded here because it touches the same regeneration code as findings #1, #3, and #5.

**Problem**: Player edits (broken voxels) were stored only in the live `BaseChunk`. When a chunk converted Real→LOD or was unloaded, that data was discarded; revisiting the area regenerated pristine terrain from the seed, silently erasing all digging.

**Fix**: Store **diffs, not volumes**. A `VoxelDiffs` resource (`HashMap<IVec3, HashMap<IVec3, VoxelType>>`) records per-chunk modifications. Destruction writes to it; chunk generation (`load_chunks_system` and `convert_lod_to_real_system`) clones the relevant chunk's diffs into the async task and replays them via `BaseChunk::apply_diffs` after seed generation. Diffs are cleared in `teardown_world` (new game = clean world).

**Why diffs over a chunk cache**: a dug tunnel costs ~3 KB instead of caching a 175 KB chunk; survives Real↔LOD round-trips for free; trivially serializable to disk later (save files); and matches the multiplayer model (server authoritative state = `seed + diffs`).

**Note**: `apply_diffs` currently syncs both `voxel_types` and `densities`. Finding #3 (drop `densities`) will remove the second line.

---

## Related bug — Unload leaks ghost chunk-map entries — ✅ FIXED (June 2026)

A **correctness** bug (permanent terrain holes), found while debugging LOD round-trips. Recorded here because the LOD dedup (finding #1) exposed it.

**Problem**: `unload_chunks_system` despawned every entity but only removed it from `chunk_map`/`octree`/`spatial_hash` inside `if let Some(base_chunk) = ...` — i.e. **only for `BaseChunk`s**. LOD chunks and still-generating chunks (no `BaseChunk` component) were despawned but left a **ghost key** in `chunk_map`. On return, `load_chunks_system`'s `contains_key` check saw the ghost and skipped reloading → **permanent hole, no console message**. Triggered by traveling past the 70-chunk unload radius and back; the dedup made the surviving origin chunk a LOD one, walking straight into the bug.

**Fix**: Carry the position in the unload queue — `to_unload: Vec<(IVec3, Entity)>` instead of `Vec<Entity>`. `update_chunk_load_queue` already has `chunk_pos` when building the list (it iterates `chunk_map`), so it stores it. `unload_chunks_system` then cleans the maps **unconditionally** for every chunk type and no longer needs the `Query<Option<&BaseChunk>>` lookup.

**Lesson**: the old code tried to *reconstruct* the position from the entity at unload time, and the reconstruction only worked for one component type. Carrying the data forward removed the bug.

---

## Tier 1 — Likely the biggest FPS wins

### 1. Every LOD column is generated and rendered 5 times 🔴 — ✅ IMPLEMENTED (June 2026)

**Problem**: `update_chunk_load_queue` queues all Y levels (-1 to 3) for every XZ position (`chunk_loading.rs:163`), and the LOD path doesn't distinguish Y:

- `LodChunk::generate_surface` only uses `position.x/z` (`lod_chunks.rs:124`) — Y is ignored.
- `mesh_lod_chunk` emits absolute world heights, not heights relative to the chunk's Y.

So for each distant column the game generates the surface 5 times (binary-searching noise each time, on the main thread) and spawns 5 entities with byte-identical, co-located meshes — z-fighting plus 5× the draw calls and memory.

**Scale**: At radius 64 that's ~48,000 LOD entities where ~9,600 would do.

**Fix**: Only queue LOD chunks at a single Y level (e.g. y=0) and skip the other 4.

**Effort**: Low. **Impact**: Very high (5× less LOD work, draw calls, and memory).

### 2. `find_surface_height` binary search is mathematically unnecessary 🔴 — ✅ IMPLEMENTED (June 2026)

**Problem**: The density function is literally `generate_height(x,z) - y` (`biomes.rs:115-121`), which is monotonic in Y — so the surface height **is** `generate_height(x,z)`. The binary search in `lod_chunks.rs:150` does ~10 density evaluations (each hitting 3 noise layers) to compute something one call returns directly.

**Fix**: Replace `find_surface_height(...)` with a single `terrain_gen.biome_gen.generate_height(world_x, world_z)` call.

**Effort**: Trivial. **Impact**: ~10× cheaper LOD generation. Combined with finding #1, ~50× less LOD generation work — this alone mostly removes the known "LOD blocks the main thread" stutter, even before making LOD async.

### 3. The `densities` array is redundant and dominates memory 🔴 — ✅ IMPLEMENTED (June 2026)

**Problem**: `BaseChunk` stores 33³ f32 densities (~143 KB) + 32³ voxel types (32 KB) per chunk (`dynamic_chunks.rs:13-14`). But density is only ever used as a solid/air boolean (`<= 0.0`), and `voxel_types[x][y][z] == Air` carries exactly the same information (destruction already keeps both in sync — it sets `Air` and `-1.0` together).

**Scale**: At real-chunk radius 32 × 5 Y levels there can be ~16,000 real chunks ≈ **2.8 GB**. Dropping `densities` takes that to ~0.5 GB and significantly improves cache locality during meshing.

**Fix**:
- Remove `densities`; replace `get_density(x,y,z) <= 0.0` checks with `voxel_types[...] == VoxelType::Air`.
- Go further: a `Uniform(VoxelType) | Dense(Box<...>)` enum per chunk — most chunks are all-air or all-solid, so the typical chunk becomes 1 byte instead of 175 KB.
- Optional: flatten `voxel_types` to `Box<[VoxelType; 32768]>` with manual indexing for better locality.

**Effort**: Medium (touches meshing, generation, destruction). **Impact**: ~5.5× less memory, faster meshing.

### 4. Backface culling disabled + one material per chunk 🔴 — ✅ IMPLEMENTED (June 2026)

**Problem A**: Every chunk material is created with `cull_mode: None` (`chunk_loading.rs:310`, `:370`, `main.rs:179`). The greedy mesher already emits correct winding for both face directions (`greedy_meshing.rs:324-329`), so the GPU rasterizes and shades **both sides of every quad** — roughly 2× fragment cost across the 200 m view distance.

**Problem B**: `materials.add(...)` per chunk creates thousands of identical `StandardMaterial` assets. This defeats Bevy's render batching and bloats bind-group churn.

**Fix**:
- Remove `cull_mode: None` (default is backface culling). Verify no faces disappear — if some do, that's a winding bug worth fixing rather than masking.
- Create a handful of shared `Handle<StandardMaterial>`s once (one per debug color / voxel type) in a resource and clone handles.
- Long-term: per-vertex colors + one single material → everything batches.

**Effort**: Low. **Impact**: High (≈half the GPU fragment work, far fewer draw-call state changes).

### 5. Up to 24 full remesh + trimesh collider builds per frame on the main thread 🔴

**Problem**: `complete_chunk_generation_system` does the neighbor-aware greedy remesh **and** `Collider::from_bevy_mesh` (TriMesh = BVH construction, expensive) synchronously, up to `MAX_CHUNK_COMPLETIONS_PER_FRAME = 24` times per frame (`chunk_loading.rs:349-400`). This is almost certainly the main source of frame spikes while exploring.

**Fix options** (increasing effort):
1. Use a time budget (stop after ~3 ms) instead of a fixed count.
2. Copy the 6 neighbor border slices (6 × 32×32 bytes = 6 KB) into the async task and do the **full** mesh + collider off-thread; `Collider` is plain data and can be built in a task. The main thread then only inserts components.
3. Also: `meshes.add(mesh.clone())` clones the whole vertex buffer — build the collider first, then move the mesh into `Assets` without cloning.

**Effort**: Low (option 1) to Medium (option 2). **Impact**: High (removes frame spikes).

---

## Tier 2 — Architecture & per-frame waste

### 6. `update_chunk_transitions_system` scans the entire chunk map every frame 🟡

It iterates all loaded chunks (potentially tens of thousands) with a `sqrt` per chunk, every frame, even when the player hasn't moved (`chunk_loading.rs:479`).

**Fix**: Gate on player-chunk change (like the load queue already does) and compare squared distances against squared thresholds. The same `sqrt`-avoidance applies in several other spots.

### 7. Chunk-boundary hitch in `update_chunk_load_queue` 🟡

With 3.2 m chunks the player crosses a boundary every few seconds. Each crossing builds a ~64,000-entry `HashSet<IVec3>`, diffs it against the map, sorts, and rebuilds the keep-set (`chunk_loading.rs:153-235`).

**Fix**: Work per-XZ-column (2D set, 5× smaller) and expand Y at dispatch time; or compute the diff incrementally (moving one chunk only changes a thin ring of positions).

### 8. Skip predictably-empty chunks using the heightmap 🟡

Vertical generation (5 Y levels per column) is the documented main bottleneck. One cheap `generate_height` probe per column corner gives the column's height range **before** dispatching tasks:

- Chunk bottom above max height → all air → skip entirely (no task, no entity, no 175 KB).
- Chunk top below min height → fully buried → data only, no mesh/collider needed.

In plains terrain this eliminates roughly half of the 5 vertical levels.

### 9. `ChunkOctree` is write-only 🟡

It is inserted/removed/rebuilt everywhere, but nothing ever queries it — `ChunkMap` does the lookups and `SpatialHashGrid` does the radius queries. Its maintenance cost (and unused-code noise) buys nothing.

**Fix**: Remove it, or wire it into something real.

### 10. `setup` freezes the app on Play 🟡

It generates ~400 chunks synchronously — terrain gen + mesh + trimesh collider each (`main.rs:147-211`).

**Fix**: Smaller initial radius (the player only needs the chunks under their feet) and let the async loader fill in the rest; or a brief loading state.

### 11. Voxel drops are pricier than they look 🟡

- Each drop allocates a brand-new identical cube `Mesh` and `StandardMaterial` (`rapier_integration.rs:44-54`) → cache one handle of each.
- `Sleeping::disabled()` (`rapier_integration.rs:96`) keeps every drop active in the physics solver for its full 60-second lifetime → let them sleep.

### 12. Destruction remeshes the whole chunk per broken voxel 🟡

Including a fresh TriMesh collider, synchronously (`destruction.rs:436-449`). Fine occasionally, but fast tools / multi-voxel patterns will spike. The same "mesh off-thread" machinery from finding #5 would cover this.

**Correctness note** (not perf): multi-voxel destruction patterns are silently clipped at chunk borders.

---

## Smaller / hygiene 🟢

- **`raycast_voxel`** re-derives the chunk via `world_to_voxel` plus a HashMap+Query lookup at *every* DDA step (`destruction.rs:198-225`). Caching the current chunk while the ray stays inside it removes ~90% of lookups.
- **Greedy mesher** allocates a fresh mask `Vec` per slice — 192 allocations per chunk. Reuse one buffer (easy once meshing moves off-thread).
- **Release profile**: add `[profile.release] lto = "thin"` (optionally `codegen-units = 1`) — typically 5-15% runtime gain in math-heavy code.
- **Dependencies**: the `noise` crate appears unused (`fastnoise-lite` is what's used) — compile-time cost only. Check whether `bevy-inspector-egui` ships in release builds.
- **Shadows**: a directional light with shadows over tens of thousands of small meshes re-renders much of the scene into shadow cascades. Once draw calls drop (finding #4), consider limiting shadow distance — distant LOD terrain doesn't need to cast shadows.

---

## Suggested implementation order

| Step | Finding | Effort | Expected impact |
|---|---|---|---|
| 1 ✅ | #2 Remove LOD binary search | Trivial | ~10× cheaper LOD gen |
| 2 ✅ | #1 Dedupe LOD per column | Low | 5× less LOD everything |
| 3 ✅ | #4 Enable backface culling + shared materials | Low | ~2× less GPU fragment work |
| 4 | #6 Gate transitions on player movement | Low | Removes per-frame full-map scan |
| 5 ✅ | #3 Drop `densities` array | Medium | ~5.5× less memory |
| 6 | #5 Mesh + collider off-thread | Medium | Removes frame spikes |
| 7 | #8 Skip empty chunks via height probe | Medium | ~Half the vertical generation |
| 8 | Rest of Tier 2 / hygiene | Varies | Incremental |

Steps 1-3 are small, low-risk edits with outsized payoff. Verify FPS / frame time after each step (the debug HUD already shows both).

**Relation to `docs/optimization_roadmap.md`**: the roadmap's existing items (LOD async, LOD frustum culling) remain valid, but findings #1 and #2 shrink the LOD problem so much that they should come first.

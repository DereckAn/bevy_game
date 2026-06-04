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

### 5. Up to 24 full remesh + trimesh collider builds per frame on the main thread 🔴 — ✅ IMPLEMENTED (June 2026, options 1+3)

**Implemented**: time budget (`CHUNK_COMPLETION_BUDGET_MS = 4`) instead of relying only on the fixed count, so per-frame integration cost is bounded by wall-clock regardless of chunk cost; and removed the `mesh.clone()` by building the collider from a borrow first, then moving the mesh into `Assets`.

**Not done (option 2)**: full off-thread meshing. Deferred — the neighbor-aware mesher needs adjacent chunk voxels (no ECS access from a worker thread), which would require snapshotting neighbor borders at spawn time (worse seams) plus a re-mesh-on-neighbor-arrival system. Larger, bug-prone; revisit if frame time still spikes after the budget.

**Follow-up — nearest-first integration order (June 2026)**: a side effect of the time budget was that, at spawn, near holes lingered while the distant LOD horizon (generated synchronously, no budget) appeared instantly — terrain seemed to fill far→close. Fixed by sorting pending tasks by horizontal squared distance to the player before integrating, so the budget is spent on the *nearest ready* chunks first. Required storing `chunk_pos` on `ChunkGenerationTask` (the position is otherwise only known after polling the task). `complete_chunk_generation_system` now takes a player `Transform` query, snapshots pending tasks via `iter()`, sorts, then re-fetches each with `get_mut` to poll.

Original analysis below.


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

### 7. Chunk-boundary hitch in `update_chunk_load_queue` 🟡 — ✅ IMPLEMENTED (June 2026)

**Done**: Removed the ~64,000-entry `chunks_needed: HashSet<IVec3>` entirely. It existed to dedup, but the circle-generation triple loop visits each `(cx,cy,cz)` exactly once — no duplicates — so it was pure overhead (64k allocs/hashes + a second 64k iteration). Merged into one pass: generate each position, check `chunk_map.contains_key` inline, push only the missing ones to `to_load_vec`. The unload keep-set rebuild was left as-is (iterates the loaded set, ~thousands — comparatively minor).



With 3.2 m chunks the player crosses a boundary every few seconds. Each crossing builds a ~64,000-entry `HashSet<IVec3>`, diffs it against the map, sorts, and rebuilds the keep-set (`chunk_loading.rs:153-235`).

**Fix**: Work per-XZ-column (2D set, 5× smaller) and expand Y at dispatch time; or compute the diff incrementally (moving one chunk only changes a thin ring of positions).

### 8. Skip predictably-empty chunks using the heightmap 🟡 — ✅ IMPLEMENTED (June 2026, all-air case)

**Done**: Before generating a Real chunk, `load_chunks_system` probes the terrain via `chunk_is_above_terrain` — samples a 5×5 grid of `generate_height` over the chunk's XZ footprint, takes the max + 0.5 m margin, and if that's below the chunk's bottom Y, the chunk is entirely air. Such chunks get a zero-size `EmptyChunk` marker (kept in `ChunkMap` so they aren't re-evaluated, cleaned up by unload/teardown like any chunk) instead of a generated `BaseChunk` + mesh + collider. Skipped when the chunk has diffs (future-proofing; today an all-air chunk can't have any). In plains this removes ~3 of the 5 vertical levels.

**Not done (buried case)**: skipping mesh/collider for fully-buried solid chunks while keeping their voxel data — deferred (player can dig into them, so the data must exist; more complex).



Vertical generation (5 Y levels per column) is the documented main bottleneck. One cheap `generate_height` probe per column corner gives the column's height range **before** dispatching tasks:

- Chunk bottom above max height → all air → skip entirely (no task, no entity, no 175 KB).
- Chunk top below min height → fully buried → data only, no mesh/collider needed.

In plains terrain this eliminates roughly half of the 5 vertical levels.

### 9. `ChunkOctree` is write-only 🟡 — ✅ IMPLEMENTED (June 2026)

**Done**: Deleted `octree.rs` entirely (including `BoundingBox`, `OctreeNode`, `OctreeStats`, and its 3 tests) and removed all `octree.insert/remove/rebuild/stats` calls + `ResMut<ChunkOctree>` params from `setup`, `teardown_world`, `complete_chunk_generation_system`, `unload_chunks_system`, and `convert_real_to_lod_system`. Nothing queried it — `ChunkMap` does lookups, `SpatialHashGrid` does radius queries. Pure maintenance cost removed; warnings dropped 37→34.



It is inserted/removed/rebuilt everywhere, but nothing ever queries it — `ChunkMap` does the lookups and `SpatialHashGrid` does the radius queries. Its maintenance cost (and unused-code noise) buys nothing.

**Fix**: Remove it, or wire it into something real.

### 10. `setup` freezes the app on Play 🟡 — ✅ IMPLEMENTED (June 2026)

**Done**: Reduced `setup`'s synchronous `initial_radius` from 5 (~390 chunks) to 2 (~65 chunks). The player spawns at y=20 and free-falls, so only a small patch of ground under spawn must exist instantly; the async loader fills the rest (nearest-first, per the #5 follow-up). Safe because the player's spawn chunk (y=6) ≠ `last_player_chunk` default (0,0,0), so `update_chunk_load_queue` queues on frame 1 — async fill starts immediately, no sentinel needed. ~6× less work on the Play button.

**Note**: could shrink further or apply the #8 air-skip in `setup` too, but radius 2 already removes the perceptible freeze.



It generates ~400 chunks synchronously — terrain gen + mesh + trimesh collider each (`main.rs:147-211`).

**Fix**: Smaller initial radius (the player only needs the chunks under their feet) and let the async loader fill in the rest; or a brief loading state.

### 11. Voxel drops are pricier than they look 🟡 — ✅ IMPLEMENTED (June 2026)

**Done**: New `DropAssets` resource (`FromWorld`, registered in `PhysicsPlugin`) holds one shared cube `Handle<Mesh>` and one `Handle<StandardMaterial>` per `VoxelType` (indexed by `voxel_type as usize`). `spawn_rapier_voxel_drop` now takes `&DropAssets` and clones handles instead of calling `meshes.add`/`materials.add` per drop. Also removed `Sleeping::disabled()` so resting drops sleep (no solver cost for their 60 s lifetime). Caller `update_voxel_breaking_system` dropped its now-unused `materials` param.



- Each drop allocates a brand-new identical cube `Mesh` and `StandardMaterial` (`rapier_integration.rs:44-54`) → cache one handle of each.
- `Sleeping::disabled()` (`rapier_integration.rs:96`) keeps every drop active in the physics solver for its full 60-second lifetime → let them sleep.

### 12. Destruction remeshes the whole chunk per broken voxel 🟡 — ✅ IMPLEMENTED (June 2026, dirty-flag + budget)

**Done**: Breaking no longer remeshes inline in the input system. `update_voxel_breaking_system` now just modifies voxels, records the diff, spawns drops, and tags the chunk with a `DirtyChunk` marker (its signature lost the `ParamSet`/`meshes`/`mesh_query` — simplified to one `Query<&mut BaseChunk>`). A new `remesh_dirty_chunks_system` (registered right after it in the Update chain) regenerates mesh + TriMesh collider for dirty chunks under a 4 ms/frame budget (`DIRTY_REMESH_BUDGET_MS`), removing the marker when done. This coalesces multiple breaks on the same chunk/frame into one remesh and bounds bursts (fast tools, multi-voxel patterns) so they can't spike a frame.

**Not done**: single-dig remesh cost is unchanged (still a full-chunk greedy mesh + collider BVH, just relocated + bounded). Eliminating that needs off-thread meshing (deferred #5 option 2) or partial/region remeshing.



Including a fresh TriMesh collider, synchronously (`destruction.rs:436-449`). Fine occasionally, but fast tools / multi-voxel patterns will spike. The same "mesh off-thread" machinery from finding #5 would cover this.

**Correctness note** (not perf): multi-voxel destruction patterns are silently clipped at chunk borders.

---

## Smaller / hygiene 🟢

- **`raycast_voxel`** re-derives the chunk via `world_to_voxel` plus a HashMap+Query lookup at *every* DDA step (`destruction.rs:198-225`). Caching the current chunk while the ray stays inside it removes ~90% of lookups.
- **Greedy mesher** allocates a fresh mask `Vec` per slice — 192 allocations per chunk. Reuse one buffer (easy once meshing moves off-thread).
- **Release profile**: ✅ IMPLEMENTED (June 2026) — added `[profile.release]` with `lto = "thin"` and `codegen-units = 1`. ~5-15% runtime gain in math-heavy code; tradeoff is longer release compile time.
- **Dependencies**: ✅ removed the unused `noise` crate (June 2026) — confirmed zero references (`fastnoise-lite` is what's used); drops it and its transitive deps from the build graph for faster compiles. Still TODO: check whether `bevy-inspector-egui` ships in release builds.
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
| 6 ✅ | #5 Mesh + collider time-budget + no-clone (option 2 deferred) | Low | Removes frame spikes |
| 7 | #8 Skip empty chunks via height probe | Medium | ~Half the vertical generation |
| — ✅ | #9 Remove write-only `ChunkOctree` | Low | Less per-chunk maintenance |
| 7 ✅ | #8 Skip all-air chunks via height probe | Medium | ~Half the vertical generation in plains |
| — ✅ | #7 Drop redundant 64k HashSet in load queue | Low | Removes per-boundary hitch |
| — ✅ | #11 Cache drop mesh/material + let drops sleep | Low | Less alloc + solver cost per drop |
| — ✅ | #12 Dirty-flag + budgeted destruction remesh | Medium | Bounds dig bursts, off input path |
| — ✅ | #10 Shrink synchronous initial gen (radius 5→2) | Low | Removes Play-button freeze |
| — ✅ | Release LTO (`lto="thin"`, `codegen-units=1`) | Trivial | ~5-15% runtime |
| 8 | Rest of Tier 2 / hygiene | Varies | Incremental |

Steps 1-3 are small, low-risk edits with outsized payoff. Verify FPS / frame time after each step (the debug HUD already shows both).

**Relation to `docs/optimization_roadmap.md`**: the roadmap's existing items (LOD async, LOD frustum culling) remain valid, but findings #1 and #2 shrink the LOD problem so much that they should come first.

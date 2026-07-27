# Chunk & LOD System

How the world streams terrain around the player: full-detail **Real** chunks up
close, cheap **LOD** heightmaps in the distance, and the (gapless) transitions
between them.

Core file: [`src/voxel/chunk_loading.rs`](../src/voxel/chunk_loading.rs).
LOD mesh: [`src/voxel/lod_chunks.rs`](../src/voxel/lod_chunks.rs).

---

## 1. Two representations

| | Real chunk | LOD chunk |
|---|---|---|
| Type | `BaseChunk` (32³ voxels) | `LodChunk` (surface heightmap) |
| Geometry | greedy-meshed voxels | one sloped quad per grid cell |
| Collision | yes (Rapier collider) | none (visual only) |
| Stored in | `ChunkMap` (per `IVec3`) | `ColumnLods` (per column `IVec2`) |
| Generation | async (thread pool) | synchronous (cheap) |
| Reflects player edits | yes (`VoxelDiffs`) | no (raw noise only) |

A Real chunk exists **per (x, y, z)** — a near column is a vertical stack of them.
A LOD is **one per (x, z) column**: a heightmap with absolute Y values that
represents the whole column at once, stored **separately** from `ChunkMap` so it
never fights a Real chunk for a slot.

---

## 2. Distance thresholds (in chunks)

All distances are **horizontal** (X/Z); Y is ignored.

| Constant | Value | Meaning |
|---|---|---|
| `REAL_RADIUS` | 32 | Columns within this load Real chunks. |
| `REAL_KEEP` | 36 | Real chunks are evicted beyond this (hysteresis: `> REAL_RADIUS` so the boundary doesn't flicker). |
| `CHUNK_LOAD_RADIUS` | 64 | LODs are created out to here. |
| `CHUNK_UNLOAD_RADIUS` | 70 | LODs despawned beyond here (hysteresis vs load). |
| `LOD_DROP` | 0.3 m | LODs render this far **below** the true surface so a Real chunk on top wins the depth test (no z-fighting during the overlap). |

One chunk = `BASE_CHUNK_SIZE * VOXEL_SIZE` = 32 × 0.1 = **3.2 m**.
`WORLD_CHUNK_RADIUS` (in `core::constants`) hard-caps the finite world.

---

## 3. The systems (run order)

A `.chain()` in [`main.rs`](../src/main.rs) under `GameState::InGame`:

1. **`update_chunk_load_queue`** — on chunk-change, enqueues a Real position per Y
   within `REAL_RADIUS`, and one LOD **column** per (x,z) beyond it (deduped vs
   `ColumnLods`).
2. **`load_chunks_system`** — drains the queue (budgeted). Real → async gen task;
   LOD → build synchronously (`spawn_column_lod`) and store in `ColumnLods`.
3. **`complete_chunk_generation_system`** — integrates finished async Real chunks
   (render mesh with neighbours + collider).
4. **`unload_chunks_system`** — drains `to_unload` (budgeted), despawning entities.
5. **`retire_covered_lods_system`** — LOD→Real half of the transition (§4).
6. **`evict_real_to_lod_system`** — Real→LOD half of the transition (§4).

---

## 4. Transitions: load before unload (no holes)

The rule: **never remove one representation until the other is confirmed present.**
Because LODs are column-keyed and separate from `ChunkMap`, there is no shared slot
to fight over.

### Moving away — Real → LOD (`evict_real_to_lod_system`, on chunk-change)

1. Collect every chunk whose column is beyond `REAL_KEEP`.
2. **First** ensure each such column has a LOD (build it if missing).
3. **Then** remove the Real chunks from `ChunkMap` and queue them for despawn.

The LOD backdrop exists before the Real chunks disappear → no gap.

### Moving closer — LOD → Real (`retire_covered_lods_system`, every frame)

Real chunks are generated async by the normal load path; the LOD stays visible the
whole time. It's retired **only when the column's surface Real chunk is loaded and
meshed** — an exact coverage test:

```text
surface_y = floor(generate_height(column_center) / chunk_height_m)
covered   = ChunkMap has a completed BaseChunk at (x, surface_y, z)
```

Runs every frame because coverage completes asynchronously even while the player
stands still. (A "poll only near LODs" optimization was tried and reverted; today
it scans all LODs each frame — cheap arithmetic, with a noise sample only for the
few near ones.)

---

## 5. Two bugs that were fixed here

- **The `y=0` carrier (holes).** The old design stored the LOD in `ChunkMap` at
  `y=0` and treated that chunk as the column's carrier. Tall Ice peaks (surface
  ~60 m) plus the deep-floor optimization left `y=0` empty → no carrier → the whole
  column vanished. The column-keyed design (§1, §4) removes the dependency.
- **The unload slot-steal.** `unload_chunks_system` used to remove
  `ChunkMap[pos]` unconditionally. Since `evict` already removes the entry before
  queuing, a fast walk-back (which recreates the position with a **new** entity)
  had its slot stolen by the delayed unload of the **old** entity → orphaned chunks,
  duplicate reloads, budget starvation. Fixed: unload only clears the map slot if it
  still points to the same entity.

---

## 6. Deep-chunk floor optimization

`chunk_is_below_terrain_floor` skips generating Real chunks whose top is more than
`DIGGABLE_DEPTH_M` (~9.6 m ≈ 3 chunks) below the column's surface. Under a 60 m Ice
peak this drops ~14 buried stone chunks per column. Trade-off: dig straight down
past that depth and you hit ungenerated void (acceptable, tunable). Mirror of the
existing `chunk_is_above_terrain` air-skip.

---

## 7. The LOD mesh (continuous heightmap)

`mesh_lod_chunk` in [`lod_chunks.rs`](../src/voxel/lod_chunks.rs) builds a
**continuous sloped heightmap**: each cell is one quad whose four corners sample
the terrain height at the corner's exact world position. Neighbouring LODs sample
the *same* shared corner, so edges line up — watertight, no skirts.

> This replaced an earlier "flat tiles + one-sided skirts" mesh that looked like a
> see-through **spider web** at grazing angles (flat tops edge-on + backface-culled
> skirts). The sloped surface presents real area from every angle.

Grid resolution drops with distance (`LodLevel::grid_size`, must divide 32):

| Tier | Distance | Grid | Triangles / chunk |
|---|---|---|---|
| Medium | ≤ 64 chunks | 8×8 | 128 |
| Low | ≤ 128 | 4×4 | 32 |
| Minimal | > 128 | 2×2 | 8 |

Colours come from `voxel_color` at world coords (same noise field as Real chunks,
so distant terrain matches). Pine/oak get cone impostors; other vegetation is
invisible at distance.

---

## 8. Player edits & persistence

Breaking/placing a voxel records a diff in `VoxelDiffs`
(`chunk_pos → {local → VoxelType}`, see [`destruction.rs`](../src/voxel/destruction.rs)).
The authoritative world is **seed + diffs**: `apply_diffs` re-applies them whenever
a Real chunk is generated.

- Edits **survive** LOD↔Real cycling and unload/reload within a session.
- The **LOD does not** reflect diffs (raw noise), so an edit is invisible at
  distance and pops in when the column becomes Real.
- In-memory only; `teardown_world` clears it → edits lost on returning to the menu.

---

## 9. Efficiency notes & future work

The column-keyed transition is a **correctness** fix, not a speedup — Real/LOD
counts and per-frame cost are about the same as the old `y=0` design. Levers if you
want to go further:

- **Frustum-cull LODs.** `update_frustum_culling` only queries `BaseChunk`, so
  every LOD in range is drawn (even behind the camera). Culling them would cut LOD
  draw calls ~2–3× — the best next win and a prerequisite for a larger view distance.
- **View distance is draw-call bound.** Extending `CHUNK_LOAD_RADIUS` multiplies LOD
  count by area (radius²), not triangles. Fewer, larger LOD meshes at distance would
  be the real fix for very long draws.
- **Churn near the boundary.** Only a 4-chunk gap (`REAL_RADIUS` 32 / `REAL_KEEP`
  36), so oscillating there tears down and regenerates the (tall) Ice columns
  repeatedly. Widening `REAL_KEEP` reduces this at the cost of keeping more Real
  chunks; a delayed-unload "reprieve" would be the fuller fix.
- **`SpatialHashGrid` is query-less** — maintained but never read since the refactor
  removed its unload query. Removable for a small win.
- **`retire` scans all LODs every frame** — bounded (mostly cheap distance checks),
  but grows with view distance; the reverted `pending`-set approach targeted this.

> Note: `world_limits.md` predates this refactor (it still says "LOD radius up to
> 200 chunks" and describes a `convert_*` transition); this file supersedes it for
> the chunk/LOD lifecycle.

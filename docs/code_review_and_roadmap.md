# Code Review, Architecture Analysis & Optimization Roadmap

**Date:** February 16, 2026
**Scope:** Full review of the voxel world generation system, architecture assessment for future scalability, and phased plan to reach production quality.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Critical Bugs](#2-critical-bugs)
3. [Performance Bottlenecks](#3-performance-bottlenecks)
4. [Architecture Issues](#4-architecture-issues)
5. [Algorithm Analysis](#5-algorithm-analysis)
6. [Scalability Assessment](#6-scalability-assessment)
7. [Phase 1 — Fix the Foundation](#7-phase-1--fix-the-foundation)
8. [Phase 2 — Architecture for Scale](#8-phase-2--architecture-for-scale)
9. [Phase 3 — Feature Infrastructure](#9-phase-3--feature-infrastructure)
10. [Alternative Approaches & Advice](#10-alternative-approaches--advice)
11. [File-by-File Findings](#11-file-by-file-findings)

---

## 1. Executive Summary

The game vision — small voxels (0.1m), massive explorable worlds, abandoned civilizations with hordes of enemies, base building, multi-world portals, and eventual multiplayer raids — is ambitious and well-scoped. The technology choices (Bevy ECS, Rapier3D, greedy meshing, async chunk generation, LOD) are all correct for this kind of game.

However, the current implementation has **critical performance bugs** that make terrain generation roughly **1,000x slower than optimal**, several **logic bugs** that cause incorrect physics and visual corruption, and **architectural gaps** that will block every major planned feature (structures, enemies, building, multiplayer).

The good news: the overall direction is right. The codebase is well-organized, the module structure is clean, and the algorithms chosen are standard and proven. The fixes are surgical — most are localized to specific files and don't require rewriting the whole system.

**Current performance:** ~30-45 FPS
**Expected after Phase 1 fixes:** 90-144+ FPS (the noise allocation fix alone could yield 10-50x speedup in chunk generation)

---

## 2. Critical Bugs

### Bug #1: FastNoiseLite Allocated ~36,000 Times Per Chunk

**File:** `src/voxel/biomes.rs`, `generate_height()` (lines ~117-121)
**Severity:** CRITICAL — This is the #1 performance killer in the entire codebase.

**What happens:**
Every call to `generate_height()` creates a brand-new `FastNoiseLite` instance, configures it with biome parameters, samples it once, then throws it away. `FastNoiseLite` construction involves building a permutation table (256-512 bytes of initialization).

For a single 33×33×33 chunk:
- 35,937 calls to `generate_height()`
- Each creates 1 `FastNoiseLite` (2 for mountains)
- = **35,937 to 71,874 allocations per chunk**

At 32 chunks/frame, that's **~1.15 million to 2.3 million allocations per frame**.

**Why it happens:**
The `FastNoiseLite` is configured per-biome (different frequency, octaves, etc.), so the code creates a fresh one to apply those settings. But the biome parameters are deterministic — the same biome always uses the same noise config.

**Fix:**
Pre-create one `FastNoiseLite` instance per biome type in `BiomeGenerator::new()`. Store them in a `HashMap<BiomeType, FastNoiseLite>` or a fixed array. In `generate_height()`, look up the pre-built instance instead of constructing a new one.

```rust
// BEFORE (current — allocates every call)
pub fn generate_height(&mut self, world_x: f32, world_z: f32) -> f32 {
    let biome = self.get_biome(world_x, world_z);
    let params = biome.params();
    let mut noise = FastNoiseLite::new();  // ALLOCATION
    noise.set_noise_type(Some(NoiseType::OpenSimplex2));
    noise.set_frequency(Some(params.frequency));
    // ...
    noise.get_noise_2d(world_x, world_z)  // single use, then dropped
}

// AFTER (pre-built — zero allocations)
pub fn generate_height(&mut self, world_x: f32, world_z: f32) -> f32 {
    let biome = self.get_biome(world_x, world_z);
    let noise = &mut self.biome_noises[biome as usize];  // pre-built
    noise.get_noise_2d(world_x, world_z)
}
```

**Expected impact:** 10-50x speedup in chunk generation.

---

### Bug #2: Terrain Height Recomputed 33x Per Column

**File:** `src/voxel/dynamic_chunks.rs` + `src/voxel/biomes.rs`
**Severity:** CRITICAL — Combined with Bug #1, makes generation ~1,000x slower than optimal.

**What happens:**
`get_density(x, y, z)` computes `terrain_height(x, z) - world_y`. The terrain height depends only on X and Z, but the density function is called for every (X, Y, Z) point in the chunk. For a 33×33×33 chunk:

- 35,937 density evaluations
- Only 33×33 = 1,089 unique (X, Z) pairs
- **33 redundant height evaluations per column**

**Fix:**
Pre-compute a 2D heightmap for the chunk before the 3D pass:

```rust
// Compute heights once per XZ column
let mut height_map = vec![0.0f32; 33 * 33];
for x in 0..33 {
    for z in 0..33 {
        let wx = chunk_world_x + x as f32 * VOXEL_SIZE;
        let wz = chunk_world_z + z as f32 * VOXEL_SIZE;
        height_map[x * 33 + z] = terrain_gen.generate_height(wx, wz);
    }
}

// Then for density, just subtract Y
fn get_density(x: usize, y: usize, z: usize) -> f32 {
    height_map[x * 33 + z] - (chunk_world_y + y as f32 * VOXEL_SIZE)
}
```

**Expected impact:** 33x fewer noise evaluations. Combined with Bug #1 fix: **~1,000x total speedup**.

---

### Bug #3: Physics Collider Never Updated After Destruction

**File:** `src/voxel/destruction.rs`
**Severity:** HIGH — Breaks core gameplay (building/digging).

**What happens:**
When the player breaks a voxel:
1. The voxel is removed from the `BaseChunk` data ✅
2. The mesh is regenerated via `greedy_mesh_basechunk` ✅
3. The visual mesh is updated ✅
4. The physics collider stays the same ❌

The player (and future enemies) will collide with invisible walls where voxels were destroyed, or fall through areas that should have collision.

**Fix:**
After mesh regeneration, also regenerate the collider:

```rust
// After updating the mesh, also update the collider
if let Ok(mut collider) = colliders.get_mut(entity) {
    if let Some(new_collider) = create_terrain_collider(&new_mesh) {
        *collider = new_collider;
    }
}
```

---

### Bug #4: `Vec::remove(0)` is O(n) in Hot Path

**File:** `src/voxel/chunk_loading.rs`, `load_chunks_system()` (line ~211)
**Severity:** HIGH — Causes frame time spikes with many pending chunks.

**What happens:**
`to_load.remove(0)` shifts every element in the Vec left by one position. Called up to 32 times per frame, with potentially thousands of entries:

- 5,000 pending chunks × 32 removals = 160,000 element shifts per frame
- Each shift is a `memcpy` of the remaining elements

**Fix:**
Replace `Vec` with `VecDeque`:

```rust
// BEFORE
pub struct ChunkLoadQueue {
    pub to_load: Vec<(IVec3, f32)>,
    // ...
}
// In load_chunks_system:
let (chunk_pos, _) = load_queue.to_load.remove(0);  // O(n)

// AFTER
pub struct ChunkLoadQueue {
    pub to_load: VecDeque<(IVec3, f32)>,
    // ...
}
// In load_chunks_system:
let (chunk_pos, _) = load_queue.to_load.pop_front().unwrap();  // O(1)
```

---

### Bug #5: ChunkMap Orphaned During LOD-to-Real Conversion

**File:** `src/voxel/chunk_loading.rs`, `convert_lod_to_real_system()` (line ~479)
**Severity:** HIGH — Causes stale entity references, potential panics.

**What happens:**
When a LOD chunk is converted to a Real chunk:
1. The old LOD entity is despawned ✅
2. A new async task entity is spawned for chunk generation ✅
3. The `ChunkMap` still points to the old (now despawned) entity ❌
4. Any system doing `chunk_map.chunks.get(&chunk_pos)` will get an invalid entity

**Fix:**
Update the ChunkMap when despawning the old entity, and register the new entity when the async task completes:

```rust
// When despawning the old LOD entity:
chunk_map.chunks.remove(&chunk_pos);

// When the async generation task completes (in complete_chunk_generation_system):
chunk_map.chunks.insert(chunk_pos, new_entity);
```

---

### Bug #6: Distance Calculated From World Origin, Not Player

**File:** `src/voxel/chunk_loading.rs`, `convert_real_to_lod_system()` (line ~514-515)
**Severity:** HIGH — LOD assignments are wrong for any player not at (0,0,0).

**What happens:**
```rust
let dist_sq = chunk_pos.x * chunk_pos.x + chunk_pos.z * chunk_pos.z;
```
This computes distance from (0, 0, 0), not from the player. A chunk at (100, 0, 100) will be treated as "far" even if the player is standing right next to it.

**Fix:**
```rust
let delta = chunk_pos - player_chunk_pos;
let dist_sq = delta.x * delta.x + delta.z * delta.z;
```

---

### Bug #7: Dirt/Stone Swapped in All Downsampling Functions

**File:** `src/voxel/downsampling.rs` (lines ~55-64, repeated in `downsample_4x` and `downsample_8x`)
**Severity:** MEDIUM — All LOD chunks display wrong materials (currently disabled, but blocks LOD re-enablement).

**What happens:**
The `VoxelType` enum is: `Air=0, Dirt=1, Stone=2, Wood=3, Metal=4, Grass=5, Sand=6`.
But the downsampling code maps: `0→Air, 1→Stone, 2→Dirt, ...` — Dirt and Stone are reversed.

**Fix:**
Match the mapping to the enum's `#[repr(u8)]` values:
```rust
match most_common {
    0 => VoxelType::Air,
    1 => VoxelType::Dirt,   // was Stone
    2 => VoxelType::Stone,  // was Dirt
    3 => VoxelType::Wood,
    4 => VoxelType::Metal,
    5 => VoxelType::Grass,
    6 => VoxelType::Sand,
    _ => VoxelType::Air,
}
```

Or better, use `VoxelType::from(most_common as u8)` with a `From<u8>` impl.

---

## 3. Performance Bottlenecks

### 3.1 Sequential Density Computation (Cannot Use Rayon)

**File:** `src/voxel/dynamic_chunks.rs`

`TerrainGenerator.get_density()` takes `&mut self` because `FastNoiseLite::get_noise_2d/3d` takes `&mut self`. This prevents Rayon parallelization for the most expensive operation in chunk generation.

The comment in the code says: *"sin paralelizacion para evitar problemas con el generador"* — but the root cause is the `&mut self` requirement of the noise library.

**Solutions (pick one):**
1. **Clone the generator per thread:** Create a `TerrainGenerator` clone for each Rayon thread. Cheap if noise instances are pre-built (after fixing Bug #1).
2. **Use a thread-safe noise library:** Consider `noise` crate (already a dependency) which uses `&self` for evaluation.
3. **Wrap in `Mutex`:** Worst option — kills parallelism. Don't do this.
4. **Pre-compute heightmap (Bug #2 fix) then parallelize only the cheap Y comparison:** The expensive 2D noise is computed once per column, and the 3D density is just `height - y` which can be trivially parallelized.

**Recommended approach:** Fix Bug #1 + Bug #2 first, then the density computation becomes so cheap that parallelizing it yields diminishing returns. Focus Rayon on meshing and collider generation instead.

---

### 3.2 LOD Chunks Generated Synchronously on Main Thread

**File:** `src/voxel/chunk_loading.rs`, `load_chunks_system()` (lines ~247-281)

LOD chunk generation (surface finding via binary search) runs synchronously in the chunk loading system. At 32 chunks per frame, if most are LOD, this blocks the main thread for the entire generation time.

**Fix:** Move LOD chunk generation to async tasks, same as real chunks:

```rust
// Instead of generating LOD inline:
let lod_chunk = LodChunk::generate_surface(...);  // BLOCKS main thread

// Spawn an async task:
let task = thread_pool.spawn(async move {
    LodChunk::generate_surface(...)
});
```

---

### 3.3 All 12 Voxel Systems Chained Sequentially

**File:** `src/main.rs`

All voxel systems are `.chain()`ed, meaning they run one after another even when they have no data dependencies. Systems like frustum culling, LOD updates, and breaking could run in parallel.

**Current (sequential):**
```
update_chunk_load_queue → load_chunks → complete_generation → unload_chunks
→ transitions → lod_to_real → real_to_lod → frustum_culling → lod_update
→ start_breaking → update_breaking → debug_chunks
```

**Better (parallel groups):**
```
Group 1 (chunk lifecycle): load_queue → load → complete → unload → transitions
Group 2 (visual, parallel with Group 1): frustum_culling, lod_update
Group 3 (interaction, parallel with Group 2): start_breaking → update_breaking
```

Use Bevy's system sets with explicit ordering only where needed.

---

### 3.4 Per-Frame Iteration Over All Chunks

**Files:** `chunk_loading.rs`, `frustum_culling.rs`, `lod_system.rs`

Multiple systems iterate over every loaded chunk every frame:
- `update_chunk_transitions_system`: iterates all entries in `chunk_map.chunks`
- `update_frustum_culling`: queries all `BaseChunk` entities
- `update_chunk_lod_system`: queries all `BaseChunk` entities

With the target of thousands of chunks loaded, these O(n) per-frame passes add up.

**Fix:** Add run conditions:
```rust
// Only run when player changes chunk
.run_if(resource_changed::<PlayerChunkPos>)

// Only run when camera moves significantly
.run_if(camera_moved_threshold(5.0))
```

---

### 3.5 Greedy Meshing Allocates 192 Vecs Per Chunk

**File:** `src/voxel/greedy_meshing.rs`

For each chunk, the greedy mesher creates:
- 3 axes × 32 slices × 2 directions = 192 iterations
- Each allocates `Vec<Option<VoxelType>>` (1024 elements) and `Vec<bool>` (1024 elements)
- = 384 small Vec allocations per chunk

**Fix:** Allocate the mask buffers once and reuse:
```rust
let mut mask = vec![None; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];
let mut visited = vec![false; BASE_CHUNK_SIZE * BASE_CHUNK_SIZE];

for axis in 0..3 {
    for slice in 0..BASE_CHUNK_SIZE {
        for direction in [-1, 1] {
            mask.fill(None);          // reuse, don't reallocate
            visited.fill(false);
            // ... process slice
        }
    }
}
```

---

### 3.6 Cross-Chunk Boundary Lookups During Meshing

**File:** `src/voxel/greedy_meshing.rs`

`is_face_visible_cross_chunk` does a `HashMap::get` + ECS `Query::get` for every boundary voxel. With 32² = 1,024 boundary voxels per face × 6 faces = 6,144 lookups per chunk.

**Fix:** Before meshing, pre-fetch the 6 neighbor chunks' boundary slices into local arrays. Then the mesher can read from local memory instead of doing ECS queries per voxel.

---

## 4. Architecture Issues

### 4.1 No Region/Sector System

**Impact:** Blocks large worlds, multiplayer, disk persistence.

The current `ChunkMap` is a flat `HashMap<IVec3, Entity>`. All chunks live in memory. There's no concept of regions, sectors, or paging.

For Minecraft-scale worlds, you need:
- **Region files** (e.g., 32×32 chunk groups stored in a single file)
- **Page-in/page-out** logic (only nearby regions in memory)
- **Region-level metadata** (biome map, structure placement, explored flag)

**Recommended approach:**
```
World
├── Region (32×32 chunks, stored as one file)
│   ├── ChunkColumn (1×N×1 chunks, share biome/heightmap data)
│   │   ├── Chunk (32×32×32 voxels)
│   │   └── ...
│   └── ...
└── ...
```

---

### 4.2 No Structure Generation Pipeline

**Impact:** Blocks cities, towns, buildings, roads, abandoned civilizations.

Current terrain is purely noise-based. There's no system to place predefined or procedural structures. This is one of the most complex systems needed for your vision.

**Required components:**
1. **Structure templates** — Define buildings, houses, roads as voxel blueprints (could be hand-authored or procedural)
2. **Placement pass** — After biome selection, determine where structures go (e.g., cities in plains, cabins in forests)
3. **Multi-chunk structures** — A city spans many chunks. Need a way for chunk A to know that chunk B's structure extends into it
4. **Structure-aware generation** — Chunks must wait for structure data before generating terrain. Otherwise a house might spawn inside a hill
5. **Road/path generation** — Connect structures with paths (L-system or A* based)

**Suggested architecture:**
```
1. World seed → Biome map (2D noise, already exists)
2. Biome map → Structure placement map (Poisson disk sampling for spacing)
3. Structure map → Structure blueprints (select template by biome + randomness)
4. Per-chunk: Query structure map → If structures overlap this chunk, apply blueprint
5. Then apply normal terrain noise for non-structure areas
```

---

### 4.3 No Entity/Enemy Infrastructure

**Impact:** Blocks enemies, NPCs, animals, items on ground.

For "lots and lots of enemies," you need:
- **Surface detection:** Know where the ground is so enemies can stand on it
- **Spawn management:** Control density, variety, proximity to player
- **Spatial partitioning for AI:** Only tick AI for nearby enemies
- **Entity pooling:** Reuse entities instead of spawn/despawn (expensive with physics)
- **LOD for entities:** Full AI near player, simple behavior far away, despawn very far

**Suggested architecture:**
```
SpawnManager (Resource)
├── Per-region spawn tables (what enemies, how many, where)
├── Active entity budget (e.g., max 500 enemies at once)
├── Distance-based ticking:
│   ├── <50m: Full AI, physics, animation
│   ├── 50-200m: Simple AI, no physics
│   └── >200m: Despawned, stored as data
└── Spawn triggers (player enters region, time-based, event-based)
```

---

### 4.4 No Voxel Placement System (Building)

**Impact:** Blocks base building, construction, crafting.

Only destruction exists. For placement you need:
- **Placement raycast** — Where is the player looking? Which face of which voxel?
- **Ghost preview** — Show where the block will be placed before confirming
- **Validation** — Can't place inside the player, can't place in mid-air (or can you?), etc.
- **Inventory integration** — Deduct material from inventory
- **Chunk update** — Same as destruction: update voxel data, re-mesh, re-collider

---

### 4.5 Three Redundant Spatial Data Structures

**Files:** `destruction.rs` (ChunkMap), `spatial_hash.rs`, `octree.rs`

Three data structures track the same chunk positions:
- `ChunkMap` (HashMap) — used for all actual lookups
- `SpatialHashGrid` — used only in `update_chunk_load_queue` for radius queries
- `ChunkOctree` — **never used for any functional query**, only for stats

**Recommendation:**
- Keep `ChunkMap` — it's the primary lookup
- Replace `SpatialHashGrid` radius queries with iterating the HashMap (you already do this elsewhere) or keep it only if profiling shows the HashMap iteration is too slow
- **Remove `ChunkOctree`** — it adds overhead with zero functional benefit

---

### 4.6 Hardcoded Configuration

Constants like seed (`12345`), load radius, voxel size, and FOV are scattered across multiple files as hardcoded values. Some are in `constants.rs`, others are inline magic numbers.

**Recommendation:** Centralize all world-generation parameters into `GameSettings` or a dedicated `WorldConfig` resource:
```rust
pub struct WorldConfig {
    pub seed: u64,
    pub chunk_size: usize,
    pub voxel_size: f32,
    pub load_radius: i32,
    pub unload_radius: i32,
    pub max_chunks_per_frame: usize,
    pub vertical_range: (i32, i32),  // (min_y, max_y)
}
```

---

## 5. Algorithm Analysis

### 5.1 Terrain Generation — Current Algorithm

```
For each chunk (32×32×32):
  1. Allocate 33×33×33 density array (heap, ~144 KB)
  2. Allocate 33×33×33 coordinate Vec (heap, ~430 KB)
  3. For each (x, y, z) in 33×33×33:          ← 35,937 iterations
     a. Compute world coordinates
     b. Call get_density(wx, wy, wz)
        → Call generate_height(wx, wz)
           → Allocate new FastNoiseLite        ← BUG #1
           → Configure with biome params
           → Sample noise once
           → Deallocate FastNoiseLite
        → Return height - wy
  4. Copy densities into 3D array
  5. For each (x, y, z) in 32×32×32 (parallel via Rayon):
     a. Read density
     b. Call VoxelType::from_density(density, height)
  6. Copy voxel types into 3D array
```

**Time complexity:** O(33³ × C_noise) where C_noise includes allocation
**Space complexity:** O(33³ × 4 + 32³ × 1) ≈ 176 KB per chunk, plus ~574 KB temporary
**Actual bottleneck:** The noise allocation dominates everything else

### 5.2 Terrain Generation — Optimal Algorithm

```
For each chunk (32×32×32):
  1. Allocate 33×33 heightmap (on stack or small heap, ~4.4 KB)
  2. For each (x, z) in 33×33:               ← 1,089 iterations (vs 35,937)
     a. Compute world (wx, wz)
     b. Call generate_height(wx, wz)
        → Look up pre-built noise for biome  ← NO allocation
        → Sample noise
     c. Store height in heightmap
  3. For each (x, y, z) in 32×32×32 (parallel via Rayon):
     a. density = heightmap[x][z] - world_y  ← subtraction, not noise
     b. voxel_type = from_density(density, world_y)
     c. Write directly to chunk array         ← no intermediate copy
```

**Time complexity:** O(33² × C_noise + 32³)
**Space complexity:** O(33² × 4 + 32³ × 1) ≈ 37 KB per chunk
**Expected speedup:** ~1,000x for the noise phase

### 5.3 Greedy Meshing — Current Algorithm

The greedy meshing implementation is a standard and correct algorithm:

```
For each axis (X, Y, Z):
  For each slice (0..31):
    For each direction (+1, -1):
      1. Build mask: for each (u, v) in slice, check if face is visible
         (adjacent voxel is Air or chunk boundary)
      2. Greedy merge: scan mask row by row
         a. Find unvisited non-Air cell
         b. Expand width: extend right while same voxel type
         c. Expand height: extend down while entire row matches
         d. Emit quad for the merged rectangle
         e. Mark merged cells as visited
```

**Time complexity:** O(3 × 32 × 2 × 32²) = O(196,608) per chunk — this is efficient
**Output:** Typically 70-95% fewer triangles than naive per-voxel meshing
**Correctness:** The algorithm is correct. It does not find the globally minimal quad count (that's NP-hard), but the greedy approach is the industry standard.

**Improvement opportunity:** The cross-chunk boundary checks do 6,144 HashMap + ECS lookups per chunk. Pre-fetching neighbor boundary slices would eliminate these.

### 5.4 Chunk Loading — Current Algorithm

```
On player chunk change:
  1. Iterate all positions in circle(radius=64) × Y[-1..3]  ← ~64,340 positions
  2. For each position, check ChunkMap (HashMap lookup)
  3. Collect unloaded positions into to_load Vec
  4. Sort by distance to player
  5. Collect far positions into to_unload

Per frame:
  6. Pop up to 32 entries from to_load (Vec::remove(0))     ← O(n) bug
  7. For Real chunks: spawn async task
  8. For LOD chunks: generate synchronously on main thread   ← blocks main thread
  9. Poll completed async tasks
  10. For completed: mesh, collide, spawn entity
```

**Scaling concern:** Step 1 iterates ~64,340 positions on every player chunk change. As load radius increases, this becomes O(πr²×h). For radius 100, that's ~157,000 positions.

**Improvement:** Instead of iterating the entire radius, maintain a sorted priority queue. When the player moves to a new chunk, only add the newly-in-range positions and remove the newly-out-of-range positions (differential update).

### 5.5 Spatial Hash — Analysis

```
Grid: HashMap<(i32, i32), Vec<IVec3>>
Cell size: 16 chunks

Insert: O(n) due to contains() duplicate check
Remove: O(n) due to position() linear search
Query:  O(cells_in_radius × chunks_per_cell)
```

The `insert` duplicate check is O(n) within the cell. With up to ~256 chunks per cell (16×16), this is 256 comparisons per insert. Consider using `HashSet<IVec3>` per cell instead of `Vec<IVec3>`.

### 5.6 Octree — Analysis

The octree is initialized with bounds [-200, -10, -200] to [200, 10, 200] but chunk positions can exceed these bounds (radius 64 × 3.2m = 204.8m). Chunks outside the bounds still get inserted but with poor spatial distribution.

More importantly: **the octree is never used for any functional query**. It's only used in a startup log to print stats. It should be removed to eliminate the maintenance overhead.

---

## 6. Scalability Assessment

### 6.1 Memory Scalability

| Load Radius | Total Chunks (5 Y layers) | BaseChunk Memory | Feasibility |
|-------------|--------------------------|------------------|-------------|
| 16 (current effective) | ~4,020 | ~690 MB | Borderline |
| 32 | ~16,080 | ~2.7 GB | Requires LOD |
| 64 (configured) | ~64,340 | ~10.8 GB | Impossible without streaming |
| 128 | ~257,360 | ~43.3 GB | Needs region system |

**Note:** These assume all chunks are full BaseChunks. With LOD (which stores only heightmaps), far chunks use much less memory. The key is ensuring LOD actually works (currently disabled due to bugs).

With LOD working:
- Real chunks (radius 16): ~4,020 × 176 KB = ~690 MB
- LOD chunks (radius 16-64): heightmap only, ~1-4 KB each ≈ ~240 MB
- **Total feasible:** ~1 GB for radius 64 with LOD

### 6.2 CPU Scalability

| Operation | Current Cost | After Phase 1 | Notes |
|-----------|-------------|----------------|-------|
| Chunk generation (noise) | ~33ms per chunk | ~0.03ms per chunk | 1,000x improvement |
| Greedy meshing | ~2ms per chunk | ~1.5ms per chunk | Buffer reuse |
| Collider generation | ~1ms per chunk | ~1ms per chunk | Unchanged |
| Per-frame iteration | O(all chunks) | O(changed chunks) | Run conditions |

### 6.3 Disk Scalability (Future)

For world persistence and large worlds:

| Approach | File Size Per Region | Access Pattern |
|----------|---------------------|----------------|
| Raw binary (current) | 172 KB per chunk | Too many files |
| Region files (32×32) | ~5.5 MB per region file | Minecraft-style |
| Compressed (LZ4) | ~50 KB per region | Good for SSD |
| SQLite per world | Variable | Good for metadata queries |

**Recommendation:** Region files with LZ4 compression. Voxel data compresses extremely well because chunks have large runs of the same voxel type.

---

## 7. Phase 1 — Fix the Foundation

**Goal:** Fix all critical bugs. Expected result: 90-144+ FPS, correct physics, correct visuals.
**Estimated scope:** 7 files modified, ~200 lines changed.

### Step 1.1: Fix FastNoiseLite allocation in biomes.rs
- Pre-create one `FastNoiseLite` per biome type in `BiomeGenerator::new()`
- Store in array indexed by `BiomeType as usize`
- Look up in `generate_height()` instead of constructing

### Step 1.2: Cache terrain height per XZ column in dynamic_chunks.rs
- Compute 33×33 heightmap before the 3D pass
- In density computation, use `heightmap[x][z] - world_y`
- Eliminate 33x redundant noise evaluations

### Step 1.3: Make noise compatible with Rayon (biomes.rs / dynamic_chunks.rs)
- After Steps 1.1 and 1.2, the density computation is just arithmetic (height - y)
- The expensive noise is in the 2D heightmap pass (1,089 evaluations)
- Option A: Clone `BiomeGenerator` per Rayon thread for the heightmap pass
- Option B: Use `noise` crate (already a dependency, uses `&self`) instead of `fastnoise-lite`
- The voxel type computation (already parallel) remains unchanged

### Step 1.4: Fix collider update after destruction (destruction.rs)
- After re-meshing, regenerate the `Collider` component
- Use the same `create_terrain_collider()` function used at spawn time

### Step 1.5: Fix ChunkMap orphaning (chunk_loading.rs)
- In `convert_lod_to_real_system`: remove old entry from ChunkMap before despawning
- In `complete_chunk_generation_system`: insert new entity into ChunkMap

### Step 1.6: Fix distance calculation (chunk_loading.rs)
- In `convert_real_to_lod_system`: compute distance from player chunk position, not (0,0,0)

### Step 1.7: Replace Vec::remove(0) with VecDeque (chunk_loading.rs)
- Change `to_load: Vec<...>` to `to_load: VecDeque<...>`
- Replace `.remove(0)` with `.pop_front()`
- Replace `.sort_by()` with sort-then-convert or use `BinaryHeap` for a priority queue

### Step 1.8: Fix Dirt/Stone swap in downsampling (downsampling.rs)
- Correct the match arms to align with `VoxelType`'s `#[repr(u8)]` values
- Apply fix in all three functions (`downsample_2x`, `downsample_4x`, `downsample_8x`)

---

## 8. Phase 2 — Architecture for Scale

**Goal:** Enable large worlds, persistence, and the hooks needed for future features.
**Estimated scope:** New modules, significant refactoring.

### Step 2.1: Region System
- Group chunks into regions (32×32×N chunks per region)
- Region file format: header + chunk offset table + compressed chunk data
- Region manager: load/unload entire regions, not individual chunks
- Memory budget: keep only nearby regions in RAM

### Step 2.2: Buffered Chunk Serialization
- Replace the disabled `chunk_cache.rs` with proper buffered I/O
- Use `BufWriter`/`BufReader` for all disk operations
- Add LZ4 compression (voxel data compresses 4-10x)
- Batch writes (don't write every chunk change immediately)

### Step 2.3: System Scheduling Optimization
- Remove `.chain()` from independent systems
- Add system sets: `ChunkLifecycle`, `ChunkVisual`, `PlayerInteraction`
- Add run conditions: only run when relevant state changes
- Profile with Bevy's built-in system timing

### Step 2.4: Remove Dead Code
- Remove `ChunkOctree` (unused for queries)
- Remove or fix `SpatialHashGrid` (evaluate if HashMap iteration suffices)
- Clean up disabled systems (chunk_cache, frustum_culling) — fix or remove
- Address compiler warnings from disabled code

### Step 2.5: Centralize Configuration
- Move all hardcoded values (seed, radii, sizes) to `WorldConfig` resource
- Make `GameSettings` comprehensive
- Support config file loading (for modding later)

### Step 2.6: Async LOD Generation
- Move LOD chunk generation to async tasks (same pattern as real chunks)
- Fix the downsampling bugs (Step 1.8) and re-enable LOD
- Re-enable frustum culling after fixing the FOV mismatch

---

## 9. Phase 3 — Feature Infrastructure

**Goal:** Build the systems needed for your game vision.

### Step 3.1: Structure Generation Pipeline

**Architecture for abandoned cities and buildings:**

```
WorldGenerator
├── BiomePass (existing, 2D noise → biome type)
├── StructurePlacementPass (NEW)
│   ├── Input: biome map, world seed
│   ├── Algorithm: Poisson disk sampling within biome-appropriate areas
│   ├── Output: StructureMap { position, type, rotation, size }
│   └── Types: City, Town, Village, Cabin, Road, Bridge, Ruin
├── StructureBlueprintPass (NEW)
│   ├── Input: StructureMap entries
│   ├── Select template for each structure (handmade or procedural)
│   ├── For cities: generate road grid, then fill plots with buildings
│   └── Output: Per-chunk structure overlay data
└── ChunkGenerationPass (modified existing)
    ├── Generate terrain noise (existing)
    ├── Apply structure overlay (carve/place voxels from blueprint)
    └── Output: final BaseChunk
```

**Structure templates can be:**
- Handmade `.vox` files (MagicaVoxel format)
- Procedurally generated (L-systems for buildings, wave function collapse for interiors)
- Hybrid (hand-authored walls/roofs, procedural interior/furniture)

**Multi-chunk structures:**
A structure that spans multiple chunks needs a "structure registry" resource. When generating chunk A, query the registry for any structures overlapping A's bounds. The registry is populated during the StructurePlacementPass, which runs once per region on first load.

### Step 3.2: Surface Detection & Spawn System

```
SpawnManager (Resource)
├── Surface cache: HashMap<IVec2, f32> (ground height per XZ position)
│   ├── Built from chunk heightmaps during generation
│   └── Updated when terrain is modified (destruction/placement)
├── Spawn zones: Vec<SpawnZone>
│   ├── Per-region, based on biome and structure proximity
│   ├── Higher density near cities (your "lots of enemies" requirement)
│   └── Lower density in open terrain
├── Active entity budget: max 500-1000 enemies simultaneously
├── LOD tiers for entities:
│   ├── Tier 1 (<50m): Full AI, physics, animation, pathfinding
│   ├── Tier 2 (50-150m): Simplified AI, no physics, reduced updates
│   ├── Tier 3 (150-300m): Stationary, visual only, no AI ticks
│   └── Tier 4 (>300m): Despawned, saved as data for re-spawn
└── Spawn triggers:
    ├── Player enters new region → populate with enemies
    ├── Time-based respawn (enemies return after N minutes)
    └── Event-based (raid triggers, quest triggers)
```

### Step 3.3: Voxel Placement System (Building)

```
PlacementSystem
├── Raycast from camera → target face + normal
├── Ghost preview entity (semi-transparent block at target position)
├── Validation:
│   ├── Not overlapping player or other entities
│   ├── Adjacent to existing solid voxel (optional — allow floating?)
│   └── Player has material in inventory
├── On confirm (right-click):
│   ├── Deduct from inventory
│   ├── Set voxel type in chunk
│   ├── Re-mesh chunk (greedy meshing)
│   ├── Re-collider chunk
│   └── Mark chunk dirty (for persistence)
└── Multi-block placement (optional):
    ├── Hold and drag for walls
    ├── Blueprint mode for predefined shapes
    └── Undo support (track recent placements)
```

### Step 3.4: Multi-World System

```
WorldManager (Resource)
├── worlds: HashMap<WorldId, WorldConfig>
│   ├── Each world has: seed, biome parameters, structure density, enemy types
│   ├── Different worlds can have different biome distributions
│   └── Example: "Nether"-like world with lava biomes, "Forest" world with dense trees
├── active_world: WorldId
├── portals: Vec<Portal>
│   ├── Position in current world
│   ├── Target world + position
│   ├── Activation: player stands on portal for N seconds
│   └── Visual: particle effect, glowing voxels
└── World transitions:
    ├── Save current world state to disk
    ├── Load target world (from disk or generate fresh)
    ├── Teleport player to target position
    └── Unload old world chunks (memory management)
```

### Step 3.5: Base/Safe Zone System

```
BaseSystem
├── Claim area: player places a "base core" block
├── Protected radius: enemies can't spawn within N meters
├── Storage: chests, workbenches, etc. (future crafting)
├── Persistence: base data saved per-world
├── Raiding (future multiplayer):
│   ├── Other players can attack bases
│   ├── Enemies can raid bases during events
│   ├── Defenses: walls, traps, turrets
│   └── Base health: if core is destroyed, protection drops
└── Safe zones (per-world exits):
    ├── Fixed locations in each world
    ├── Portal to hub/lobby
    ├── Player inventory persists across worlds
    └── Base core cannot be placed near exits
```

---

## 10. Alternative Approaches & Advice

### 10.1 Noise Library Choice

**Current:** `fastnoise-lite` — takes `&mut self`, blocks parallelism.
**Alternative 1:** `noise` crate (already in Cargo.toml) — uses `&self`, Rayon-friendly. Slightly different noise quality but functionally equivalent.
**Alternative 2:** `simdnoise` — SIMD-accelerated, 2-5x faster than scalar noise. Requires `unsafe` but well-tested.
**Recommendation:** Switch to the `noise` crate for terrain generation. It's already a dependency and solves the `&mut self` problem.

### 10.2 Chunk Size Tradeoffs

| Size | Voxels | World Size | Mesh Time | Memory | Notes |
|------|--------|------------|-----------|--------|-------|
| 16³ | 4,096 | 1.6m | ~0.5ms | ~4 KB | More chunks, more overhead |
| **32³** | **32,768** | **3.2m** | **~2ms** | **~32 KB** | **Current — good balance** |
| 64³ | 262,144 | 6.4m | ~15ms | ~256 KB | Fewer chunks, slower updates |

**32³ is the right choice** for 0.1m voxels. Smaller chunks would mean too many entities; larger chunks would make destruction/meshing too slow.

### 10.3 Meshing Alternatives

| Algorithm | Triangle Reduction | Complexity | UV Support | Notes |
|-----------|-------------------|------------|------------|-------|
| **Greedy meshing** | **70-95%** | **O(n²)** per slice | **Hard** | **Current — good choice** |
| Naive (per-voxel) | 0% | O(n³) | Easy | Way too many triangles |
| Marching cubes | N/A | O(n³) | Easy | Smooth terrain, not blocky |
| Surface nets | N/A | O(n³) | Medium | Smoother than marching cubes |

**Greedy meshing is correct for your style.** If you want smoother terrain (like Lay of the Land's more organic look), consider marching cubes or surface nets — but that's a major change to the visual style.

**For texturing:** The current greedy mesh doesn't generate UV coordinates, only flat colors. For textured voxels you'll need:
- Per-vertex UVs based on voxel type (requires texture atlas)
- Or triplanar mapping in the shader (no UV needed, projects textures from 3 axes)
- **Triplanar mapping is recommended** — it works naturally with greedy meshing and eliminates UV seam problems.

### 10.4 Physics Engine Considerations

**Current:** `bevy_rapier3d` with trimesh colliders per chunk.

**For your game** (thousands of destructible voxels + hundreds of enemies + player):
- Rapier is good but trimesh collider regeneration is expensive
- Consider: **heightfield colliders** for LOD chunks (much cheaper than trimesh)
- Consider: **voxel-specific collision** for the player and enemies (custom raycast against voxel data, no physics engine needed for terrain collision)
- Keep Rapier for entity-to-entity physics (drops, projectiles, ragdolls)

### 10.5 Enemy Count Strategies

For "lots and lots of enemies":

| Approach | Max Enemies | CPU Cost | Complexity |
|----------|-------------|----------|------------|
| Full Rapier physics per enemy | ~100-200 | Very high | Low |
| Custom simple physics + Rapier for player | ~500-1000 | Medium | Medium |
| Grid-based movement (no physics engine) | ~2000-5000 | Low | Medium |
| GPU instanced with simple AI | ~10,000+ | Low (GPU) | High |

**Recommendation:** Use a tiered approach:
- **Near player (<50m):** Full AI, Rapier physics, animations — max ~100 enemies
- **Medium range (50-200m):** Simple steering, no physics, billboard sprites or low-poly — max ~500
- **Far (>200m):** Just data points, rendered as dots or not at all

### 10.6 Multiplayer Architecture Considerations

Your docs already describe a client-server model. Key things to get right early:

1. **Deterministic world generation** — Same seed = same terrain on all clients. You already have this (good).
2. **Chunk ownership** — Who "owns" a chunk's modifications? The server. Clients send modification requests, server validates and broadcasts.
3. **Delta compression** — Don't send full chunks over the network. Send only the voxels that changed since generation.
4. **Prediction** — Clients predict movement locally, server corrects. Standard netcode.

**Don't build multiplayer yet**, but keep it in mind: avoid designs that assume single-player (global mutable state, no concept of "player ID", etc.).

### 10.7 World Size Comparison

| Game | Voxel Size | World Size | Chunk Size | Approach |
|------|-----------|------------|------------|----------|
| Minecraft | 1.0m | ~60M × 60M blocks | 16×16×16 | Regions, disk streaming |
| Lay of the Land | ~0.25m | Unknown | Unknown | Procedural |
| **Your game** | **0.1m** | **TBD** | **32×32×32** | **Needs regions** |

With 0.1m voxels, a Minecraft-sized world (60M blocks) would be 600M voxels across. That's 18.75 million chunks in one axis. This is technically possible with region-based streaming, but you probably want a smaller world (~1-10 km per dimension = 10,000-100,000 chunks per axis, which is very feasible with regions).

**Recommendation:** Each "world" could be 2km × 2km (20,000 × 20,000 voxels = 625 × 625 chunks). This is large enough to feel expansive, small enough to populate with structures and enemies, and manageable for memory/disk.

---

## 11. File-by-File Findings

### `src/voxel/dynamic_chunks.rs`
- **Bugs:** Hardcoded seed `12345`, density computed sequentially despite Rayon import
- **Performance:** ~600 KB temporary allocations per chunk (intermediate Vecs), double pass over data (coordinates → densities → copy)
- **Fix priority:** HIGH (Bugs #1 and #2 live here)

### `src/voxel/chunk_loading.rs`
- **Bugs:** `remove(0)` O(n), LOD-to-Real orphans ChunkMap, distance from origin not player, LOD generated synchronously
- **Performance:** Iterates ~64K positions on player move, all transitions iterate all chunks per frame
- **Fix priority:** HIGH (Bugs #4, #5, #6)

### `src/voxel/greedy_meshing.rs`
- **Correctness:** Algorithm is correct and standard
- **Performance:** 192 Vec allocations per chunk (reusable), 6,144 cross-chunk lookups per chunk (pre-fetchable)
- **Fix priority:** MEDIUM

### `src/voxel/biomes.rs`
- **Bug:** CRITICAL — FastNoiseLite allocated per call (Bug #1)
- **Design:** Biome thresholds have gaps (intentional default to Plains), moisture noise underutilized
- **Fix priority:** CRITICAL

### `src/voxel/voxel_types.rs`
- **Issue:** Grass layer is only 0.1m thick (height threshold 1.5-1.6m), making grass extremely rare
- **Fix priority:** LOW (cosmetic)

### `src/core/constants.rs`
- **Issue:** Mixed units (chunks vs meters), some constants unused (`MAX_MERGED_SIZE`), duplicated values between here and `chunk_loading.rs`
- **Fix priority:** LOW

### `src/voxel/lod_system.rs`
- **Issue:** Only updates BaseChunks, not LodChunks. Iterates all chunks every frame.
- **Fix priority:** MEDIUM

### `src/voxel/lod_chunks.rs`
- **Issue:** Hardcoded magic numbers (`32`, `0.1`). Binary search range too wide (-50 to +50 when terrain is -1 to +13).
- **Fix priority:** LOW

### `src/voxel/destruction.rs`
- **Bug:** Collider not updated after voxel break (Bug #3). ChunkMap defined here instead of in a shared module.
- **Fix priority:** HIGH

### `src/voxel/spatial_hash.rs`
- **Issue:** O(n) duplicate check in insert. `cleanup_empty_cells` never called. Query allocates Vec per call.
- **Fix priority:** LOW

### `src/voxel/octree.rs`
- **Issue:** Never used for functional queries. `find_nearest` is O(n) not O(log n). Bounds too small for chunk radius.
- **Recommendation:** Remove entirely.
- **Fix priority:** LOW

### `src/voxel/downsampling.rs`
- **Bug:** Dirt/Stone swapped (Bug #7). Wastes memory (16³ array for 4³ data).
- **Fix priority:** MEDIUM (blocks LOD re-enablement)

### `src/voxel/frustum_culling.rs`
- **Issue:** FOV hardcoded (110°) vs GameSettings (90°). Only culls BaseChunks. Crude sphere approximation.
- **Fix priority:** MEDIUM (disabled, needs fixing before re-enabling)

### `src/voxel/chunk_cache.rs`
- **Issue:** Disabled. No buffered I/O (~69K syscalls per chunk). No compression. Flat directory structure.
- **Fix priority:** LOW (rewrite in Phase 2)

### `src/voxel/tools.rs`
- **Status:** Clean design, minor issues only (all tools have same speed/durability).
- **Fix priority:** NONE

### `src/main.rs`
- **Issue:** All systems chained. Initial generation blocks main thread. Empty chunks still spawned as entities. Octree initialized but unused.
- **Fix priority:** MEDIUM

### `src/core/resources.rs`, `src/core/events.rs`, `src/core/states.rs`
- **Status:** Minimal but clean. GameSettings needs expansion. Events only for jump/land.
- **Fix priority:** LOW

---

*This document should be updated as fixes are applied and new systems are designed.*

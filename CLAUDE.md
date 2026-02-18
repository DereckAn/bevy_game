# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

```bash
cargo build                # Debug build (opt-level 1)
cargo build --release      # Release build
cargo run                  # Run game (debug)
cargo run --release        # Run game (optimized, recommended for testing perf)
cargo test                 # Run all tests
cargo test --lib           # Library tests only
cargo test <test_name>     # Run a single test
```

Dev profile uses opt-level=1 for fast compilation; dependencies compile at opt-level=3.

## Architecture

This is a **Bevy 0.17 ECS voxel game** with Rapier3D physics. The codebase is bilingual (English code identifiers, Spanish comments and docs).

### Plugin Structure (registered in main.rs)

- **PlayerPlugin** (`src/player/`) — First-person controller: WASD movement, mouse look, jump. Capsule collider with Rapier3D physics.
- **PhysicsPlugin** (`src/physics/`) — Rapier3D integration, collider generation from chunk meshes, voxel drop physics.
- **UIPlugin** (`src/ui/`) — Main menu (Play/Settings) and in-game HUD. Manages `GameState` transitions.
- **DebugPlugin** (`src/debug/`) — FPS/frame-time overlay.

### Game States (`src/core/states.rs`)

`GameState` enum: `MainMenu` → `InGame` → `Paused`. Systems are gated on these states.

### Voxel System (`src/voxel/` — bulk of the codebase)

**Chunk model**: 32x32x32 voxels per `BaseChunk`, heap-allocated (`Box<[VoxelType]>`). VoxelType is `#[repr(u8)]` for memory efficiency (Air, Dirt, Stone, Wood, Metal, Grass, Sand). Constants in `src/core/constants.rs`.

**Terrain generation pipeline** (async, parallelized with Rayon):
1. `biomes.rs` — BiomeGenerator selects biome via 2D FastNoiseLite noise (Plains, Hills, Mountains, Valley, Plateau)
2. `dynamic_chunks.rs` — TerrainGenerator creates 3D density field, converts to voxel types by height
3. `greedy_meshing.rs` — Combines adjacent same-material faces into larger quads (70-95% triangle reduction)
4. `chunk_loading.rs` — Async chunk loading/unloading around player (AsyncComputeTaskPool, 32 chunks/frame max)

**Key resources**: `ChunkMap` (HashMap<IVec3, Entity>), `ChunkOctree`, `SpatialHashGrid`.

**Destruction**: `destruction.rs` raycasts from camera, breaks voxels based on tool effectiveness, re-meshes dirty chunks.

### Disabled Systems (code present but not active)

- **Frustum culling** (`frustum_culling.rs`) — Has visibility bugs, chunks disappear incorrectly
- **Disk cache** (`chunk_cache.rs`) — Too slow for real-time
- **LOD downsampling** (`downsampling.rs`) — Overflow panic bug
- These produce many unused-code warnings, which is expected

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `bevy` 0.17.3 | Game engine (ECS, rendering, input) |
| `bevy_rapier3d` 0.32.0 | Physics (rigid bodies, colliders) |
| `bevy-inspector-egui` 0.35.0 | Debug inspector UI |
| `fastnoise-lite` 1.1.1 | Procedural noise for terrain |
| `rayon` 1.11.0 | Parallel chunk generation |

### Performance Context

Current: ~30-45 FPS. Main bottleneck is vertical chunk generation (5 Y-levels per XZ position). See `STATUS.md` for current issues and `docs/optimization_roadmap.md` for the optimization plan.

## Project Documentation

- `STATUS.md` — Current project state, known bugs, next priorities
- `docs/architecture.md` — Detailed technical architecture
- `docs/optimization_roadmap.md` — Performance optimization phases (6-10)
- `docs/roadmap.md` — Long-term feature roadmap

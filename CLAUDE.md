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

**Bevy 0.17 ECS voxel game** with Rapier3D physics. Bilingual: English code identifiers, Spanish comments/docs. Plugins registered in `main.rs` (Player, Physics, UI, Debug). `GameState` enum (`src/core/states.rs`): `MainMenu` → `InGame` → `Paused`; systems are gated on these states.

### Voxel System (`src/voxel/` — bulk of the codebase)

**Chunk model**: 32x32x32 voxels per `BaseChunk`, heap-allocated (`Box<[VoxelType]>`). VoxelType is `#[repr(u8)]` for memory efficiency (Air, Dirt, Stone, Wood, Metal, Grass, Sand). Constants in `src/core/constants.rs`.

**Terrain generation pipeline** (async, parallelized with Rayon):
1. `biomes.rs` — BiomeGenerator selects biome via 2D FastNoiseLite noise (Plains, Hills, Mountains, Valley, Plateau)
2. `dynamic_chunks.rs` — TerrainGenerator creates 3D density field, converts to voxel types by height
3. `greedy_meshing.rs` — Combines adjacent same-material faces into larger quads (70-95% triangle reduction)
4. `chunk_loading.rs` — Async chunk loading/unloading around player (AsyncComputeTaskPool, 32 chunks/frame max)

**Key resources**: `ChunkMap` (HashMap<IVec3, Entity>), `ChunkOctree`, `SpatialHashGrid`.

**Destruction**: `destruction.rs` raycasts from camera, breaks voxels based on tool effectiveness, re-meshes dirty chunks.

### Removed Systems

The disk cache (`chunk_cache.rs`) and LOD downsampling (`downsampling.rs`) were deleted as dead code (see git history). Frustum culling (`frustum_culling.rs`) is active and registered in `main.rs`.

Remaining unused items are annotated with `#[allow(dead_code)]` plus a one-line rationale — they are placeholders for planned features (tool durability/types, voxel material properties, game settings, extra menu actions), not accidental dead code.

### Performance Context

Current: ~30-45 FPS. Main bottleneck is vertical chunk generation (5 Y-levels per XZ position). See `STATUS.md` for current issues and `docs/optimization_roadmap.md` for the optimization plan.

## Project Documentation

- `STATUS.md` — Current project state, known bugs, next priorities
- `docs/optimization_roadmap.md` — Performance optimization phases
- `docs/code_review_and_roadmap.md` — Code review findings and roadmap
- `docs/world_limits.md` — World size/coordinate limits
- `plan.md` — Working plan



## Behavioral guidelines to reduce common LLM coding mistakes. Merge with project-specific instructions as needed.

**Tradeoff:** These guidelines bias toward caution over speed. For trivial tasks, use judgment.

## 1. Think Before Coding

**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them - don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

## 2. Simplicity First

**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

## 3. Surgical Changes

**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it - don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that YOUR changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

## 4. Goal-Driven Execution

**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

---

**These guidelines are working if:** fewer unnecessary changes in diffs, fewer rewrites due to overcomplication, and clarifying questions come before implementation rather than after mistakes.
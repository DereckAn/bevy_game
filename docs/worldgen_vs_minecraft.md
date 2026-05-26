# Worldgen: This Game vs. Minecraft

A comparison of how this voxel game generates its world versus how Minecraft does it. Written as background/learning material — not a TODO list.

## What's the same

- **Seed-deterministic**: same seed → same world, every time.
- **Chunk streaming**: the world is divided into chunks, loaded/unloaded around the player instead of all at once.
- **Coherent noise**: both use Perlin/Simplex-family noise as the base signal (this game uses FastNoiseLite's OpenSimplex2).

## The big difference — heightmap vs. true 3D noise

**This game**: terrain is `density = height(x, z) − y`. The ground height is a *function of the horizontal position* — exactly one surface height per `(x, z)` column (`biomes.rs::generate_height`, `dynamic_chunks.rs::generate_terrain`).

A direct consequence: **caves, overhangs, and floating islands are mathematically impossible**. For any column, everything below `height` is solid and everything above is air — always. The density formula is monotonic in `y`, so there can never be air *under* solid.

**Minecraft**: samples noise at `(x, y, z)` — real **3D noise**. "Solid-ness" varies independently at every height, so terrain is not tied to a single surface height. That's why Minecraft has overhangs, floating islands, and noise-based caves. Modern Minecraft (1.18+) builds this from a configurable graph of **density functions**.

> This is the single change with the biggest ripple effect: moving from the heightmap density to true 3D noise is what would unlock caves/overhangs here.

## Generation is a pipeline, not one step

**This game** — two stages:
1. Compute the 2D heightmap (one noise eval per XZ column).
2. Assign material by depth below the surface (grass / dirt / stone).

**Minecraft** — a long, staged pipeline per chunk, each stage potentially depending on neighbor chunks:
1. Base noise shape (3D density field).
2. Surface rules (which block caps each column, per biome).
3. **Carvers** — caves and ravines gouged out *after* the base terrain.
4. **Features / decoration** — trees, ores, lakes, flowers, etc.
5. **Structures** — villages, dungeons, strongholds.
6. Lighting (sky + block light propagation via flood fill).

## Biomes

- **This game**: one smooth "continentalidad" field interpolating valley ↔ mountain (a single noise channel driving base height + amplitude).
- **Minecraft (1.18+)**: **multi-noise** — six independent climate parameters (temperature, humidity, continentalness, erosion, weirdness, depth) whose combination selects the biome. This is how deserts, jungles, and mountains sit coherently next to each other.

## Persistence — different philosophies

- **Minecraft**: generates a chunk **once**, then saves the *entire chunk* to disk (region `.mca` files, "Anvil" format) and reloads it forever after. Never regenerates. Robust against changing the generator, but storage grows with explored area (whole chunks saved).
- **This game**: regenerate from seed + replay a small **diff** of player edits (the `VoxelDiffs` system). Stores only *what changed* (a few KB) instead of the whole chunk (~175 KB). Tiny and bandwidth-friendly (good for multiplayer), but **breaks if the seed or generation algorithm ever changes** — diffs are only meaningful relative to the terrain they were recorded against.

## Scale

- **This game**: voxels are **10 cm** (`VOXEL_SIZE = 0.1`). A 32³ chunk is **3.2 m** across.
- **Minecraft**: blocks are **1 m**. A chunk is **16 m** wide × 384 m tall.

The small voxel size here is *why* greedy meshing matters so much (far more faces to combine) and why the player crosses a chunk boundary every few seconds (the source of performance finding #7 — the chunk-boundary hitch).

## Summary table

| Aspect | This game | Minecraft |
|---|---|---|
| Base terrain | 2D heightmap (`height(x,z) − y`) | 3D noise / density functions |
| Caves / overhangs | Impossible (monotonic in Y) | Yes (3D noise + carvers) |
| Generation stages | Height → material-by-depth | Noise → surface → carve → decorate → structures → light |
| Biomes | 1 continuous continentalidad field | 6-parameter multi-noise climate |
| Persistence | Seed + diffs (store changes only) | Save whole chunk to disk (region files) |
| Voxel size | 10 cm | 1 m |
| Chunk size | 32³ (3.2 m) | 16×16×384 (16 m wide) |
| World bounds | Finite (radius 128 chunks) | Effectively infinite |
| LOD | Two-tier (Real + surface-only LOD) | None by default (mods like Distant Horizons add it) |

# Plan: Medieval buildings & cities (hand-made kit + procedural assembly)

## Context
You want big, detailed medieval buildings and cities that don't repeat, and you want to author the
details yourself. The game has no structure system yet: no `.vox` loading, no serde, no prefabs.
It does have parts that fit this well:
- deterministic generation from seed + cell (trees, `src/vegetation/trees/placement.rs`);
- a pure height function, `BiomeGenerator::generate_height` (`src/voxel/biomes.rs:141`);
- a finite world, `WORLD_CHUNK_RADIUS = 128`, about 820 m across;
- the "seed + diffs" editing model (`src/voxel/destruction.rs:55`).

**Recommendation: a hybrid.** You hand-author small **modules** in MagicaVoxel: wall segments,
corners, doors, windows, roof pieces, chimneys, towers, props. Code then **assembles** them into
buildings, and buildings into villages and cities. Variety comes from:
1. **Combinatorics:** random module variants per slot, footprint shapes and floor counts.
2. **Material styles:** templates paint "semantic" colors (wall, trim, roof…) that code remaps per building.
3. **Rotation and mirroring** of modules.
4. **Hand-made landmarks** (castle, cathedral) as whole prefabs, placed once per city.

This is how Townscaper-style games and most voxel city generators work. You spend your time on
detail, and the code spends its time on variety.

## Constraints found in the repo (must solve first)
| Constraint | Where | Fix |
|---|---|---|
| Voxels are 0.1 m, so a 3 m floor is 30 voxels. MagicaVoxel's limit is 256³ (25.6 m) per model | `src/core/constants.rs:8` | Fine for modules; landmarks can be split into several models |
| No per-voxel color; 24 VoxelTypes with 1 color each (shader adds tone variation) | `src/voxel/voxel_types.rs`, `assets/shaders/palette_extension.wgsl` | Add about 15 medieval materials (u8 allows 256). Palette index = VoxelType id |
| Normal world `y_max = 4` gives about 16 m of height, so towers get cut off | `src/voxel/chunk_loading.rs:293-303` | Raise `y_max` and report structure height to `chunk_is_above_terrain` (`:672`), as trees do (`:705`) |
| Buildings need flat ground | `biomes.rs:141` | Flatten the height function inside settlement areas (stays pure, so LOD matches) |
| Only water is transparent; no glass | `greedy_meshing.rs:195` | Later: route Glass through the water/translucent layer |
| Column LOD beyond about 100 m would make buildings vanish | `src/voxel/lod_chunks.rs` | Later: add building roof heights to the LOD heightmap |

## Scale conventions (decide once, write in `docs/`)
- **Module grid = 32 voxels (3.2 m).** This equals a chunk and fits a medieval floor height.
- Wall thickness is 4 voxels. Doors are about 10×22 voxels. Windows sit on a fixed sill and head height.
- Every module has its origin at the bottom-left-back corner and faces −Z. This keeps rotation predictable.

## Phases

### Phase 1: Materials + MagicaVoxel palette
- Add medieval VoxelTypes: Cobblestone, StoneBrick, Plaster, DarkTimber, Planks, RoofTile, Thatch,
  Glass, Iron, Straw, Gravel(road), Mossy stone… Each one means touching the enum, `from_u8`, `properties`,
  `palette_of`, the WGSL `SPREADS` array and its clamp, and `VOXEL_TYPE_COUNT`
  (see the checklist in `voxel_types.rs` and `docs/palette_system_plan.md`).
- Reserve a range of **semantic indices** (for example 200–215): `WALL_PRIMARY`, `WALL_SECONDARY`, `TRIM`,
  `ROOF`, `FRAME`, `FLOOR`, `ACCENT`. Templates paint with these, and the assembler maps them to real
  materials per `MaterialStyle`. The same house can come out in stone, plaster or timber.
- Add a small test or bin that **exports a 256-color `palette.png`** from `voxel_color()` values, plus
  distinct marker colors for the semantic slots. You load it in MagicaVoxel so every index you paint
  maps 1:1 to a game material.
- ✅ Verify: open the palette in MagicaVoxel, paint a test cube, and see that colors match in game (Phase 2).

### Phase 2: `.vox` import → `Template`
- Add the `dot_vox` crate. That is the one new dependency; the format is fiddly enough to justify it.
- New module `src/structures/` (a `StructuresPlugin`). It loads every `assets/structures/**/*.vox` at startup into
  `Template { size: UVec3, voxels: Box<[u8]>, tags }`. It swaps MagicaVoxel Z-up to Bevy Y-up.
- Tags come from the file name, e.g. `wall_window_01.vox` → kind=`wall_window`, variant 01. There is no metadata format.
- Put templates in a `StructureLibrary` resource wrapped in `Arc`, so async chunk tasks can read it.
- ✅ Verify: a unit test loads a tiny `.vox` fixture, then asserts its size and one voxel id at a known coordinate.

### Phase 3: Stamp one prefab into the world (your authoring loop)
- `Placement { template_id, origin: IVec3, rotation: 0..4, mirror, style }`, plus a function
  `stamp_into_chunk(chunk, placement, library)` that copies only the part overlapping the chunk.
  This is the same clipping idea as the realistic oak (`realistic_oak.rs:131`).
- Call it from `BaseChunk::generate_terrain` right after trees (`dynamic_chunks.rs:123-126`), before diffs
  are applied (`chunk_loading.rs:447`).
- **Debug key:** place a chosen template or building in front of the player, and re-roll it with a new seed.
  This is your main tool for iterating on modules. Pair it with Bevy asset hot reload of `.vox` if that is cheap.
- ✅ Verify: a hand-made house appears and spans chunk borders without seams. Rotations 0–3 look correct.

### Phase 4: Building assembler (procedural buildings from modules)
- Input: footprint (a rectangle or L-shape in module cells), floor count, `MaterialStyle`, seed.
- Rules are simple and tag-based: corners go at corners; ground floor gets `wall_door` on the street side; upper
  floors get `wall_window`/`wall`; medieval jetties (upper floor overhangs 1 module); the roof is picked by
  footprint (`roof_edge`, `roof_ridge`, `roof_corner`, `gable`); chimneys and props come at random.
- Output: `Vec<Placement>` (module ids + transforms), **not** a voxel blob. Modules are shared, so a whole city
  costs kilobytes.
- Wave Function Collapse only if tag rules become painful. Skip it for now.
- ✅ Verify: the debug key generates 20 different houses from about 15 modules. A unit test checks that a 2×3
  footprint yields 4 corner placements.

### Phase 5: Settlements (villages → cities)
- The world is finite, so **generate every settlement once at world start**. Store an `Arc<WorldStructures>`
  holding all `Placement`s bucketed by chunk (`HashMap<IVec3, Vec<idx>>`). Chunk generation then just looks up
  its bucket. There is no per-chunk re-gathering like trees do.
- Pick 1–4 settlement sites from the seed, preferring flat, low-slope land (sample `generate_height`).
- Layout: a main road spine with noise-bent branches → subdivide road frontage into lots → one building per lot,
  with size tier by distance to center (houses at the edge, 3–4 floor townhouses in the core) → a plaza
  and one landmark prefab (church/keep) at the center → optional city wall ring built from wall/tower modules.
- Terrain: `generate_height` flattens toward the settlement base height inside a radius with a smoothstep falloff.
  Roads paint Gravel/Cobblestone at the surface. `place_trees` skips cells inside settlement bounds.
- Raise `y_max` for Normal. `chunk_is_above_terrain` must consider the tallest placement in the column.
- ✅ Verify: new worlds with different seeds show different town layouts, trees stay out of streets, and nothing is clipped at the top.

### Phase 6: Polish (only when needed)
- LOD: add building roof heights and colors into `ColumnLods` so towns are visible from far away.
- Glass through the translucent layer; emissive lanterns.
- Interiors: modules can include floors and stairs from the start, and furniture props later.
- More styles and biomes: a new `MaterialStyle` plus a new module folder (e.g. `assets/structures/desert/`).
  The assembler code stays the same.
- Warn: adding structures changes generation, so old `VoxelDiffs` won't line up. Accept it or reset saves.

## What you author (Phase 1–4 starter kit, about 20 .vox files)
`corner_{01,02}`, `wall_{01..03}`, `wall_window_{01..04}`, `wall_door_{01,02}`, `floor_beam`,
`roof_edge`, `roof_ridge`, `roof_corner`, `gable_{01,02}`, `chimney_01`, `stairs_01`, plus 1 landmark
(`church_01`, which can be several models). Paint only with the semantic indices so styles can recolor it.

## Critical files
- New: `src/structures/{mod.rs, vox.rs, assembler.rs, settlement.rs}`, `assets/structures/medieval/*.vox`
- Modified: `src/voxel/voxel_types.rs`, `src/voxel/palette.rs`, `assets/shaders/palette_extension.wgsl`,
  `src/voxel/dynamic_chunks.rs`, `src/voxel/chunk_loading.rs`, `src/voxel/biomes.rs`,
  `src/vegetation/trees/placement.rs`, `src/main.rs`, `Cargo.toml`
- Reuse: `generate_height` (biomes.rs:141), tree clipping (`realistic_oak.rs`), ceiling logic (`tree_ceiling_for_chunk`, placement.rs:426), `DirtyChunk` remeshing.

## Verification (end to end)
`cargo test structures` for the loader, assembler and layout tests. Then `cargo run --release`:
1. Use the debug key to spawn houses and re-roll them.
2. Start new worlds with 3 seeds and fly to the towns.
3. Check that FPS doesn't drop against the current 30–45 baseline.

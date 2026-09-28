# Mangrove Biome

The fourth world type (`WorldKind::Mangrove`): the **first biome with water**. A
flat coast at sea level with turquoise lagoons and winding river channels, muddy
tidal flats, and mangrove trees on arching stilt roots. Selected from the main
menu like the other kinds.

Biomes are **not** placed procedurally — `WorldKind` is a global resource chosen
once at the menu ([`core/resources.rs`](../src/core/resources.rs)) and threaded
through the generation code.

This is **Phase A** (settled water). Minecraft-style *flowing* water is Phase B
(not yet implemented) — see the roadmap note at the end.

---

## 1. New voxel types

Added to `VoxelType` in [`voxel/voxel_types.rs`](../src/voxel/voxel_types.rs);
`VOXEL_TYPE_COUNT` bumped **20 → 24**.

| id | Variant | Role |
|----|---------|------|
| 20 | `Water` | translucent water (own mesh + material, non-collidable) |
| 21 | `Mud` | tidal-flat surface |
| 22 | `MangroveWood` | trunks + prop roots |
| 23 | `MangroveLeaves` | dense canopy |

Same exhaustive/mirror sites as any voxel type were updated: `properties()`,
`from_u8()`, `from_depth()` (mud → sand → stone), the enum + count, `palette.rs`,
and the GPU mirror [`assets/shaders/palette_extension.wgsl`](../assets/shaders/palette_extension.wgsl)
(`SPREADS` grown to 24 rows, clamp raised to `23u`). `Water` is also added to the
non-collidable list in `is_collidable()` and has a new `is_water()` helper.

Note: `Water` does **not** use the tonal palette — it renders through its own
translucent `StandardMaterial`, so its `SPREADS` row is flat/unused.

---

## 2. Colors (from the reference photos)

Base sRGB constants in [`vegetation/config.rs`](../src/vegetation/config.rs):
`WATER_COLOR` (coastal turquoise), `MUD_COLOR` (wet brown), `MANGROVE_WOOD_COLOR`
(grey-brown), `MANGROVE_LEAVES_COLOR` (vivid green).

---

## 3. Terrain: flat coast + sea level + rivers

- `SEA_LEVEL_M` in [`core/constants.rs`](../src/core/constants.rs): the fixed water
  plane height. Any empty space below it becomes `Water` at generation.
- [`voxel/biomes.rs`](../src/voxel/biomes.rs): `MANGROVE_*` relief constants give a
  near-flat coast whose valleys sit just below sea level (lagoons) and whose islets
  rise just above it. A new low-frequency `river_noise` field carves **winding river
  channels** along its zero-contour, dropping terrain below sea level so it floods.
- [`voxel/dynamic_chunks.rs`](../src/voxel/dynamic_chunks.rs) `generate_terrain`:
  one branch fills `Air` below `SEA_LEVEL_M` with `Water` (mangrove only).
- [`voxel/chunk_loading.rs`](../src/voxel/chunk_loading.rs): `y_max = 3` for the flat
  biome.

---

## 4. Water rendering (translucent, separate mesh)

Water can't share the opaque chunk mesh: that mesh's vertex **alpha** encodes the
voxel id for the palette shader. So water is meshed and drawn separately.

- [`voxel/greedy_meshing.rs`](../src/voxel/greedy_meshing.rs): the mesher's
  `collidable_only: bool` became `enum MeshLayer { Opaque, Water, Collider }`.
  `Opaque` = solid minus water (terrain shows under/beside water); `Water` = water
  faces exposed to air only (water-vs-water and water-vs-terrain culled);
  `Collider` unchanged. New `greedy_mesh_basechunk_water(...)`.
- [`voxel/chunk_loading.rs`](../src/voxel/chunk_loading.rs): `ChunkMaterials` gains a
  shared translucent water `StandardMaterial` (`AlphaMode::Blend`, `cull_mode: None`).
  Each real chunk spawns a **child entity** carrying the water mesh + that material,
  no collider. It despawns in cascade with the chunk.

---

## 5. Mangrove tree

[`vegetation/trees/mangrove.rs`](../src/vegetation/trees/mangrove.rs):
`mangrove_template` builds a short tapered `MangroveWood` trunk, 4-6 arching stilt
roots (`voxelize_tapered`, relabeled `Wood → MangroveWood` like the snowy pine),
and a wide low canopy of `MangroveLeaves` (three `add_leaf_blob`s). Wired into
[`placement.rs`](../src/vegetation/trees/placement.rs): `TreeKind::Mangrove`, its
`height()`, a `tree_in_cell` branch, and a **waterline gate** in `place_trees` that
only stamps mangroves where the surface is within ~1 m of sea level. Grass tufts are
skipped for the biome ([`vegetation/grass.rs`](../src/vegetation/grass.rs)).

---

## 6. Swim / buoyancy physics

[`player/swim.rs`](../src/player/swim.rs): `water_physics` samples the voxel at the
player capsule; when it's `Water` it lowers `GravityScale` (buoyancy), drags
horizontal speed, and lets **Space swim up / Shift swim down** (idle → gentle rise
to the surface). Runs after `player_movement`. The player spawns with
`GravityScale(1.0)`; `Water` being non-collidable is what lets the capsule enter.

---

## Known limitations (Phase A)
- Distant **LOD** chunks render no water (only nearby real chunks).
- Water renders as full cubes (no per-level surface height).
- Water is **settled** — digging below the waterline does not refill. Flowing
  simulation is Phase B (event-driven local flow triggered by edits near water).

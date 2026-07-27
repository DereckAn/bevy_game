# Ice Biome

The third world type (`WorldKind::Ice`): tall snowy mountains with a
Snow → Ice → Stone surface and white trees. Selected from the main menu like
`Normal` and `Desert`.

Biomes are **not** placed procedurally — `WorldKind` is a global resource chosen
once at the menu ([`core/resources.rs`](../src/core/resources.rs)) and threaded
through the generation code.

---

## 1. New voxel types

Added to `VoxelType` in [`voxel/voxel_types.rs`](../src/voxel/voxel_types.rs);
`VOXEL_TYPE_COUNT` bumped **16 → 20**.

| id | Variant | Role |
|----|---------|------|
| 16 | `Snow` | terrain surface (soft) |
| 17 | `Ice` | band under the snow (harder) |
| 18 | `WhiteWood` | tree trunks |
| 19 | `WhiteLeaves` | tree canopies / snowy needles |

Adding a `VoxelType` touches several exhaustive/mirror sites — all done for these:
`properties()`, `from_u8()`, `from_depth()`, the enum + count, `palette.rs`, and
the GPU mirror `assets/shaders/palette_extension.wgsl` (`SPREADS` grown to 20 rows,
clamp raised to `19u`).

---

## 2. Colors (slate-blue palette)

Base sRGB constants in [`vegetation/config.rs`](../src/vegetation/config.rs). Both
the flat color (`properties()`) and the tonal palette (`palette.rs`) reference
these, and the shader only holds the light/dark multipliers — so editing `config`
recolors the whole biome.

| Material | Const | Hex |
|----------|-------|-----|
| Snow | `SNOW_COLOR` | `#d5dbe2` (lightest) |
| Ice | `ICE_COLOR` | `#b3bfcb` |
| WhiteLeaves | `WHITE_LEAVES_COLOR` | `#778ca4` |
| WhiteWood | `WHITE_WOOD_COLOR` | `#4f6271` (dark trunk) |

`#313c45` (the palette's darkest) is currently unused. Per-voxel tonal variation
still applies on top of each base color (the palette shader).

---

## 3. Terrain — tall mountains

Relief constants in [`voxel/biomes.rs`](../src/voxel/biomes.rs) (`ICE_*`), much
larger than Normal/Desert so peaks reach ~60 m:

- `ICE_VALLEY_BASE ≈ 2`, `ICE_MOUNTAIN_BASE ≈ 40`
- `ICE_MIN_AMPLITUDE ≈ 2`, `ICE_MAX_AMPLITUDE ≈ 14`, `ICE_MOUNTAIN_DETAIL ≈ 5`

Because the surface is so high, the vertical chunk range is raised for Ice in
[`chunk_loading.rs`](../src/voxel/chunk_loading.rs) (`y_max = 20` for `Ice`, vs 4
Normal / 6 Desert), so peaks aren't clipped flat.

### Surface layering

`VoxelType::from_depth` ([`voxel_types.rs`](../src/voxel/voxel_types.rs)) for `Ice`:

```text
depth < 0.1 m  → Snow   (~1 voxel)
depth < 0.5 m  → Ice    (~4 voxels)
else           → Stone
```

Grass tufts are disabled on snow ([`vegetation/grass.rs`](../src/vegetation/grass.rs)
returns early for `Ice`).

---

## 4. Trees

Two species, mixed 50/50 in the `Ice` branch of `tree_in_cell`
([`vegetation/trees/placement.rs`](../src/vegetation/trees/placement.rs)). Templates
in [`vegetation/trees/white_tree.rs`](../src/vegetation/trees/white_tree.rs).

- **Snowy pine** (`snowy_pine_template`) — reuses the normal pine geometry, then
  relabels `PineWood → WhiteWood` and `PineNeedles → WhiteLeaves`. Trunk 40–59.
- **White birch** (`white_birch_template`) — the "simple" tree: a **2×2 (≈4-voxel
  thick)** `WhiteWood` trunk with a `WhiteLeaves` sphere on top. Currently tall:
  trunk **40–60**, canopy radius **4–6** (see `placement.rs` Ice branch).

Distant tree impostors (`lod_chunks.rs`) only cover pine/oak, so ice trees simply
appear when their column becomes Real (no far-distance impostor).

---

## 5. Spawn on the surface

Because Ice peaks are so tall, the old fixed spawn (`y = 20`) could bury the player
inside a mountain. `spawn_player` ([`player/components.rs`](../src/player/components.rs))
now samples the real terrain height at the spawn column and drops the player a few
metres above it.

---

## 6. Menu

A third button, **ICE**, in [`ui/menu.rs`](../src/ui/menu.rs)
(`MenuAction::Play(WorldKind::Ice)`); the existing handler already applies it.

---

## 7. Full file checklist

Everything the biome touches, for reference when adding a fourth biome:

| File | Change |
|------|--------|
| `core/resources.rs` | `WorldKind::Ice` variant |
| `ui/menu.rs` | ICE button |
| `voxel/voxel_types.rs` | 4 voxel types, count, `properties`, `from_u8`, `from_depth` |
| `vegetation/config.rs` | 4 color constants |
| `voxel/palette.rs` | 4 tonal-palette entries |
| `assets/shaders/palette_extension.wgsl` | 4 `SPREADS` rows, array size 20, clamp `19u` |
| `voxel/biomes.rs` | `ICE_*` relief constants + `generate_height` arm |
| `voxel/chunk_loading.rs` | `y_max` arm for `Ice` |
| `vegetation/grass.rs` | `Ice` arm (no tufts) |
| `vegetation/trees/placement.rs` | `SnowyPine`/`WhiteBirch` kinds + Ice placement branch |
| `vegetation/trees/white_tree.rs` | new templates |
| `vegetation/trees/mod.rs` | declare `white_tree` module |
| `player/components.rs` | spawn on surface |

The `WorldKind` matches are exhaustive, so the compiler flags every arm that needs
an `Ice` case — a reliable checklist when adding biomes.

## Verification

`cargo run --release` → **ICE**: tall snowy peaks (not flat-topped), Snow surface
over an Ice band (dig to confirm) over Stone, snowy pines + chunky white birches in
slate tones, and the player lands on the surface rather than inside a peak.

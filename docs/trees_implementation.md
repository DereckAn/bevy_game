# Trees — Step-by-Step Implementation Plan

How to get trees into the world. A tree = a **template of voxels** (trunk of `Wood`
+ canopy of a new `Leaves` type) **stamped** into a chunk's `voxel_types` during
generation. We reuse the existing `Wood` for the trunk; no `Log` type.

Why this is mostly cheap: your existing systems handle the rest for free —
greedy meshing renders any voxel type, destruction breaks any voxel, and the
diff/persistence model saves player edits on top.

There are **three sub-problems**: (A) the leaf material, (B) the tree shape,
(C) placement. Only (C) — placement across chunk borders — needs real care.

---

## Decisions to lock first

These change the code, so pick before implementing (defaults recommended):

1. **Leaf collision** — solid (stand on them, like Minecraft — *simplest*) vs.
   pass-through (bigger change: the mesher/physics treat "solid" as "not Air").
   → **Default: solid.**
2. **Tree shape** — one fixed size first (e.g. trunk 5, canopy radius 2), variety later.
   → **Default: one fixed shape, then add a small random range.**
3. **Where trees grow** — any grass surface (*simplest*) vs. biome-restricted
   (denser in Plains/Hills, none on Mountains — needs `biomes.rs`).
   → **Default: any grass surface for v1.**
4. **Density** — spacing cell size `S` (e.g. 8×8 voxels) + per-cell probability.
   → **Default: start sparse, tune visually.**

---

## Step 1 — Add the `Leaves` voxel type

Small and mechanical. All in `src/voxel/voxel_types.rs`, mirroring `Grass`/`Sand`:

- [ ] Add `Leaves = 7` to the `VoxelType` enum.
- [ ] Add a `properties()` arm: a green `color`, low `hardness` (~0.2), `is_solid: true`,
      `drops_self`, `name: "Leaves"`, a `density`.
- [ ] Add `7 => VoxelType::Leaves` to `from_u8`.
- [ ] (Optional) a color in `voxel_color.rs`. Not required — non-grass types already
      use `properties().color`; only `Grass` is special-cased there.

**Verify:** `cargo build`, and the `test_voxel_type_size` test still passes (must stay 1 byte).
Optional sanity check: hand-place a few `Leaves` voxels and confirm they render/break.

---

## Step 2 — Define the tree template (a pure function)

A function that, given a size, returns the voxel offsets relative to the tree base.
No randomness inside — pure and reproducible.

- [ ] **Trunk**: vertical column of `Wood` at `(0, 0..H, 0)`, height `H` ~4–7.
- [ ] **Canopy**: a blob of `Leaves` near the top — sphere/ellipsoid of radius `R` ~2–3
      centered just below the trunk top. "Inside" = `dx² + dy² + dz² ≤ R²`.
- [ ] Output: conceptually a list of `(offset: IVec3, VoxelType)`.

Keep it a pure function of its parameters so the same tree is always identical.

---

## Step 3 — Deterministic placement (the crux)

Three things to get right.

### 3.1 Determinism (non-negotiable)
Placement **must** be a pure function of `seed + world position`. Never a global RNG.
The world is "seed + player diffs," so a chunk must regenerate **identically** every
load. Hash world XZ to decide "tree here? how big?" — same philosophy as `biome_gen`.

### 3.2 Spacing
Don't put a tree on every column. Trick:
- [ ] Divide the world into a grid of `S×S` cells.
- [ ] Each cell holds **at most one** candidate tree, positioned *jittered within the
      cell* by the hash → natural spacing, no Poisson math.
- [ ] A per-cell probability (and/or biome) decides if the candidate actually grows.

### 3.3 The cross-chunk problem — and why it's not hard here
A tree near a chunk edge has its trunk in one chunk and half its canopy in the
neighbor. The naive fix (writing into neighbor chunks) is messy and order-dependent.

**Clean solution your codebase enables:** generate trees from a **global
deterministic function** and let each chunk compute the parts of any tree that
overlap it. The key fact: `biome_gen.generate_height(world_x, world_z)` is a
**pure function**, so while generating chunk C you can compute the surface height —
and thus the whole tree — of a tree whose *base lives in a neighboring chunk*, and
stamp only the voxels that land inside C. **No neighbor writes, seamless borders.**

### 3.4 The placement pass (pseudocode)
```
for each grid cell whose tree could reach into this chunk
    (cells within MAX_CANOPY_RADIUS of the chunk's XZ bounds):
        decide via hash(cell, seed): is there a tree? what params?
        if yes:
            base_y = generate_height(tree_x, tree_z)   // pure → any chunk agrees
            (optional) reject if not grass / too steep / underwater
            for each (offset, type) in tree_template(params):
                world_voxel = base + offset
                if world_voxel is inside THIS chunk:
                    write it into local voxel_types (usually only into Air)
```
That loop is the entire trick. Same code runs for every chunk; overlapping trees
come out consistent because every input is a pure function of world position + seed.

---

## Step 4 — Wire it into generation

- [ ] Add the placement pass as a **new step at the end of `BaseChunk::generate_terrain`**
      (in `src/voxel/dynamic_chunks.rs`), after the base terrain fill, before the chunk
      is returned. Or a `decorate()` method it calls.
- [ ] It already has everything it needs: `terrain_gen.biome_gen` (for heights of any
      column), `self.position`, and `self.voxel_types` to write into.
- [ ] Only write into `Air` voxels (don't overwrite existing terrain). Trunk grows up
      from the surface into air; canopy sits in air.

**Verify:** `cargo run --release` — trees appear on grass; trees straddling chunk
borders are whole (no half-trees); reloading the area reproduces the same trees;
breaking trunk/leaves works.

---

## Suggested order

1. **Step 1** (`Leaves` type) — tiny, verifiable alone. Build + optional hand-place.
2. **Steps 2+3+4** with **one fixed shape on any grass surface, global-function
   placement** — nail the cross-chunk behavior early.
3. Then add **variety** (size range) and **biome restriction** (needs `biomes.rs`).

---

## Notes / gotchas

- **Surface check:** to place only on grass (not stone tops, not underwater), check the
  surface voxel type via the same depth logic as terrain (`from_depth` at depth 0 = grass),
  and optionally reject steep columns using a slope check (same idea as `slope_at` in
  `greedy_meshing.rs`).
- **Vertical chunk borders:** tall trees can cross a Y chunk boundary too — the same
  global-function approach handles it (the chunk stamps only voxels in its Y range).
- **Don't use global RNG anywhere in placement** — it breaks Real↔regenerate consistency.
- Reference: Part C of `docs/visuals_and_vegetation.md` (this plan implements Approach 1).

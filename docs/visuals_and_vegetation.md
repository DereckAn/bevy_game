# Visuals & Vegetation — Plan and Resources

Reference + learning roadmap for three goals:

1. **Better terrain colors** (the random greens look noisy).
2. **Per-material patterns/textures** (grass, rock, wood, …).
3. **Vegetation** (trees, plants, bushes).

Written as a starting map, not a step-by-step. Each section has: *why the current thing falls short → techniques → how it maps to this codebase → resources*. Links can rot; the resource **names** are the durable part — search them if a URL is dead.

---

## Part A — Better terrain colors

### Why random greens look bad
Picking from 11 **high-contrast, discrete** greens with noise gives a patchy/noisy look because:
- The hues jump (dark `#071309` next to bright `#41BE5E`) — the eye reads discontinuity as "noise," not "grass."
- There's no *shading* cue (no darkening in crevices, no slope/height influence), so it reads flat.
- Discrete palette indices create hard color steps.

### Techniques (roughly easy → advanced)
1. **Use a smooth color *ramp*, not discrete picks.** Drive a single low-frequency noise value `t ∈ [0,1]` through a continuous gradient between **2–3 harmonious greens**. Far calmer than 11 random hues. (We had a version of this before switching to discrete — worth revisiting with a better palette.)
2. **Cosine palettes (Inigo Quilez).** A 4-parameter formula `a + b*cos(2π(c*t+d))` produces smooth, controllable, great-looking gradients. Perfect for natural color variation from one noise value. *(iquilezles.org → "palettes".)*
3. **Tie color to terrain, not just noise.** Real grass varies by:
   - **Slope** — steeper = more dirt/rock showing (blend toward brown on slopes).
   - **Height** — higher = drier/yellower or snow; lower = lusher.
   - **Moisture/biome** — you already have a continentalidad field; add a moisture noise.
4. **Fake ambient occlusion (AO).** Darken voxels next to other solid voxels (crevices, under ledges). This single trick adds the most "depth" to blocky terrain. Search: *"voxel ambient occlusion" / "0fps ambient occlusion for smooth lighting"*.
5. **Work in a better color space.** Interpolating in linear RGB can look muddy; perceptual spaces (Oklab/HSL) give more even transitions. Bevy's `Color` supports several spaces.
6. **Reduce contrast + saturation.** Stylized games usually use a *narrow* value range. Lower the spread between your lightest/darkest green.

### Maps to this codebase
- All of this lives in `src/voxel/voxel_color.rs` (`voxel_color` / `grass_color`). Swapping the discrete-palette pick for a cosine-palette ramp is a small, local change.
- Slope/AO need neighbor info — the mesher (`add_greedy_quad`) knows the face normal and could pass an AO factor; AO needs sampling neighbor voxels in `greedy_meshing.rs`.

### Resources
- **Inigo Quilez — palettes & noise**: `iquilezles.org/articles/` (cosine palettes, value/gradient noise, fbm).
- **The Book of Shaders**: `thebookofshaders.com` (noise, patterns, color — interactive).
- **Lospec Palette List**: `lospec.com/palette-list` (curated stylized palettes; filter by greens/nature).
- **Coolors** `coolors.co` / **Adobe Color** `color.adobe.com` (build harmonious palettes; use "analogous"/"monochromatic" for grass).
- **0fps voxel articles**: `0fps.net` (meshing, AO, texturing voxels).
- Search terms: *"terrain texturing by height and slope"*, *"stylized grass color ramp"*, *"oklab color interpolation"*.

---

## Part B — Per-material patterns / textures

Right now materials are **flat per-voxel colors**. "Patterns" (grass blades, rock cracks, wood grain) need one of two paths:

### Path 1 — Procedural patterns in code (no texture files)
Extend `voxel_color.rs` so each `VoxelType` has its own *function* of position:
- **Wood**: stripes/grain → `sin(local_y * k)` banding + brown ramp.
- **Rock/Stone**: mottled gray via 2–3 octaves of value noise.
- **Sand**: fine high-frequency speckle.
- **Grass**: the coherent ramp from Part A.

Pros: no assets, fits your current vertex-color pipeline, cheap. Cons: limited detail (per-vertex/per-quad granularity; greedy meshing limits resolution unless you split faces like we did for grass tops).

### Path 2 — Actual textures (the Minecraft way)
Apply image textures to voxel faces. This is the real route to crisp patterns. Requires:
1. **UV coordinates** in the mesh — `greedy_meshing.rs` currently emits none. You'd add `Mesh::ATTRIBUTE_UV_0`. (Greedy-merged quads need *tiled/repeating* UVs so the texture repeats per voxel across a merged quad — search *"greedy meshing texture coordinates tiling"*.)
- 2. **A texture atlas or texture array** — one tile per voxel-type face. Texture **arrays** are cleaner than atlases for voxels (no bleeding/mipmap issues).
3. **A material that samples it** — `StandardMaterial { base_color_texture }` for simple cases, or a **custom WGSL material** for per-face/per-type array indexing.
4. **Nearest-neighbor filtering** for a crisp pixel look (`ImageSampler::nearest`), or linear for smooth.

### Triplanar mapping (worth knowing)
Voxel terrain is hard to UV-unwrap. **Triplanar mapping** samples a texture by world position along X/Y/Z and blends by the normal — no UVs needed, no stretching on slopes. Common in voxel/terrain engines. Search: *"triplanar mapping"* (Ben Golus's article is the canonical deep-dive).

### Maps to this codebase
- Path 1: pure `voxel_color.rs` work — natural next step, low risk.
- Path 2: touches `greedy_meshing.rs` (UVs), a new texture asset + sampler setup, and likely a custom material (replacing the white `StandardMaterial` in `ChunkMaterials`). Bigger project; do it once Path 1's limits annoy you.

### Resources
- **Bevy examples** (`github.com/bevyengine/bevy/tree/main/examples`): `3d/texture`, `shader/` (custom materials), `3d/parallax_mapping`. Match the example version to your Bevy `0.17`.
- **Bevy Cheatbook**: `bevy-cheatbook.github.io` (assets, materials, shaders).
- **Free CC0 textures**: `ambientcg.com`, `polyhaven.com` (PBR), **Kenney** `kenney.nl` (stylized/voxel-friendly kits).
- **Triplanar**: Ben Golus "Normal Mapping for a Triplanar Shader" (search).
- **Voxel texturing/meshing**: `0fps.net`, and "Vercidium" voxel-optimization videos on YouTube.
- Search terms: *"bevy texture array"*, *"voxel texture atlas mipmap bleeding"*, *"greedy mesh per-voxel UV"*.

---

## Part C — Trees, plants, and bushes

Three approaches; for a voxel game, **#1 is the natural fit**.

### Approach 1 — Voxel "stamps" (recommended start)
A tree = a small template of voxels (trunk = `Wood`, canopy = a new `Leaves` type) written into chunk data during generation.
- **Add voxel types**: `VoxelType::Leaves` (and maybe `Log`). You already have `Wood`. Update `properties()` + `voxel_color`.
- **Templates**: define small 3D patterns (e.g., a 5×7×5 canopy + trunk column). Start hardcoded; later load from data.
- **Placement**: decide *where* trees go (see "Placement" below), then stamp the template into the chunk's `voxel_types`.
- **When**: a "decoration pass" after `from_depth` terrain in `BaseChunk` generation — or a separate stage. Surfaces (grass tops) are the spawn spots.
- **Fits your systems**: greedy meshing already renders new voxel types; persistence (diffs) already handles edits; breaking works for free.

### Approach 2 — Mesh props (GLTF models / billboards)
Spawn separate 3D models or 2D billboards for grass tufts, flowers, bushes on the surface — *not* voxels. Good for fine detail (grass blades, ferns) that voxels can't show. Bevy loads GLTF; billboards = quads always facing the camera. Search: *"bevy gltf scene"*, *"billboard grass instancing"*.

### Approach 3 — Procedural tree generation (advanced)
Generate trunk/branch geometry algorithmically:
- **L-systems** — grammar-based branching. Classic, great for variety.
- **Space colonization algorithm** — grow branches toward "attractor" points; very natural results.
These produce *meshes*, not voxels (or you voxelize the result). More work; do later if you want unique trees.

### Placement (shared by all approaches)
- **Where**: only on grass surface, not on steep slopes, not underwater, respecting biome (forests denser than plains).
- **How many / spacing**: a density value (noise or biome) + **Poisson-disk sampling** for natural, non-overlapping spacing. Simpler: per-column random with a probability + minimum spacing check.
- **Determinism**: placement must be a pure function of seed + position (like terrain), so a chunk regenerates identically — *critical* for your seed+diffs model. Don't use global RNG; hash the position.
- **⚠️ Cross-chunk caveat**: a tree near a chunk border has its trunk in one chunk and canopy in the next. Options: (a) generate trees from a *deterministic global* function so each chunk can compute the parts of any tree overlapping it, or (b) a post-pass that can write into neighbor chunks. This is the main hard part — search *"minecraft cross-chunk tree generation"* / *"feature placement chunk boundaries"*.

### Maps to this codebase
- New `VoxelType` variants → `voxel_types.rs` (enum + `properties` + `from_u8`), and a color in `voxel_color.rs`.
- A decoration step in `dynamic_chunks.rs::generate_terrain` (or a new `decoration.rs`) that stamps templates after the base terrain.
- Keep placement a hash of world position (mirrors `biome_gen`), so it survives Real↔LOD↔regenerate.

### Resources
- **The Algorithmic Beauty of Plants** (free book): `algorithmicbotany.org/papers/#abop` — the L-systems bible.
- **Space colonization**: Runions et al., "Modeling Trees with a Space Colonization Algorithm" (PDF, searchable).
- **Sebastian Lague** (YouTube): procedural terrain/erosion/coral — great intuition-builders.
- **Red Blob Games**: `redblobgames.com` (noise, distribution, map generation; excellent explanations).
- **Poisson-disk sampling**: Bridson's algorithm (search "Bridson Poisson disk sampling fast").
- **Minecraft Wiki — World generation / Tree**: how a shipping voxel game places features.
- **Kenney Nature Kit** `kenney.nl` (free models if you go the prop route).
- Bevy: `bevyengine.org`, examples `3d/load_gltf`.
- Search terms: *"voxel tree generation algorithm"*, *"procedural foliage placement density map"*, *"poisson disk sampling rust"*.

---

## Suggested learning order

1. **Colors first** (Part A) — cheapest, biggest immediate visual payoff. Try a cosine-palette ramp + lower contrast + fake AO. All in `voxel_color.rs` (+ a little AO in the mesher).
2. **Procedural per-type patterns** (Part B, Path 1) — give rock/wood/sand their own look in `voxel_color.rs`. No assets needed.
3. **Trees as voxel stamps** (Part C, Approach 1) — add `Leaves`, a tree template, deterministic placement on grass. Tackle the cross-chunk caveat early.
4. **Textures** (Part B, Path 2) — only when flat/procedural colors stop satisfying. Bigger lift (UVs, atlas/array, custom material).
5. **Procedural/advanced vegetation** (Part C, Approaches 2–3) — props, billboards, L-systems for variety.

## One-line resource cheat sheet
- Color/noise/palettes: **Inigo Quilez**, **The Book of Shaders**, **Lospec**, **Coolors**
- Voxel meshing/texturing/AO: **0fps.net**, **Vercidium (YouTube)**, **triplanar mapping (Ben Golus)**
- Procedural gen / placement: **Red Blob Games**, **Sebastian Lague**, **Bridson Poisson disk**
- Trees: **Algorithmic Beauty of Plants**, **space colonization (Runions)**, **Minecraft Wiki**
- Bevy specifics: **Bevy examples (GitHub)**, **Bevy Cheatbook**
- Free assets: **Kenney**, **ambientCG**, **Poly Haven**

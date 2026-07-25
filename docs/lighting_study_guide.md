# Lighting Study Guide

A study roadmap for improving the game's lighting, tailored to this Bevy 0.17
voxel project. Most of it is transferable rendering theory; a few items are
Bevy-specific features already available in the codebase.

Context: the game currently uses a `DirectionalLight` (sun) driven by the
day/night cycle, Bevy's physical `Atmosphere`, `Bloom`, `DistanceFog`, a flat
`AmbientLight`, and `StandardMaterial` (metallic-roughness PBR). See
`src/sky/`.

## Topics, in the order they help most

### 1. Exposure & tonemapping
Directly why bright highlights clip to white. Learn how HDR light values get
mapped to the screen, why bright surfaces wash out, and how ACES / AgX /
TonyMcMapface differ. We currently use `Tonemapping::TonyMcMapface` — try
`AcesFitted` and `AgX` and compare.
- Search: "exposure and tonemapping in real-time rendering", "ACES vs AgX".

### 2. Color temperature / white balance (Kelvin)
Why real noon sun isn't pure white, why shadows read blue, why sunsets are
warm. Fixes the "ugly white" look. Real noon sunlight is a slightly warm white
(~5500 K), not `(1,1,1)`.
- Search: "color temperature lighting Kelvin", "warm light cool shadow".

### 3. Physically based light units (lux / lumens)
Bevy's `illuminance` is in real lux; understanding it makes the numbers
meaningful instead of guesswork.
- Reference: Google **Filament** documentation (google.github.io/filament) —
  the gold standard, and it matches how Bevy implements light units.

### 4. PBR materials — albedo / metallic / roughness
The "white reflection" on objects is a specular highlight: its size/sharpness
is controlled by *roughness*, its color by the *light*. Learn the
metallic-roughness workflow.
- Reference: **LearnOpenGL** PBR chapters (learnopengl.com) — best free intro.

### 5. Indirect light / ambient / global illumination
The biggest available upgrade. The scene currently uses a flat `AmbientLight`
(uniform gray), the #1 cause of "flat/ugly" lighting. Real ambient light comes
from the sky and bounces.
- Topics: image-based lighting (IBL), environment maps, ambient occlusion.

### 6. Shadows & ambient occlusion
Cascaded shadow maps (softness, bias/acne) and SSAO for contact shadows in
crevices. Both exist in Bevy.

## Bevy-specific things to try (one step away)

- **Replace flat ambient with sky-based ambient.** The `Atmosphere` can
  generate an environment map that lights objects with the actual sky color —
  warm from the sun side, cool from the sky. Look at `AtmosphereEnvironmentMap`
  / `EnvironmentMapLight`. Single biggest quality jump for this scene.
- **`ScreenSpaceAmbientOcclusion`** on the camera — instant depth in corners.
- **Cascaded shadow config** — tune shadow distance/softness.
- Read Bevy's own runnable examples (they match this version's API):
  `3d/lighting`, `3d/atmosphere`, `3d/tonemapping`, `3d/ssao`, `3d/pbr`,
  `3d/environment_map` in https://github.com/bevyengine/bevy/tree/main/examples

## Creators / written references (graphics-focused)

- **Acerola** — rendering techniques, approachable, strong on stylized + effects.
- **SimonDev** — shaders and graphics math, clear explanations.
- **Inigo Quilez** (iquilezles.org) — reference for procedural noise and SDFs
  (also useful for the clouds).
- **Freya Holmér** — the math behind shaders (smoothstep, interpolation,
  splines), relevant to the `lerp`/`smoothstep` already used in `src/sky/`.
- **Filament docs + LearnOpenGL** — the two written references that cover ~80%
  of PBR lighting correctly.

## Where to start for this project's current issues

1. **Exposure / tonemapping** — fixes the white clipping.
2. **Image-based lighting from the atmosphere** — fixes the flat ambient.

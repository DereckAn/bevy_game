# Work Session Plan — Multi-issue Pass

**Started:** 2026-06-11
**Goal:** Fix cursor/menu, pause menu, destruction holes, terrain uniformity + seed, and bound the world size.
**Out of scope (deferred):** Issue #4 — "The Finals"-style main menu redesign.

## Decisions (locked)
- **Map mode size:** 256×256 chunks (~820m) — finite, bounded world.
- **Seed:** random every launch (new map each time the game starts).
- **Main menu redesign (#4):** deferred to its own future effort. For now only fix the cursor.

---

## Status — done this session ✅

### 1. Mouse / cursor by GameState — ✅ DONE
- Removed window startup cursor-hiding (`main.rs`).
- Cursor now driven by state: `grab_cursor` on `OnEnter(InGame)`, `release_cursor` on `OnEnter(Paused)` and `OnEnter(MainMenu)` (`player/mod.rs`, `player/input.rs`).
- `player_look` / `player_movement` / `cursor_grab_on_click` gated to `in_state(InGame)`.

### 2. ESC → pause menu — ✅ DONE
- `ui/pause.rs`: `toggle_pause` (ESC toggles InGame↔Paused), `setup_pause_menu` / `cleanup_pause_menu`, `pause_button_system`.
- Buttons: Resume (→InGame), Settings (placeholder), Sound (placeholder), Quit to Menu (→MainMenu).
- Gameplay `Update` systems in `main.rs` gated to `in_state(InGame)` → world freezes while paused.

### 4. Terrain uniformity — ✅ DONE
- `biomes.rs` rewritten: continuous height field. `biome_noise` acts as smooth continentalness; base height & amplitude interpolate (`lerp`/`smoothstep`) between valley and mountain. Mountain detail fades in with a smooth weight (no hard threshold). Single shared `terrain_noise` for local detail.
- No more flat→cliff jumps; transitions span hundreds of voxels.

### 5. World seed each launch (Issue #3) — ✅ DONE
- `WorldSeed` resource (`core/resources.rs`), randomized via `WorldSeed::random()` in `main.rs`.
- Threaded into `BaseChunk::new(pos, seed)` / `generate_terrain(seed)` / `TerrainGenerator::new(seed)` at all call sites (`main.rs`, `chunk_loading.rs` ×2).
- Fixed `biomes.rs` bug where terrain noise ignored the passed seed (was hardcoded 12345/54321).

### 6. Bounded world (256×256) — ✅ DONE
- `WORLD_CHUNK_RADIUS = 128` (`core/constants.rs`).
- `update_chunk_load_queue` skips chunk positions with `|x|` or `|z|` > radius → finite map.

### 3. Destruction holes — ✅ PRIMARY CAUSE FIXED (verify in-game for the secondary one)
- **Root cause found:** player starts with a Shovel; `from_density` assigned voxel type by absolute world_y, so any surface below y=0.5m was **Stone** → shovel vs stone = 0.3 effectiveness → ~17s to break → looked unbreakable in low areas.
- **Fix:** new `VoxelType::from_depth(density, depth_below_surface)` — grass on top (~1 voxel), dirt under (~4 voxels), stone deeper. Surfaces are now always grass/dirt → diggable everywhere. Wired into `generate_terrain` (uses heightmap) and LOD surface coloring. Tests added.

---

### Player spawn timing — ✅ DONE (fixed after first report)
- **Bug:** player spawned at `Startup` as a dynamic body and free-fell through gravity during the menu (physics isn't state-gated), ending up far below where terrain later generates → "map over my head".
- **Fix:** player now spawns on the `MainMenu → InGame` transition (Y=20, above max terrain ~12m) and despawns on `InGame → MainMenu`. Terrain `setup` also moved to that same transition. Pause/resume (InGame↔Paused) no longer re-spawns the player or regenerates terrain.

### Main menu black screen — ✅ DONE (fixed after second report)
- **Bug:** the menu UI rendered against the player's `Camera3d`, which used to exist at `Startup`. Moving player spawn to the InGame transition left the menu with no camera → black screen.
- **Fix:** `setup_main_menu` spawns a `Camera2d` tagged `MainMenuUI` (cleaned up on exit). Transition order (OnExit → OnTransition → OnEnter) ensures the 3D and 2D cameras never coexist.

### Camera ambiguity + frozen world after Quit-to-Menu → Play — ✅ DONE (fixed after third report)
- **Bug:** Quit-to-Menu goes `Paused → MainMenu`, but `despawn_player` was gated to `InGame → MainMenu`, so the player's `Camera3d` leaked. The menu then spawned `Camera2d` → two active cameras (the `Camera order ambiguities` warning). Clicking Play again spawned a *second* player + camera + duplicate world → input split → frozen.
- **Fix:**
  - `despawn_player` now runs on `OnEnter(MainMenu)` (covers InGame→MainMenu *and* Paused→MainMenu).
  - New `teardown_world` (`chunk_loading.rs`) runs on `OnEnter(MainMenu)`: despawns all chunk entities (via live component queries, not stored IDs) + directional light, clears `ChunkMap`/`SpatialHashGrid`/`ChunkLoadQueue`, resets `ChunkOctree`. Replays start clean.
  - Note: despawning by `ChunkMap` values first caused benign "entity does not exist" warnings (stale IDs); switched to `Query<Entity, Or<(With<BaseChunk>, With<LodChunk>, With<ChunkGenerationTask>)>>` which only yields live entities.
  - Also fixed a separate benign "entity does not exist" warning on Quit-to-Menu: menu/pause **child buttons** were tagged with the cleanup marker (`MainMenuUI`/`PauseMenuUI`), so despawning the tagged root recursively despawned them, then the cleanup loop hit them again. Markers now live only on root nodes (+ menu camera); children clean up in cascade.
  - Result: exactly one active camera at all times; Quit-to-Menu → Play works without duplicates or freeze.

---

### Performance pass — ✅ DONE (round 1, rebalanced)
- **Bounded completion work per frame, balanced against generation.** `complete_chunk_generation_system` was processing *every* finished async task in one frame, each doing a full neighbor-aware greedy remesh **and** a Rapier trimesh collider build on the **main thread** — the stutter source.
  - **First attempt (BUG):** capped completions to 6/frame while generation still started 32/frame → integration fell behind, generated-but-unmeshed chunks piled up → "holes everywhere" while moving.
  - **Fix:** `MAX_CHUNKS_PER_FRAME` 32→**16** (throttle generation at the source) and `MAX_CHUNK_COMPLETIONS_PER_FRAME` = **24** (completion must stay ≥ generation so integration never falls behind). Both below the old 32-burst → smoother frame time, no backlog/holes.
- **Removed wasted async meshing.** Tasks computed `greedy_mesh_basechunk_simple`, but the result was discarded (`_temp_mesh`) and re-meshed on completion. Tasks now return just `(pos, BaseChunk)` → less worker-thread CPU, chunks finish sooner.

**Invariant to remember:** `MAX_CHUNK_COMPLETIONS_PER_FRAME` must be ≥ `MAX_CHUNKS_PER_FRAME`, or chunks generate faster than they're meshed and the terrain fills with holes.

**Proposed perf round 2 (not done — needs buy-in, more invasive):**
- **Heightmap cache across vertical chunks.** The documented main bottleneck: each XZ column generates 5 Y-level chunks, and each recomputes the *same* 2D heightmap (~4k noise samples). Caching it per (cx,cz) cuts ~80% of generation noise. Requires a shared concurrent cache (`Arc<Mutex<HashMap<IVec2, Arc<Vec<f32>>>>>` as a Resource, cloned into async tasks, cleared in `teardown_world`) threaded through `generate_terrain`. Moderate refactor.
- **Limit physics colliders to a near-player radius** (colliders are built for all real chunks ≤32; player only touches nearby terrain). Cuts trimesh-build cost but changes where the player can collide — gameplay decision.

## Remaining / follow-ups (next session)

- **[teardown] Minor leftovers not cleaned on Quit-to-Menu:** in-flight voxel drops (`RapierVoxelDrop`) and `VoxelBreaking` markers aren't despawned by `teardown_world`. Harmless (markers are state-gated, drops are rare) but worth adding for completeness.
- **[seed] Same map on replay within a launch.** Seed is randomized once per launch (your chosen behavior). If you later want a fresh map each time you press Play, re-randomize `WorldSeed` in `teardown_world` or on the MainMenu→InGame transition.

- **[3b] Chunk-seam re-meshing on break (VERIFY IN-GAME).** Breaking a voxel re-meshes only the owning chunk (`destruction.rs:433`). If a broken voxel sits on a chunk boundary, the neighbor chunk's mesh/collider isn't refreshed → possible visual gap or stale face at seams. Fix: when `local_pos` is on a face (0 or 31), also re-mesh the face-neighbor chunk(s). Confirm whether this is still visible after the material fix before investing.
- **[3c] Multi-voxel destruction pattern clips at chunk borders** (`destruction.rs:372-380`): `local_pos+offset` cast to `usize` underflows across chunk boundaries → those voxels silently skipped. Make crater patterns cross chunk boundaries.
- **[perf] `TerrainGenerator::new` rebuilt per chunk** (`dynamic_chunks.rs:43`) — builds several FastNoiseLite each time. Consider caching/sharing per seed.
- **[appearance] `from_density` now only used by tests** — can retire once nothing else needs it.
- **[map mode] No hard wall at the world boundary** — player can walk into the void past ±128 chunks. Add a barrier/teleport-back if desired.

## Deferred
- **Issue #4** — "The Finals"-style main menu (player center, corner stats, game-mode select, animated background).

## Verification done
- `cargo build` clean (only pre-existing warnings).
- `cargo test --lib` → 15 passing (incl. 4 new `from_depth` tests).
- In-game manual test still pending (cursor/pause feel, terrain look, digging in valleys).

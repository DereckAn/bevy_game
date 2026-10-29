# Plan: Enemies (horde of 4k, types, abilities, elites, procedural bosses)

## Goal
A Megabonk-style horde: about 4,000 enemies on screen that pile up on each other. Many enemy types
with their own identities and abilities, plus elites, plus procedural bosses that take abilities
from the bosses the player has already defeated.

## Where we are
- `src/enemy/mod.rs`: 5 purple boxes spawned in a 5 m ring. Each one is a full Rapier dynamic body
  that chases the player. Ground and tree collision come for free from the chunk colliders.
- The player is a Rapier body (`PlayerController`) with the camera as a child.
- **No combat exists.** Nothing has health, and the player cannot hurt anything. "Defeating a boss"
  needs this first.
- `SpatialHashGrid` (`src/voxel/spatial_hash.rs`) indexes **chunks**, not entities. The horde needs
  its own grid of enemies.
- The game already runs at 30–45 FPS before any enemies. Measure FPS at the end of every phase with
  `cargo run --release`.

## The rule that keeps the easy phases from being thrown away
Phases 1–6 run on Rapier bodies, and Phase 7 replaces Rapier movement for the horde. To make sure
types, abilities and elites survive that swap:
- **Abilities never write Rapier's `Velocity` directly.** They write a `DesiredVelocity`
  (or a speed multiplier). Only one system turns that into actual motion. In Phase 7 only that
  one system changes.
- Stats and abilities are plain components and data. They don't know how the enemy moves.

## Phases (easiest → hardest)

### Phase 1: Enemy types as data (easy)
- An `EnemyKind` enum (`Grunt`, `Small`, `Big`, `Fast`, `Slow`…) plus one stats table:
  size, speed, color, HP and contact damage.
- One shared mesh, scaled by size, and one material per kind. Spawning picks a kind and reads the table.
- Add the `DesiredVelocity` rule from above here, while the code is still tiny.
- **Done when:** 5 kinds spawn at the same time, each visibly different in size, color and speed.

### Phase 2: Continuous spawner (easy)
- Spawn waves over time in a ring around the player, instead of all at once at startup.
- Only spawn where the chunk below has already loaded. Check `ChunkMap` for the chunk; otherwise the
  enemy falls through the world. This removes today's 5 m spawn limit.
- Raise the spawn rate with elapsed time. Keep a cap (`MAX_ENEMIES`).
- Despawn enemies that end up too far away, so the count doesn't creep upward.
- **Done when:** enemies keep arriving from all sides at 20–40 m, and none fall through the ground.

### Phase 3: Health, damage, death (easy–medium)
- `Health` on the player and on enemies. Enemies that touch the player deal contact damage, with a
  cooldown.
- The player needs some way to hurt enemies: see **Open questions**. Until that's decided, use a
  placeholder auto-attack on the nearest enemy within N m every X seconds.
- Death = despawn. Basic HUD: player HP and kill count.
- **Done when:** you can die, and you can kill enemies.

### Phase 4: Abilities as components (medium)
- One component plus one system per ability. Start with two:
  - `Explodes { radius, damage, fuse }`: arms near the player, then damages everything in the radius.
  - `Dashes { cooldown, burst }`: a short burst of `DesiredVelocity` toward the player.
- An enemy kind's table entry lists which abilities it spawns with. Combining abilities needs no
  new code.
- **Done when:** an "explosive + fast" enemy works with zero code specific to that combination.

### Phase 5: Elites (easy once Phase 4 exists)
- `Elite` marker: stats multiplied, slightly bigger, a visible tint or outline, and 1–2 extra random
  abilities from the pool.
- A small chance to spawn as an elite, rising over time.
- **Done when:** elites show up occasionally and are clearly tougher and recognizable.

### Phase 6: Procedural bosses with an inherited ability pool (medium–hard)
- A boss is a big base body plus a list of abilities, scaled by player progress. Bosses stay on
  Rapier: there are few of them, so real collision is affordable.
- `DefeatedBossAbilities` resource: when a boss dies, its abilities are added to the pool.
  The next boss = a few from the pool + one new ability, with the numbers scaled up.
- Boss-only abilities (summons, slams, shields…) get added here, one at a time.
- Saving the pool across game sessions (serde to disk) is a separate, later step.
- **Done when:** boss #2 visibly reuses an ability from boss #1, plus one it didn't have.

### Phase 7: Horde movement without Rapier (hard)
- Regular enemies drop `RigidBody`/`Collider`. One movement system reads `DesiredVelocity`,
  applies a simple gravity and snaps to the ground.
- **Ground and trees:** read the actual voxels through `ChunkMap` (as `src/player/swim.rs` does).
  Not `generate_height`: that returns the original terrain, so it doesn't know about broken
  voxels or trees. An enemy blocked by a solid voxel slides around it.
- Hitting the player becomes a distance check, not a physics contact.
- **Done when:** behavior matches Phase 6 (ground, trees, chasing), but enemies have no Rapier body.

### Phase 8: Separation and piling (hard)
- An enemy spatial hash, rebuilt every frame: cell → enemy list.
- Separation: each enemy pushes away from neighbors that overlap it.
- Piling: a crowded enemy that cannot move forward climbs on top of the one in front, so the horde
  forms waves and towers like in Megabonk. Tune the climb height and how quickly piles collapse.
- **Done when:** 500 enemies pile up on the player without passing through each other, and it looks right.

### Phase 9: Scale to 4k (hardest)
- Profile first. Likely fixes, in order:
  1. Parallel queries (`par_iter_mut`) for movement and separation.
  2. Far enemies (beyond about 40 m) update less often, or move without separation.
  3. Bevy automatic instancing should already batch the meshes; check how many draw calls there are.
  4. Enable Rapier's `parallel` feature for the remaining bodies: player, elites, bosses.
- **Done when:** 4,000 enemies at ≥ 30 FPS in a release build, with the horde piled on the player.

## Open questions (decide before the phase that needs them)
1. **How does the player attack?** Megabonk-style auto-attack weapons, or aimed tools such as the
   shovel and pickaxe that already exist? (Phase 3)
2. **The door:** "don't spawn behind the door". There are no doors yet; buildings are only planned
   in `docs/buildings_and_cities_plan.md`. Does this mean building interiors, or out of the
   player's view? (Phase 2)
3. **Can enemies break voxels?** For example explosive enemies cratering the terrain. That would
   interact with the "seed + diffs" editing model. (Phase 4)
4. **Do bosses carry over between game sessions**, or does the pool reset every run? (Phase 6)

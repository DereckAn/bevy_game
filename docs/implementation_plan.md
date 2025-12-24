# Plan de Implementación - Extraction Shooter Voxel Postapocalíptico

Juego de extracción multijugador en mundo abierto con voxels destructibles, inspirado en Call of Mini, Lay of the Land, Helldivers, Minecraft, Arc Raiders y The Finals.

## User Review Required

> [!IMPORTANT]
> **Decisiones Arquitectónicas Críticas**
> 
> 1. **Voxel Engine**: Usar chunks de 32³ con Surface Nets para terreno suave (ya implementado en Fase 1)
> 2. **Networking**: `lightyear` para servidor autoritativo con predicción del cliente
> 3. **Inventario**: 256 slots (2⁸) para balance entre capacidad y rendimiento
> 4. **Target Inicial**: 150-500 enemigos simultáneos con spatial hashing
> 5. **Mundo Inicial**: Mapa fijo de ~1km² para MVP, procedural en fases avanzadas

> [!WARNING]
> **Desafíos Técnicos Mayores**
> 
> - **Voxel Destruction en Multiplayer**: Sincronizar destrucción voxel-por-voxel entre 8 jugadores requiere delta compression agresiva
> - **Rendimiento con 500+ Enemigos**: Necesitaremos GPU instancing + LOD AI desde fase temprana
> - **Edificios Colapsables**: Física de voxels caídos puede ser costosa, considerar simplificación (desaparecer voxels en lugar de simular caída)

---

## Proposed Changes

### Core Systems Architecture

#### Voxel System (`src/voxel/`)

**[MODIFY]** [mod.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/mod.rs)
- Agregar `VoxelType` enum (Dirt, Stone, Wood, Metal, etc.) con resistencias
- Implementar sistema de "voxel groups" para optimizar drops (10-30 voxels por golpe)

**[NEW]** [voxel_types.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/voxel_types.rs)
```rust
pub enum VoxelType {
    Air,
    Dirt { hardness: f32 },
    Stone { hardness: f32 },
    Wood { hardness: f32 },
    Metal { hardness: f32 },
    // ... más tipos
}

pub struct VoxelDrop {
    voxel_type: VoxelType,
    count: u32, // 10-30 para árboles, 1-5 para piedra
}
```

**[NEW]** [destruction.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/destruction.rs)
- Sistema de destrucción con herramientas específicas
- Cálculo de "golpe efectivo" para determinar cantidad de drops
- Explosiones que destruyen grupos de voxels
- Sistema de colapso de edificios (marcar voxels para eliminación en lugar de física completa)
- **Raycast optimizado:**
  - Fase 2 MVP: Raycast punto-por-punto (simple, funcional)
  - Fase 2.5: Algoritmo DDA (10x más rápido, estilo Minecraft)
  - ChunkMap con HashMap para acceso O(1) a chunks
  - Separar raycast UI (cada frame, 2m) vs destrucción (al click, 5m)
  - Cache de último voxel mirado para evitar re-renderizado de outline

**[NEW]** [drops.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/drops.rs)
- Pool de entidades para drops de voxels
- Auto-recolección después de X segundos
- Límite de drops simultáneos para rendimiento

---

#### Inventory System (`src/inventory/`)

**[NEW]** [mod.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/inventory/mod.rs)
```rust
pub const INVENTORY_SIZE: usize = 256; // 2^8 slots

pub struct Inventory {
    slots: [Option<ItemStack>; INVENTORY_SIZE],
}

pub struct ItemStack {
    item_type: ItemType,
    count: u32,
    max_stack: u32, // Voxels: 999, Armas: 1
}

pub enum ItemType {
    Voxel(VoxelType),
    Tool(ToolType),
    Weapon(WeaponType),
    Ammo(AmmoType),
}
```

**[NEW]** [crafting.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/inventory/crafting.rs)
- Sistema de crafting para herramientas (pico, pala, hacha)
- Recetas para municiones
- Sistema de niveles para desbloquear recetas

---

#### Combat System (`src/combat/`)

**[NEW]** [weapons.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/combat/weapons.rs)
```rust
pub enum WeaponType {
    Melee { damage: f32, range: f32 },
    Pistol { damage: f32, fire_rate: f32, ammo_type: AmmoType },
    Rifle { damage: f32, fire_rate: f32, ammo_type: AmmoType },
    Bow { damage: f32, arrow_speed: f32 },
    Grenade { damage: f32, radius: f32 },
}
```

**[NEW]** [damage.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/combat/damage.rs)
- Sistema de daño por zona (headshot, body, limbs)
- Puntos débiles específicos por enemigo
- Fuego amigo habilitado

**[NEW]** [projectiles.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/combat/projectiles.rs)
- Object pooling para proyectiles (balas, flechas, granadas)
- Física de proyectiles con `bevy_rapier3d`
- Explosiones que destruyen voxels

---

#### Enemy System (`src/enemy/`)

**[NEW]** [enemy_types.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/enemy/enemy_types.rs)
```rust
pub enum RobotZombieType {
    Basic { speed: f32, health: f32, damage: f32 },
    Fast { speed: f32, health: f32, damage: f32 },
    Tank { speed: f32, health: f32, damage: f32 },
    Ranged { speed: f32, health: f32, damage: f32, weapon: WeaponType },
    Explosive { speed: f32, health: f32, explosion_radius: f32 },
}

pub struct WeakPoint {
    location: BodyPart, // Head, Chest, Back, etc.
    damage_multiplier: f32,
}
```

**[NEW]** [spawning.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/enemy/spawning.rs)
- Spawn dinámico: 150 base + 50 por jugador adicional
- Spawn waves durante partida
- Despawn de enemigos lejanos para mantener límite

**[NEW]** [ai.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/enemy/ai.rs)
- Pathfinding jerárquico con `oxidized_navigation` o custom A*
- Spatial hashing para detección de jugadores
- LOD AI: enemigos lejanos usan AI simplificada

---

#### World Generation (`src/world/`)

**[NEW]** [biomes.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/biomes.rs)
```rust
pub enum Biome {
    Forest,
    Desert,
    Snow,
    City,
    Wasteland,
}

pub struct BiomeConfig {
    tree_density: f32,
    building_density: f32,
    enemy_spawn_rate: f32,
}
```

**[NEW]** [weather.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/weather.rs)
- Ciclo día/noche
- Clima dinámico (lluvia, nieve, niebla)
- **Tormenta de Radiación**: evento que regenera el mapa y fuerza a jugadores a refugiarse

**[NEW]** [procedural.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/procedural.rs)
- Generación procedural con noise (FastNoise2 o `noise-rs`)
- Montañas, ríos, lagos con voxels
- Edificios procedurales destructibles
- Árboles y vegetación

**[NEW]** [water.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/water.rs)
- Agua estática (no fluye)
- Sistema de natación
- Sistema de ahogamiento (daño por tiempo bajo agua)

---

#### Networking (`src/networking/`)

**[NEW]** [mod.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/networking/mod.rs)
- Integración con `lightyear`
- Servidor autoritativo
- Client-side prediction para movimiento

**[NEW]** [voxel_sync.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/networking/voxel_sync.rs)
- Delta compression para cambios de voxels
- Batch updates (enviar cambios cada 100ms en lugar de inmediato)
- Interest management: solo sincronizar chunks cercanos a jugadores

**[NEW]** [entity_sync.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/networking/entity_sync.rs)
- Sincronización de jugadores (posición, rotación, animaciones)
- Sincronización de enemigos (servidor autoritativo)
- Sincronización de proyectiles

**[NEW]** [session.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/networking/session.rs)
- Sesiones de extracción (partidas largas)
- Regeneración de mundo con tormenta de radiación
- Persistencia de inventario de jugador entre sesiones

---

#### Progression System (`src/progression/`)

**[NEW]** [experience.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/progression/experience.rs)
```rust
pub struct PlayerLevel {
    level: u32,
    xp: u32,
    xp_to_next: u32,
}

pub struct UnlockSystem {
    unlocked_recipes: HashSet<RecipeId>,
    unlocked_weapons: HashSet<WeaponId>,
}
```

**[NEW]** [skills.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/progression/skills.rs)
- Árbol de habilidades
- Habilidades pasivas (más daño, más velocidad, más resistencia)
- Habilidades activas (dash, escudo temporal, etc.)

**[NEW]** [loot.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/progression/loot.rs)
- Drops de enemigos (armas, munición, recursos)
- Drops de jugadores muertos (PvP)
- Sistema de rareza

---

#### UI System (`src/ui/`)

**[NEW]** [hud.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/ui/hud.rs)
- Vida, stamina, munición
- Indicador de nivel y XP
- Marcadores para jugadores aliados
- Crosshair dinámico

**[NEW]** [inventory_ui.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/ui/inventory_ui.rs)
- Interfaz de inventario (256 slots)
- Drag & drop de items
- Crafting UI
- Juego continúa mientras está abierto (no pausa)

**[NEW]** [missions.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/ui/missions.rs)
- Sistema de misiones/objetivos
- Tracker de progreso

---

#### Audio System (`src/audio/`)

**[NEW]** [mod.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/audio/mod.rs)
- Integración con `bevy_kira_audio` o `bevy_oddio`
- Música dinámica (cambia según situación: combate, exploración, tormenta)
- Sonidos posicionales 3D
- Efectos de sonido por arma/enemigo

**[NEW]** [voice_chat.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/audio/voice_chat.rs)
- Integración con `discord-sdk` o `vivox`
- Chat de voz posicional (más fuerte si está cerca)

---

## Verification Plan

### Automated Tests

**Performance Benchmarks**:
```bash
cargo bench --bench voxel_destruction
cargo bench --bench enemy_spawning
cargo bench --bench networking_sync
```

**Integration Tests**:
```bash
cargo test --test voxel_system
cargo test --test combat_system
cargo test --test multiplayer_sync
```

### Manual Verification

**Fase por Fase**:
1. **Fase 1 (Completada)**: ✅ Movimiento, chunks, terreno básico
2. **Fase 2**: Destrucción de voxels, drops, inventario básico
3. **Fase 3**: Enemigos básicos (150), pathfinding, combate melee
4. **Fase 4**: Armas a distancia, munición, crafting
5. **Fase 5**: Multijugador 8 jugadores, sincronización
6. **Fase 6**: Mundo procedural, biomas, clima
7. **Fase 7**: Progresión, niveles, habilidades
8. **Fase 8**: Polish, optimización final

**Métricas de Éxito**:
- ✅ 60 FPS con 150 enemigos (8 jugadores)
- ✅ <100ms latencia en multijugador
- ✅ Destrucción de voxels sincronizada sin lag
- ✅ Edificios colapsan sin crash
- ✅ Inventario 256 slots funcional

---

## Crates Recomendados

### Core
- ✅ `bevy` (0.12+)
- ✅ `bevy_rapier3d` - Física

### Voxel
- `block-mesh` - Meshing optimizado
- `noise` - Generación procedural
- `ndshape` - Indexing eficiente de chunks

### Networking
- `lightyear` - Networking para Bevy
- `serde` - Serialización

### AI
- `oxidized_navigation` - Navmesh + pathfinding
- `big-brain` - Behavior trees (opcional)

### Audio
- `bevy_kira_audio` - Audio engine
- `oddio` - Audio 3D posicional

### Optimización
- `rayon` - Paralelización
- `bevy_mod_picking` - Picking optimizado
- `bevy_framepace` - Control de framerate

### UI
- `bevy_egui` - UI debugging
- `bevy_ui` - UI nativa de Bevy

---

## Próximos Pasos

1. ✅ Revisar y aprobar este plan
2. Actualizar `plan.md` con nueva información
3. Crear estructura de carpetas detallada
4. Implementar Fase 2: Destrucción de voxels + inventario
5. Iterar según feedback

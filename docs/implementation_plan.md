# Plan de Implementación - Extraction Shooter Voxel Multijugador

Juego de extracción multijugador en mundos de misión procedurales con bases subterráneas persistentes, edificios de hasta 20 pisos, voxels de 10cm de detalle, y arquitectura de streaming para mundos masivos.

## Visión Completa del Juego

**Extraction Shooter Voxel Multijugador** inspirado en Call of Mini, Lay of the Land, Helldivers, Minecraft, Arc Raiders y The Finals.

### Características Principales
- **Mundos de Misión**: Generados proceduralmente basados en biomas del overworld (volcán, nieve, bosque, desierto, ciudad)
- **Bases Subterráneas**: Persistentes, expandibles con construcción voxel, comercio y cultivo
- **Edificios Masivos**: Hasta 20 pisos + sótanos profundos (2048 voxels de altura total)
- **Invasiones de Bases**: Sistema PvP/PvE opcional para atacar/defender bases
- **Voxels Detallados**: 10cm de resolución para máximo detalle
- **Greedy Meshing**: Reducción de 70% en triángulos para rendimiento óptimo
- **Dual Contouring**: Terreno suave que se combina con estructuras voxel
- **Streaming Dinámico**: Carga/descarga de mundos sin tiempos de espera
- **Multijugador Fundamental**: PC primero, consolas después

## User Review Required

> [!IMPORTANT]
> **Decisiones Arquitectónicas Críticas Actualizadas**
> 
> 1. **Voxel Engine**: Chunks de 128×2048×128 con greedy meshing y dual contouring
> 2. **Altura de Mundo**: 2048 voxels (204.8m) para edificios de 20 pisos + minas profundas
> 3. **Networking**: `lightyear` para servidor autoritativo con sincronización multi-mundo
> 4. **Inventario**: 256 slots persistente entre mundos
> 5. **Target**: 500+ enemigos con spatial hashing por mundo
> 6. **Arquitectura de Mundos**: Streaming dinámico con presupuesto de memoria de 4GB

> [!WARNING]
> **Desafíos Técnicos Mayores Actualizados**
> 
> - **Chunks 2048 de Altura**: Heap allocation obligatoria para prevenir stack overflow
> - **Greedy Meshing**: Crítico para rendimiento con voxels de 10cm - target 70% reducción de triángulos
> - **Streaming Multi-Mundo**: Sincronizar destrucción voxel entre mundos de misión y bases
> - **Memory Management**: 4GB budget para múltiples mundos cargados simultáneamente
> - **Dual Contouring**: Terreno suave + estructuras voxel requiere algoritmo híbrido

---

## Proposed Changes

### Core Systems Architecture

#### Voxel System (`src/voxel/`) - UPDATED

**[MODIFY]** [mod.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/mod.rs)
- Agregar `VoxelType` enum (Dirt, Stone, Wood, Metal, etc.) con resistencias
- Implementar sistema de "voxel groups" para optimizar drops (10-30 voxels por golpe)
- Integrar greedy meshing y dual contouring

**[MODIFY]** [voxel_types.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/voxel_types.rs)
```rust
pub enum VoxelType {
    Air,
    Dirt { hardness: f32 },
    Stone { hardness: f32 },
    Wood { hardness: f32 },
    Metal { hardness: f32 },
    // ... más tipos para diferentes biomas
}

pub struct VoxelDrop {
    voxel_type: VoxelType,
    count: u32, // 10-30 para árboles, 1-5 para piedra
    world_id: WorldId, // Nuevo: tracking de mundo
}
```

**[MODIFY]** [destruction.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/destruction.rs)
- Sistema de destrucción con herramientas específicas
- Cálculo de "golpe efectivo" para determinar cantidad de drops
- Explosiones que destruyen grupos de voxels
- Sistema de colapso de edificios (marcar voxels para eliminación)
- **Raycast optimizado DDA** (ya implementado)
- **Soporte para chunks 2048 de altura**

**[NEW]** [greedy_meshing.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/greedy_meshing.rs)
```rust
pub struct GreedyMesher {
    chunk_data: &[VoxelType],
    dimensions: IVec3, // 128×2048×128
}

impl GreedyMesher {
    pub fn generate_mesh(&self) -> OptimizedChunkMesh {
        // Target: 70% reducción de triángulos
        // <50ms por chunk de 128×2048×128
    }
}
```

**[NEW]** [dual_contouring.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/dual_contouring.rs)
```rust
pub struct DualContouringMesher {
    density_field: Box<[f32; 129 * 129 * 129]>,
}

impl DualContouringMesher {
    pub fn generate_terrain_mesh(&self) -> TerrainMesh {
        // Terreno suave que se combina con estructuras voxel
        // <100ms por chunk de generación
    }
}
```

**[MODIFY]** [drops.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/voxel/drops.rs)
- Pool de entidades para drops de voxels
- Auto-recolección después de X segundos
- Límite de drops simultáneos para rendimiento
- **Soporte multi-mundo**: drops persisten en mundo original durante teleportación
- **Ground detection para dual contouring terrain**

---

---

#### World Management System (`src/world/`) - NEW

**[NEW]** [mod.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/mod.rs)
```rust
pub mod mission_generator;
pub mod underground_base;
pub mod streaming;
pub mod teleportation;
pub mod overworld;

pub use mission_generator::*;
pub use underground_base::*;
pub use streaming::*;
pub use teleportation::*;
pub use overworld::*;
```

**[NEW]** [mission_generator.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/mission_generator.rs)
```rust
pub struct MissionWorldGenerator {
    noise_generator: FastNoise,
    biome_configs: HashMap<BiomeType, BiomeConfig>,
}

pub struct BiomeConfig {
    // Terrain generation
    height_scale: f32,
    roughness: f32,
    
    // Structure generation
    building_density: f32,
    tree_density: f32,
    
    // Mission parameters
    mission_types: Vec<MissionType>,
    enemy_spawn_rate: f32,
}

pub enum BiomeType {
    Volcano, // lava, ash, volcanic structures
    Snow,    // snow, ice, cold-weather structures
    Forest,  // dense trees, wooden structures
    Desert,  // sand, cacti, ruins
    City,    // concrete buildings, urban structures
}
```

**[NEW]** [underground_base.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/underground_base.rs)
```rust
pub struct UndergroundBase {
    owner: PlayerId,
    world_data: Box<[VoxelType; 64 * 64 * 64]>, // Smaller than mission worlds
    modifications: HashMap<IVec3, VoxelType>, // Player changes
    facilities: Vec<BaseFacility>,
    invasion_status: InvasionStatus,
}

pub enum BaseFacility {
    TradingPost { inventory: Inventory, orders: Vec<TradeOrder> },
    Farm { crop_type: CropType, growth_stage: f32 },
    Storage { capacity: usize, stored_items: Inventory },
    Defense { turret_type: TurretType, ammo: u32 },
}

pub enum InvasionStatus {
    Safe,
    UnderAttack { attackers: Vec<Entity>, start_time: f64 },
    Breached { breach_points: Vec<IVec3>, loot_stolen: Vec<ItemStack> },
}
```

**[NEW]** [streaming.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/streaming.rs)
```rust
pub struct WorldStreamingSystem {
    active_worlds: HashMap<WorldId, LoadedWorld>,
    loading_worlds: HashMap<WorldId, LoadingTask>,
    memory_budget: usize, // 4GB total
    current_memory_usage: usize,
}

pub struct LoadedWorld {
    world_data: WorldData,
    chunks: HashMap<IVec3, VoxelChunk>,
    entities: Vec<Entity>,
    last_accessed: Instant,
    memory_usage: usize,
}

// Target: <5 seconds world load, <4GB total memory
```

**[NEW]** [teleportation.rs](file:///c:/Users/derec/Documents/Git/bevy_game/src/world/teleportation.rs)
```rust
pub struct TeleportSystem {
    teleport_points: HashMap<WorldId, Vec<TeleportPoint>>,
    active_teleports: Vec<ActiveTeleport>,
}

pub enum TeleportType {
    MissionToBase,
    BaseToMission,
    Emergency, // Durante invasiones
    Overworld,
}

// Target: <1 second teleportation time
```

---

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
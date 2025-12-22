## Plan de Desarrollo - Fase por Fase

### Fase 1: Fundamentos (2-4 semanas)

- [x] Setup proyecto Bevy
- [x] Sistema de chunks voxel básico (32³, ~0.1m por voxel)
- [x] Terreno simple con Surface Nets (suave, no blocky)
- [x] Cámara primera persona
- [x] Movimiento del jugador (WASD + salto)
- [x] Física básica con `bevy_rapier3d`

### Fase 1.5: Optimización Fundacional (1-2 semanas) 🚀

- [ ] **Profiling setup**: `tracy`, `puffin`, o `bevy_inspector_egui`
- [ ] **Chunk LOD system**: Diferentes niveles de detalle por distancia
- [ ] **Frustum culling**: Solo renderizar chunks visibles
- [ ] **Occlusion culling**: No renderizar chunks ocultos
- [ ] **Chunk pooling**: Reutilizar memoria de chunks
- [ ] **Async chunk generation**: Generar terreno en threads separados
- [ ] **Mesh optimization**: Reducir vértices redundantes

### Fase 2: Combate Básico (2-3 semanas)

- [ ] Sistema de armas melee (espada/hacha)
- [ ] Animación de ataque
- [ ] Sistema de vida (jugador)
- [ ] HUD básico (vida, stamina)

### Fase 2.5: Optimización de Combate (1 semana) ⚡

- [ ] **Object pooling**: Pool de proyectiles/efectos
- [ ] **Spatial partitioning**: Quadtree/Octree para colisiones
- [ ] **Batch rendering**: Agrupar draws de efectos similares
- [ ] **Animation compression**: Optimizar datos de animación

### Fase 3: Zombies (3-4 semanas)

- [ ] Spawn de zombies básico
- [ ] AI pathfinding hacia jugador
- [ ] Ataque melee de zombies
- [ ] Sistema de vida/muerte de zombies
- [ ] Spatial hashing para 800+ entidades
- [ ] Spawner constante de zombies

### Fase 3.5: Optimización Masiva de Entidades (2 semanas) 🔥

- [ ] **ECS optimization**: Componentes densos, queries eficientes
- [ ] **Hierarchical pathfinding**: A\* jerárquico para 1000+ zombies
- [ ] **Behavior trees pooling**: Reutilizar árboles de comportamiento
- [ ] **GPU instancing**: Renderizar 1000+ zombies con instancing
- [ ] **Level-of-detail AI**: AI simple para zombies lejanos
- [ ] **Temporal load balancing**: Distribuir AI updates en frames
- [ ] **Memory-mapped entities**: Entidades en memoria contigua

### Fase 4: Mundo (2-3 semanas)

- [ ] Ciclo día/noche
- [ ] Iluminación dinámica
- [ ] Terreno destructible (zombies y jugador)
- [ ] Re-meshing de chunks modificados

### Fase 4.5: Optimización de Mundo (1-2 semanas) 🌍

- [ ] **Lighting optimization**: Shadow cascades, light culling
- [ ] **Texture streaming**: Cargar texturas bajo demanda
- [ ] **Procedural generation caching**: Cache de generación procedural
- [ ] **Incremental mesh updates**: Solo actualizar partes modificadas
- [ ] **GPU-driven rendering**: Culling y rendering en GPU

### Fase 5: Multijugador (4-6 semanas)

- [ ] Setup `lightyear` para networking
- [ ] Sincronización de jugadores
- [ ] Sincronización de zombies (server authoritative)
- [ ] Fuego amigo
- [ ] PvP básico

### Fase 5.5: Optimización de Red (2 semanas) 📡

- [ ] **Delta compression**: Solo enviar cambios
- [ ] **Prediction/rollback**: Client-side prediction
- [ ] **Interest management**: Solo sincronizar entidades relevantes
- [ ] **Bandwidth optimization**: Compresión de datos de red
- [ ] **Connection pooling**: Reutilizar conexiones

### Fase 6: Polish + Optimización Final (ongoing) ✨

#### Rendering Extremo:

- [ ] **GPU-driven culling**: Frustum + occlusion culling en GPU
- [ ] **Mesh shaders**: Geometry generation en GPU (si disponible)
- [ ] **Variable rate shading**: Menos shading en periféricos
- [ ] **Temporal upsampling**: Renderizar a menor resolución + upscale
- [ ] **Custom allocators**: Allocators específicos por sistema

#### CPU Extremo:

- [ ] **SIMD optimization**: Vectorización manual crítica
- [ ] **Cache-friendly data**: Estructuras optimizadas para cache
- [ ] **Lock-free algorithms**: Evitar mutex en hot paths
- [ ] **Custom ECS scheduler**: Scheduler optimizado para nuestro caso
- [ ] **Profile-guided optimization**: PGO compilation

#### Memory Extremo:

- [ ] **Memory budgets**: Límites estrictos por sistema
- [ ] **Custom memory pools**: Pools especializados
- [ ] **Compression everywhere**: Comprimir assets, saves, etc.
- [ ] **Memory defragmentation**: Compactar memoria periódicamente

#### Targets de Rendimiento Obsesivos:

- [ ] **60 FPS mínimo** con 2000+ zombies
- [ ] **<16ms frame time** en 99% de frames
- [ ] **<100MB RAM** para chunks activos
- [ ] **<1ms** tiempo de spawn de zombie
- [ ] **<50ms** tiempo de generación de chunk
- [ ] **<10MB/s** bandwidth en multijugador

---

### 2. **Modularización por Feature**

Cada feature en su carpeta:

```
src/
├── main.rs
├── voxel/        # Feature: mundo voxel
├── player/       # Feature: jugador
├── enemy/        # Feature: enemigos
├── combat/       # Feature: combate
├── networking/   # Feature: multijugador
└── ui/           # Feature: interfaz
```

## Estructura Profesional Recomendada

```
src/
├── main.rs                 # Solo inicializa App + plugins
├── lib.rs                  # Exporta todo (para tests)
├── core/                   # Recursos compartidos
│   ├── mod.rs
│   ├── constants.rs        # CHUNK_SIZE, VOXEL_SIZE, etc.
│   ├── resources.rs        # GameState, Settings
│   └── events.rs           # Eventos globales
├── voxel/
│   ├── mod.rs              # VoxelPlugin
│   ├── chunk.rs            # Component + datos
│   ├── meshing.rs          # Sistema de mesh
│   ├── generation.rs       # Generación procedural
│   └── destruction.rs      # Destrucción de terreno
├── player/
│   ├── mod.rs              # PlayerPlugin
│   ├── components.rs       # Player, Inventory, etc.
│   ├── movement.rs         # Sistema movimiento
│   ├── camera.rs           # Sistema cámara FPS
│   └── input.rs            # Manejo de input
├── enemy/
│   ├── mod.rs              # EnemyPlugin
│   ├── components.rs       # Zombie, Health, AI
│   ├── spawning.rs         # Sistema spawn
│   ├── ai.rs               # Pathfinding, comportamiento
│   └── combat.rs           # Ataque al jugador
├── combat/
│   ├── mod.rs
│   ├── damage.rs           # Sistema de daño
│   ├── weapons.rs          # Armas
│   └── hitbox.rs           # Detección de colisiones
├── networking/
│   ├── mod.rs
│   ├── client.rs
│   ├── server.rs
│   └── sync.rs             # Sincronización de entidades
└── ui/
    ├── mod.rs
    ├── hud.rs              # Vida, stamina
    └── menu.rs             # Menú principal
```

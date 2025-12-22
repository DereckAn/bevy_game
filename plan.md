## Plan de Desarrollo - Fase por Fase

### Fase 1: Fundamentos (2-4 semanas)

- [x] Setup proyecto Bevy
- [x] Sistema de chunks voxel básico (32³, ~0.1m por voxel)
- [x] Terreno simple con Surface Nets (suave, no blocky)
- [x] Cámara primera persona
- [x] Movimiento del jugador (WASD + salto)
- [x] Física básica con `bevy_rapier3d`

### Fase 2: Combate Básico (2-3 semanas)

- [ ] Sistema de armas melee (espada/hacha)
- [ ] Animación de ataque
- [ ] Sistema de vida (jugador)
- [ ] HUD básico (vida, stamina)

### Fase 3: Zombies (3-4 semanas)

- [ ] Spawn de zombies básico
- [ ] AI pathfinding hacia jugador
- [ ] Ataque melee de zombies
- [ ] Sistema de vida/muerte de zombies
- [ ] Spatial hashing para 800+ entidades
- [ ] Spawner constante de zombies

### Fase 4: Mundo (2-3 semanas)

- [ ] Ciclo día/noche
- [ ] Iluminación dinámica
- [ ] Terreno destructible (zombies y jugador)
- [ ] Re-meshing de chunks modificados

### Fase 5: Multijugador (4-6 semanas)

- [ ] Setup `lightyear` para networking
- [ ] Sincronización de jugadores
- [ ] Sincronización de zombies (server authoritative)
- [ ] Fuego amigo
- [ ] PvP básico

### Fase 6: Polish (ongoing)

- [ ] Armas a distancia
- [ ] Zombies a distancia
- [ ] Sistema de oleadas
- [ ] Tercera persona
- [ ] Crafting
- [ ] Más tipos de zombies

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

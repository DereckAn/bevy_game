## Plan de Desarrollo - Fase por Fase

### Fase 1: Fundamentos (2-4 semanas)
- [x] Setup proyecto Bevy
- [x] Sistema de chunks voxel básico (32³, ~0.1m por voxel)
- [x] Terreno simple con Surface Nets (suave, no blocky)
- [x] Cámara primera persona
- [ ] Movimiento del jugador (WASD + salto)
- [ ] Física básica con `bevy_rapier3d`

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
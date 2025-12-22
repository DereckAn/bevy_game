# Roadmap Detallado - Extraction Shooter Voxel

## 🎯 MVP (Minimum Viable Product) - Fases 1-4

### ✅ Fase 1: Fundamentos (COMPLETADA)
- [x] Setup proyecto Bevy
- [x] Sistema de chunks voxel básico (32³)
- [x] Terreno con Surface Nets
- [x] Cámara primera persona
- [x] Movimiento WASD + salto
- [x] Física con `bevy_rapier3d`

---

### 🔨 Fase 2: Destrucción y Recursos (3-4 semanas)

**Objetivo**: Jugador puede destruir voxels y recolectar recursos

#### Features Core:
- [ ] **VoxelType System**
  - Tipos: Dirt, Stone, Wood, Metal
  - Propiedades: hardness, drop_rate, texture
  - Resistencia diferente por tipo

- [ ] **Herramientas Básicas**
  - Hacha (para madera)
  - Pico (para piedra/metal)
  - Pala (para tierra)
  - Sistema de durabilidad

- [ ] **Destrucción Inteligente**
  - Raycast desde cámara para detectar voxel objetivo
  - Cálculo de "golpe efectivo" basado en:
    - Herramienta correcta (+50% efectividad)
    - Ángulo de golpe
    - Durabilidad de herramienta
  - Drops variables: 10-30 voxels para árboles, 1-5 para piedra

- [ ] **Sistema de Drops**
  - Entidades físicas que caen al suelo
  - Auto-recolección al acercarse
  - Despawn después de 60 segundos
  - Pool de 500 drops máximo

- [ ] **Inventario Básico (256 slots)**
  - Estructura de datos eficiente
  - Stacking de items (999 voxels por slot)
  - UI simple para ver inventario
  - Hotbar con 10 slots rápidos

#### Optimizaciones:
- [ ] Chunk re-meshing incremental (solo actualizar chunk modificado)
- [ ] Batch de cambios de voxels (aplicar cada 100ms)
- [ ] Spatial hashing para drops

#### Tests:
- [ ] Benchmark: destruir 1000 voxels < 16ms
- [ ] Test: inventario lleno (256 slots) sin lag
- [ ] Test: 500 drops simultáneos a 60 FPS

---

### 🧟 Fase 3: Enemigos Básicos (3-4 semanas)

**Objetivo**: Robot zombies que persiguen y atacan al jugador

#### Features Core:
- [ ] **Robot Zombie Básico**
  - Modelo 3D simple (o placeholder)
  - Animaciones: idle, walk, attack, death
  - Stats: 100 HP, velocidad media, 10 damage melee

- [ ] **AI Sistema**
  - Detección de jugador (radio 30m)
  - Pathfinding básico hacia jugador
  - Evitar obstáculos
  - Ataque melee cuando está cerca (2m)

- [ ] **Spawning System**
  - Spawn inicial: 150 zombies
  - Spawn continuo: 5 zombies cada 30 segundos
  - Despawn de zombies muy lejanos (>100m)
  - Límite máximo: 500 zombies

- [ ] **Combat Básico**
  - Jugador puede golpear zombies con herramientas
  - Sistema de vida para zombies
  - Muerte de zombie → drop de recursos
  - Daño al jugador (sistema de vida)

- [ ] **HUD de Combate**
  - Barra de vida del jugador
  - Contador de enemigos cercanos
  - Indicador de daño recibido

#### Optimizaciones:
- [ ] Spatial hashing para detección de jugadores
- [ ] LOD AI: zombies lejanos (>50m) usan AI simplificada
- [ ] GPU instancing para renderizar 500+ zombies
- [ ] Temporal load balancing: distribuir AI updates en frames

#### Tests:
- [ ] 150 zombies a 60 FPS
- [ ] 500 zombies a 30 FPS mínimo
- [ ] Pathfinding < 5ms por zombie

---

### 🔫 Fase 4: Armas y Crafting (3-4 semanas)

**Objetivo**: Sistema de combate completo con armas a distancia

#### Features Core:
- [ ] **Armas Melee Mejoradas**
  - Espada, hacha de combate, pala
  - Animaciones de ataque
  - Combos básicos

- [ ] **Armas a Distancia**
  - Pistola (semiautomática)
  - Rifle (automático)
  - Arco (proyectil físico)
  - Sistema de munición

- [ ] **Sistema de Munición**
  - Tipos: balas pistola, balas rifle, flechas
  - Munición limitada
  - Recarga de armas

- [ ] **Crafting System**
  - Recetas para herramientas
  - Recetas para munición
  - UI de crafting
  - Requisitos de recursos

- [ ] **Proyectiles**
  - Física de balas (raycast instantáneo)
  - Física de flechas (proyectil con gravedad)
  - Object pooling (500 proyectiles)
  - Efectos visuales (trazas, impactos)

- [ ] **Daño por Zona**
  - Headshot: 2x daño
  - Body: 1x daño
  - Limbs: 0.5x daño

#### Optimizaciones:
- [ ] Raycast batching para balas
- [ ] Projectile pooling
- [ ] Particle system pooling

#### Tests:
- [ ] 100 balas simultáneas a 60 FPS
- [ ] Crafting de 100 items < 1ms
- [ ] Headshot detection precisa

---

## 🌐 Multijugador - Fases 5-6

### 🌍 Fase 5: Networking Básico (4-5 semanas)

**Objetivo**: 8 jugadores pueden jugar juntos

#### Features Core:
- [ ] **Setup Lightyear**
  - Servidor dedicado
  - Cliente con predicción
  - Configuración de 8 jugadores

- [ ] **Sincronización de Jugadores**
  - Posición, rotación
  - Animaciones
  - Estado de vida
  - Inventario (solo al entrar/salir)

- [ ] **Sincronización de Enemigos**
  - Servidor autoritativo
  - Posición, estado
  - Vida, muerte
  - Spawning sincronizado

- [ ] **Sincronización de Voxels**
  - Delta compression
  - Batch updates (cada 100ms)
  - Interest management (solo chunks cercanos)

- [ ] **Fuego Amigo**
  - Daño entre jugadores habilitado
  - Indicadores de equipo (marcadores)

#### Optimizaciones:
- [ ] Delta compression para voxels
- [ ] Interest management (solo sincronizar entidades cercanas)
- [ ] Bandwidth limiting (<10MB/s por jugador)

#### Tests:
- [ ] 8 jugadores sin lag (<100ms latencia)
- [ ] Destrucción de voxels sincronizada
- [ ] Combate PvP funcional

---

### ⚡ Fase 6: Optimización de Red (2-3 semanas)

**Objetivo**: Multijugador fluido y eficiente

#### Features Core:
- [ ] **Client-Side Prediction**
  - Predicción de movimiento
  - Rollback en caso de desincronización
  - Interpolación suave

- [ ] **Optimización de Bandwidth**
  - Compresión agresiva
  - Solo enviar cambios (delta)
  - Priorización de datos críticos

- [ ] **Session Management**
  - Lobby system
  - Matchmaking básico
  - Reconexión automática

#### Tests:
- [ ] <100ms latencia promedio
- [ ] <5MB/s bandwidth por jugador
- [ ] Reconexión sin pérdida de progreso

---

## 🌲 Mundo Abierto - Fases 7-9

### 🗺️ Fase 7: Mundo Procedural (4-5 semanas)

**Objetivo**: Mundo grande con biomas variados

#### Features Core:
- [ ] **Generación Procedural**
  - Noise-based terrain (FastNoise2)
  - Montañas, valles, llanuras
  - Ríos y lagos (voxels de agua)

- [ ] **Biomas**
  - Bosque (muchos árboles)
  - Desierto (arena, cactus)
  - Nieve (pinos, nieve)
  - Ciudad (edificios)
  - Wasteland (postapocalíptico)

- [ ] **Estructuras Procedurales**
  - Árboles de diferentes tamaños
  - Edificios destructibles
  - Ruinas
  - Vegetación

- [ ] **Tamaño de Mundo**
  - MVP: 1km² (fijo)
  - Futuro: Ilimitado procedural

#### Optimizaciones:
- [ ] Chunk streaming (cargar/descargar según distancia)
- [ ] Procedural generation caching
- [ ] LOD para terreno lejano

#### Tests:
- [ ] Generación de chunk < 50ms
- [ ] Transición entre biomas suave
- [ ] 1km² explorable sin lag

---

### 🌦️ Fase 8: Clima y Ambiente (2-3 semanas)

**Objetivo**: Mundo vivo con clima dinámico

#### Features Core:
- [ ] **Ciclo Día/Noche**
  - 20 minutos real = 1 día en juego
  - Iluminación dinámica
  - Skybox dinámico

- [ ] **Clima Dinámico**
  - Lluvia (reduce visibilidad)
  - Nieve (en bioma nieve)
  - Niebla (en bosque)

- [ ] **Tormenta de Radiación** 🔥
  - Evento cada 2 horas
  - Daño extremo a jugadores expuestos
  - Regenera recursos del mapa
  - Fuerza a jugadores a refugiarse

- [ ] **Sistema de Agua**
  - Agua estática (lagos, ríos)
  - Natación
  - Ahogamiento (daño después de 30s bajo agua)

- [ ] **Animales**
  - Neutrales (conejos, ciervos)
  - Hostiles (lobos, osos)
  - Drops de recursos

#### Tests:
- [ ] Ciclo día/noche sin drops de FPS
- [ ] Tormenta de radiación funcional
- [ ] Natación y ahogamiento

---

### 🏗️ Fase 9: Construcción (3-4 semanas)

**Objetivo**: Jugadores pueden construir estructuras

#### Features Core:
- [ ] **Modo Construcción**
  - Colocar voxels desde inventario
  - Preview de colocación
  - Rotación de bloques

- [ ] **Estructuras**
  - Muros defensivos
  - Refugios
  - Trampas básicas

- [ ] **Física de Colapso**
  - Edificios sin soporte colapsan
  - Simplificación: voxels desaparecen en lugar de caer
  - Drops de voxels al colapsar

#### Tests:
- [ ] Construcción fluida
- [ ] Colapso de edificios sin crash
- [ ] Sincronización de construcciones en multiplayer

---

## 📈 Progresión - Fases 10-11

### 🎖️ Fase 10: Sistema de Niveles (2-3 semanas)

**Objetivo**: Progresión del jugador

#### Features Core:
- [ ] **Experiencia**
  - XP por matar enemigos
  - XP por recolectar recursos
  - XP por completar misiones
  - Sistema de niveles (1-100)

- [ ] **Habilidades**
  - Árbol de habilidades
  - Pasivas: +daño, +velocidad, +vida
  - Activas: dash, escudo, etc.
  - Puntos de habilidad por nivel

- [ ] **Desbloqueos**
  - Recetas de crafting por nivel
  - Armas por nivel
  - Edificios por nivel

#### Tests:
- [ ] Progresión balanceada
- [ ] Habilidades funcionales
- [ ] Desbloqueos sincronizados en multiplayer

---

### 🎁 Fase 11: Loot y Misiones (2-3 semanas)

**Objetivo**: Contenido rejugable

#### Features Core:
- [ ] **Sistema de Loot**
  - Drops de enemigos (armas, munición)
  - Drops de jugadores (PvP)
  - Rareza (común, raro, épico, legendario)
  - Loot boxes en mundo

- [ ] **Misiones**
  - Misiones diarias
  - Objetivos (matar X enemigos, recolectar Y recursos)
  - Recompensas (XP, items)
  - UI de tracker

- [ ] **Extraction System**
  - Puntos de extracción en mapa
  - Jugador debe llegar para "salvar" inventario
  - Muerte = pérdida de inventario

#### Tests:
- [ ] Loot balanceado
- [ ] Misiones funcionales
- [ ] Extraction sin bugs

---

## 🎨 Polish - Fase 12+

### ✨ Fase 12: Audio y VFX (2-3 semanas)

#### Features:
- [ ] Música dinámica
- [ ] Sonidos posicionales 3D
- [ ] Chat de voz
- [ ] Efectos de partículas
- [ ] Post-processing (bloom, color grading)

### 🔧 Fase 13: Optimización Final (ongoing)

#### Targets:
- [ ] 60 FPS con 500 enemigos
- [ ] <16ms frame time
- [ ] <100MB RAM para chunks
- [ ] <10MB/s bandwidth

### 🚀 Fase 14: Contenido Adicional (futuro)

- [ ] Más tipos de enemigos
- [ ] Más biomas
- [ ] Dungeons subterráneos
- [ ] Clanes y guerras
- [ ] Trading entre jugadores
- [ ] Vehículos
- [ ] Más armas y herramientas

---

## 📊 Timeline Estimado

| Fase | Duración | Acumulado |
|------|----------|-----------|
| ✅ Fase 1 | 4 semanas | 1 mes |
| Fase 2 | 4 semanas | 2 meses |
| Fase 3 | 4 semanas | 3 meses |
| Fase 4 | 4 semanas | 4 meses |
| **MVP Singleplayer** | | **4 meses** |
| Fase 5 | 5 semanas | 5.25 meses |
| Fase 6 | 3 semanas | 6 meses |
| **MVP Multiplayer** | | **6 meses** |
| Fase 7 | 5 semanas | 7.25 meses |
| Fase 8 | 3 semanas | 8 meses |
| Fase 9 | 4 semanas | 9 meses |
| Fase 10 | 3 semanas | 9.75 meses |
| Fase 11 | 3 semanas | 10.5 meses |
| **Versión Completa** | | **~11 meses** |
| Fase 12+ | Ongoing | - |

---

## 🎯 Milestones Clave

### Milestone 1: Gameplay Loop Básico (Mes 4)
- ✅ Movimiento
- ✅ Destrucción de voxels
- ✅ Inventario
- ✅ Enemigos básicos
- ✅ Combate
- ✅ Crafting

### Milestone 2: Multiplayer Funcional (Mes 6)
- ✅ 8 jugadores
- ✅ Sincronización
- ✅ Fuego amigo
- ✅ PvP

### Milestone 3: Mundo Completo (Mes 9)
- ✅ Generación procedural
- ✅ Biomas
- ✅ Clima
- ✅ Construcción

### Milestone 4: Progresión (Mes 11)
- ✅ Niveles
- ✅ Habilidades
- ✅ Loot
- ✅ Misiones

---

## 🔥 Prioridades de Optimización

### Críticas (hacer temprano):
1. **Chunk LOD** - Fase 2
2. **Spatial Hashing** - Fase 3
3. **GPU Instancing** - Fase 3
4. **Delta Compression** - Fase 5

### Importantes (hacer medio):
5. **Client Prediction** - Fase 6
6. **Chunk Streaming** - Fase 7
7. **Procedural Caching** - Fase 7

### Nice-to-have (hacer tarde):
8. **Mesh Shaders** - Fase 12+
9. **Variable Rate Shading** - Fase 12+
10. **Custom Allocators** - Fase 12+

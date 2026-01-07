# Roadmap Detallado - Extraction Shooter Voxel Multijugador

## � Losgros Recientes - Diciembre 2024

### ✅ Sistema de Chunks Dinámicos 32³ - COMPLETADO
- **Problema resuelto**: Stack overflow con chunks de 128³ (42MB cada uno)
- **Solución implementada**: Chunks base de 32³ con heap allocation (172KB cada uno)
- **Beneficio**: 99.6% reducción en uso de memoria por chunk
- **Arquitectura**: Sistema LOD completo como "Lay of the Land"

### ✅ Estructura LOD Dinámica - COMPLETADA
- **5 niveles de LOD** implementados y funcionales
- **Sistema automático** que actualiza LOD según distancia del jugador
- **Preparado para merge**: Estructura completa para combinar chunks automáticamente
- **Performance**: Sistema de actualización eficiente sin impacto en FPS

### ✅ Generación de Terreno Optimizada - COMPLETADA
- **Perlin noise** integrado en BaseChunks
- **16,000+ vértices** por chunk generados correctamente
- **Heap allocation** previene stack overflow completamente
- **Compatibilidad**: Funciona con sistemas existentes de meshing y destrucción

### 🎯 Próximos Hitos Críticos
1. **Fase 2D: PhysX 5.1** - Física realista para drops (2-3 semanas)
2. **Fase 2B: Merge Automático** - Combinar chunks según distancia (1-2 semanas)
3. **Fase 2C: Greedy Meshing** - 70% reducción de triángulos (2-3 semanas)

---

## 🎯 Visión del Juego

**Juego de extracción voxel multijugador** con mundos de misión procedurales, bases subterráneas persistentes, y edificios de hasta 20 pisos. Los jugadores completan misiones en mundos generados basados en biomas del overworld, recolectan recursos, y construyen/defienden bases subterráneas. Sistema de voxels de 10cm para máximo detalle, greedy meshing para rendimiento, y arquitectura de streaming para mundos masivos.

### Características Clave
- **Mundos de Misión**: 3-4 misiones por set, generados proceduralmente según bioma del overworld
- **Bases Subterráneas**: Persistentes, expandibles, con comercio y cultivo
- **Edificios Masivos**: Hasta 20 pisos + sótanos profundos (2048 voxels de altura)
- **Invasiones de Bases**: PvP/PvE opcional para defender/atacar bases
- **Voxels Detallados**: 10cm de resolución como "Lay of the Land"
- **Multijugador Fundamental**: PC primero, consolas después

## 🎯 MVP (Minimum Viable Product) - Fases 1-4

### ✅ Fase 1: Fundamentos (COMPLETADA)
- [x] Setup proyecto Bevy
- [x] Sistema de chunks voxel básico (128³ con altura 512)
- [x] Sistema de chunks voxel básico (128³ con altura 512)
- [x] Terreno con Surface Nets
- [x] Cámara primera persona
- [x] Movimiento WASD + salto
- [x] Física con `bevy_rapier3d`

---

### 🔨 Fase 2B: Merge Automático de Chunks (1-2 semanas) - 🎯 PRÓXIMO

**Objetivo**: Implementar el sistema de merge automático como "Lay of the Land"

#### Features Core:
- [ ] **ChunkMergeScheduler Funcional**
  - Sistema que detecta cuándo chunks deben combinarse/dividirse
  - Queue de tareas de merge/split con prioridades
  - Procesamiento incremental (1-2 operaciones por frame)

- [ ] **Merge Automático por Distancia**
  - Chunks lejanos se combinan automáticamente
  - Chunks cercanos se dividen para más detalle
  - Transiciones suaves sin "popping" visual

- [ ] **Optimización de Draw Calls**
  - 16x16x16 BaseChunks → 1 MergedChunk → 1 draw call
  - Reducción masiva de draw calls para terreno distante
  - Instanced rendering para chunks similares

#### Implementación:
- [ ] Completar `ChunkMergeScheduler` en `dynamic_chunks.rs`
- [ ] Sistema de detección de cambios de LOD
- [ ] Algoritmo de merge de densidades y voxel_types
- [ ] Sistema de split para chunks que se acercan
- [ ] Integración con sistema de meshing existente

#### Tests:
- [ ] 4096 BaseChunks se combinan en 1 MergedChunk correctamente
- [ ] Transiciones LOD sin artifacts visuales
- [ ] Draw calls reducidos de 4096 a 1 en distancia máxima
- [ ] Performance: merge de 16x16x16 chunks < 100ms

---

### 🔨 Fase 2C: Greedy Meshing para Chunks Dinámicos (2-3 semanas)

**Objetivo**: Implementar greedy meshing optimizado para diferentes tamaños de chunk

#### Features Core:
- [ ] **Greedy Meshing Adaptativo**
  - Algoritmo diferente según tamaño de chunk
  - 32³ chunks: máximo detalle (< 5ms)
  - 512³ chunks: optimización agresiva (< 50ms)

- [ ] **Reducción de Triángulos**
  - 70% menos triángulos que meshing naive
  - Combinar caras adyacentes del mismo material
  - Optimización especial para terreno plano

#### Implementación:
- [ ] Crear `src/voxel/greedy_meshing.rs`
- [ ] Algoritmo greedy meshing básico
- [ ] Adaptación para chunks de diferentes tamaños
- [ ] Integración con sistema de LOD

---

### 🔨 Fase 2D: PhysX 5.1 Integration (2-3 semanas) - 🎯 CRÍTICO

**Objetivo**: Física realista para drops y colisiones como "Lay of the Land"

#### Features Core:
- [ ] **PhysX 5.1 Setup**
  - Integración completa con Bevy
  - Mundo de física separado del rendering
  - Sincronización Bevy ↔ PhysX

- [ ] **Drops con Física Real**
  - Drops nunca traspasan terreno
  - Física realista: gravedad, rebotes, fricción
  - Colisiones precisas con chunks dinámicos

- [ ] **Terrain Colliders Dinámicos**
  - Colliders se actualizan cuando chunks cambian
  - Optimización para chunks combinados
  - Cleanup automático de colliders no usados

#### Implementación:
- [ ] Agregar PhysX 5.1 dependency
- [ ] Crear `src/physics/physx_integration.rs`
- [ ] Sistema de sincronización Bevy-PhysX
- [ ] Actualizar `drops.rs` para usar PhysX
- [ ] Integrar con sistema de chunks dinámicos

#### Tests:
- [ ] 1000+ drops con física real a 60 FPS
- [ ] Drops nunca traspasan terreno
- [ ] Colliders se actualizan correctamente con chunks
- [ ] Sin memory leaks en PhysX actors

---

### 🔨 Fase 2: Destrucción y Recursos (3-4 semanas) - 🚧 EN PROGRESO

**Objetivo**: Jugador puede destruir voxels y recolectar recursos con sistema optimizado para edificios altos
**Objetivo**: Jugador puede destruir voxels y recolectar recursos con sistema optimizado para edificios altos

#### Features Core:
- [x] **VoxelType System**
  - Tipos: Dirt, Stone, Wood, Metal
  - Propiedades: hardness, drop_rate, texture
  - Resistencia diferente por tipo

- [x] **Herramientas Básicas**
  - Hacha (para madera)
  - Pico (para piedra/metal)
  - Pala (para tierra)
  - Sistema de durabilidad

- [x] **Destrucción Inteligente**
  - ✅ Raycast desde cámara para detectar voxel objetivo
  - ✅ Cálculo de "golpe efectivo" basado en herramienta y durabilidad
  - ✅ **Destrucción por área (cráteres)**: Diferentes patrones según herramienta
    - Manos: 1 voxel (centro)
    - Pala: 6 voxels (cráter horizontal - excavación)
    - Pico: 7 voxels (cráter cónico - picotazo)
    - Hacha: 11 voxels (cráter vertical - cortar árboles)
  - ✅ Drops variables: 0-30 voxels según herramienta y material


- [x] **Sistema de Drops**
  - ✅ Entidades físicas que caen al suelo
  - ✅ Auto-recolección al acercarse
  - ✅ Despawn después de 60 segundos
  - ✅ Pool de 500 drops máximo
  - ✅ Drops visibles con mesh y física básica
  - ✅ Impulso inicial realista (saltan del suelo)
  - ✅ Delay de recolección (1 segundo)

#### ✅ Nuevas Características Críticas (Inspiradas en "Lay of the Land") - COMPLETADAS:
- [x] **Sistema de Chunks Dinámicos 32³ - FASE 1 COMPLETADA**
  - ✅ Chunks base pequeños de 32³ (172 KB vs 42 MB anteriores!)
  - ✅ Heap allocation para evitar stack overflow
  - ✅ Soporte para edificios de 20 pisos (64 chunks verticales = 2048 voxels)
  - ✅ Memoria ultra-eficiente: BaseChunk estructura implementada
  - ✅ Generación de terreno procedural con Perlin noise
  - ✅ Integración completa con Bevy ECS

- [x] **Sistema LOD Dinámico - ESTRUCTURA COMPLETADA**
  - ✅ Ultra (0-50m): Chunks individuales 32³ (máximo detalle)
  - ✅ High (50-100m): Chunks merged 64³ (2x2x2) - estructura lista
  - ✅ Medium (100-200m): Chunks merged 128³ (4x4x4) - estructura lista
  - ✅ Low (200-400m): Chunks merged 256³ (8x8x8) - estructura lista
  - ✅ Minimal (400m+): Chunks merged 512³ (16x16x16) - estructura lista
  - ✅ Sistema de actualización automática de LOD basado en distancia del jugador

#### 🚧 Pendientes para Completar Sistema Dinámico (Fase 2B):
- [ ] **Merge Automático de Chunks**
  - Implementar ChunkMergeScheduler funcional
  - Sistema que combine BaseChunks según LOD automáticamente
  - Transiciones suaves sin "popping" visual
  - Optimización de draw calls (4096 chunks → 1 draw call en Minimal LOD)

- [ ] **Meshing Optimizado para Chunks Dinámicos**
  - Adaptar generate_mesh_with_neighbors para chunks combinados
  - Greedy meshing específico para diferentes tamaños de chunk
  - Performance targets: <5ms para 32³, <50ms para 512³

- [ ] **Integración Completa BaseChunk → Chunk**
  - Eliminar conversión temporal BaseChunk → Chunk
  - Sistema directo de BaseChunk a mesh
  - Compatibilidad completa con sistemas existentes (destrucción, drops)

#### Optimizaciones de Drops (Implementar progresivamente):
- [ ] **Fase 2A: Detección Real del Suelo**
  - Raycast hacia abajo para encontrar superficie real
  - Soporte para terreno con ruido/irregular
  - Evitar drops flotantes o que traspasen

- [ ] **Fase 2B: Object Pooling Básico**
  - Pool de 1000 entidades pre-allocadas
  - Reutilización sin malloc/free (O(1) spawn/despawn)
  - Reducir garbage collection

- [ ] **Fase 2C: Auto-merging de Drops**
  - Combinar drops del mismo tipo cercanos (radio 1m)
  - Reducir entidades de 500 a ~50-100
  - Animación de merge suave

- [ ] **Fase 2D: Spatial Hashing para Drops**
  - Grid 3D para detección O(1) de recolección
  - Solo verificar drops en celdas cercanas al jugador
  - Optimizar de O(n) a O(1) por frame

- [ ] **Fase 2E: Sistema Híbrido (Física + Visual)**
  - Primeros 50 drops: física completa
  - Drops 51+: animación visual simple
  - Drops cercanos al jugador: prioridad física

- [ ] **Fase 2F: Instanced Rendering**
  - 1 draw call por tipo de drop (no 500 draw calls)
  - GPU buffer con transforms de todos los drops
  - Renderizar 1000+ drops sin impacto CPU

- [ ] **Fase 2G: Física Custom Optimizada**
  - Reemplazar física Bevy por sistema custom
  - Batch processing de 1000 drops en <0.1ms
  - Solo gravedad + rebote básico (sin rotación compleja)

- [ ] **Fase 2H: Chunk-based Drop Management**
  - Drops "duermen" en chunks no cargados
  - Solo procesar drops en chunks activos
  - Persistencia de drops al cambiar chunks

#### Targets de Rendimiento (Actualizados para Sistema Dinámico):
#### Targets de Rendimiento (Actualizados para Sistema Dinámico):
- [ ] 1000 drops simultáneos a 60 FPS
- [ ] Recolección O(1) usando spatial hash
- [ ] <1MB RAM para sistema de drops
- [ ] <0.5ms CPU por frame para 1000 drops
- [ ] **Chunks base: <200KB cada uno (vs 42MB anteriores)**
- [ ] **LOD transitions: <5ms por update**
- [ ] **Chunk merging: <10ms para grupos de 16x16x16**
- [ ] **Chunks base: <200KB cada uno (vs 42MB anteriores)**
- [ ] **LOD transitions: <5ms por update**
- [ ] **Chunk merging: <10ms para grupos de 16x16x16**

- [ ] **Inventario Básico (256 slots)**
  - Estructura de datos eficiente
  - Stacking de items (999 voxels por slot)
  - UI simple para ver inventario
  - Hotbar con 10 slots rápidos

#### Optimizaciones:
- [x] **Raycast optimizado con DDA** (10x más rápido que punto-por-punto)
  - ✅ Implementar algoritmo DDA (Digital Differential Analyzer)
  - Cache de último voxel mirado
  - Separar raycast de UI (cada frame, 2m) vs destrucción (al click, 5m)
- [x] **Face Culling Inteligente entre Chunks**
  - ✅ Verificar chunks vecinos antes de generar caras
  - ✅ Eliminar caras ocultas en bordes de chunks
  - ✅ Reducir vértices innecesarios (~30% menos caras)
- [ ] **ChunkMap con HashMap** para acceso O(1) a chunks (✅ Ya implementado)
- [ ] Chunk re-meshing incremental (solo actualizar chunk modificado)
- [ ] Batch de cambios de voxels (aplicar cada 100ms)
- [x] **Spatial hashing para drops** (Planificado en Fase 2D)
- [ ] **Drop Object Pooling** (Planificado en Fase 2B)
- [ ] **Drop Instanced Rendering** (Planificado en Fase 2F)
- [ ] **Drop Auto-merging** (Planificado en Fase 2C)

#### Tests:
- [x] ✅ Benchmark: raycast DDA < 0.1ms (vs 1ms punto-por-punto) 
- [x] ✅ Face culling: ~30% reducción de caras en bordes 
- [x] ✅ **Sistema de chunks 32³ funcional con heap allocation**
- [x] ✅ **LOD system actualiza automáticamente según distancia del jugador**
- [x] ✅ **Generación de terreno procedural en BaseChunks**
- [x] ✅ **16,000+ vértices por chunk generados correctamente**
- [x] ✅ **Memoria por chunk reducida de 42MB a ~172KB**
- [ ] **Test: Merge automático combina chunks según LOD** (Fase 2B)
- [ ] **Test: Transiciones LOD sin popping visual** (Fase 2B)
- [ ] **Test: Draw calls reducidos con chunks combinados** (Fase 2B)
- [ ] Benchmark: destruir 1000 voxels < 16ms
- [ ] Test: inventario lleno (256 slots) sin lag
- [ ] **Test: 1000 drops simultáneos a 60 FPS** (con optimizaciones)
- [ ] **Test: Detección de suelo real en terreno irregular**
- [ ] **Test: Auto-merge reduce drops de 500 a <100**
- [ ] **Test: Spatial hash O(1) vs O(n) recolección**

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

## 🌐 Arquitectura de Mundos - Fases 5-7

### 🌍 Fase 5: Mundos de Misión Procedurales (4-5 semanas)

**Objetivo**: Sistema completo de mundos de misión basados en biomas del overworld

#### Features Core:
- [ ] **Generación Procedural Basada en Biomas**
  - Volcán: lava, ceniza, estructuras volcánicas
  - Nieve: nieve, hielo, estructuras de clima frío
  - Bosque: árboles densos, estructuras de madera
  - Desierto: arena, cactus, ruinas
  - Ciudad: edificios de concreto, estructuras urbanas

- [ ] **Sistema de Misiones (3-4 por mundo)**
  - Destruir objetivos específicos
  - Recolectar recursos raros
  - Sobrevivir oleadas de enemigos
  - Alcanzar puntos específicos
  - Punto B (extracción) solo accesible tras completar misiones

- [ ] **Dual Contouring para Terreno Avanzado**
  - Terreno suave que se combina con estructuras voxel
  - Preserva bordes afilados para elementos construidos
  - <100ms por chunk de generación de terreno

- [ ] **Streaming de Mundos Dinámico**
  - Carga de mundos de misión en <5 segundos
  - Precarga de chunks adyacentes
  - Descarga de mundos inactivos para liberar memoria
  - Presupuesto de memoria: <4GB total

#### Tests:
- [ ] Generación de mundo basada en bioma funcional
- [ ] 3-4 misiones distribuidas correctamente
- [ ] Extracción solo accesible tras completar misiones
- [ ] Terreno dual contouring se ve natural
- [ ] Streaming de mundos sin tiempos de carga largos

---

### 🏠 Fase 6: Bases Subterráneas Persistentes (3-4 semanas)

**Objetivo**: Bases personales expandibles con comercio y cultivo

#### Features Core:
- [ ] **Sistema de Bases Subterráneas**
  - Base personal persistente para cada jugador
  - Construcción voxel para expansión
  - Todas las modificaciones persisten entre sesiones

- [ ] **Puestos de Comercio**
  - Intercambio de recursos jugador-a-jugador
  - Órdenes de compra/venta
  - Solicitudes de recursos entre jugadores

- [ ] **Sistema de Cultivo**
  - Granjas para generar recursos
  - Diferentes tipos de cultivos
  - Crecimiento en tiempo real

- [ ] **Sistema de Teleportación**
  - Viaje entre mundos de misión y bases
  - Solo permitido tras completar misiones o en zonas seguras
  - Teleportación de emergencia durante invasiones
  - Preserva inventario durante teleportación

#### Tests:
- [ ] Base persiste entre sesiones
- [ ] Construcción voxel funciona en bases
- [ ] Comercio entre jugadores operativo
- [ ] Cultivo genera recursos correctamente
- [ ] Teleportación funciona sin pérdida de inventario

---

### ⚔️ Fase 7: Sistema de Invasión de Bases (2-3 semanas)

**Objetivo**: Tensión y gameplay cooperativo/competitivo

#### Features Core:
- [ ] **Invasiones de Enemigos**
  - Ataques periódicos a bases de jugadores
  - Enemigos adaptativos según defensas de la base
  - Recompensas por defensa exitosa

- [ ] **Invasiones PvP (Opcional)**
  - Jugadores pueden invadir bases de otros (opt-in)
  - Formación de equipos para defensa/ataque
  - Respeta preferencias PvP del jugador

- [ ] **Notificaciones y Defensa**
  - Notificación al propietario durante ataques
  - Teleportación de emergencia a base bajo ataque
  - Sistema de puntos de defensa

#### Tests:
- [ ] Enemigos atacan bases periódicamente
- [ ] PvP invasiones solo con consentimiento
- [ ] Formación de equipos funciona
- [ ] Notificaciones y teleportación de emergencia operativas

---

## � Multoijugador - Fases 8-9
## 🌐 Arquitectura de Mundos - Fases 5-7

### 🌍 Fase 5: Mundos de Misión Procedurales (4-5 semanas)

**Objetivo**: Sistema completo de mundos de misión basados en biomas del overworld

#### Features Core:
- [ ] **Generación Procedural Basada en Biomas**
  - Volcán: lava, ceniza, estructuras volcánicas
  - Nieve: nieve, hielo, estructuras de clima frío
  - Bosque: árboles densos, estructuras de madera
  - Desierto: arena, cactus, ruinas
  - Ciudad: edificios de concreto, estructuras urbanas

- [ ] **Sistema de Misiones (3-4 por mundo)**
  - Destruir objetivos específicos
  - Recolectar recursos raros
  - Sobrevivir oleadas de enemigos
  - Alcanzar puntos específicos
  - Punto B (extracción) solo accesible tras completar misiones

- [ ] **Dual Contouring para Terreno Avanzado**
  - Terreno suave que se combina con estructuras voxel
  - Preserva bordes afilados para elementos construidos
  - <100ms por chunk de generación de terreno

- [ ] **Streaming de Mundos Dinámico**
  - Carga de mundos de misión en <5 segundos
  - Precarga de chunks adyacentes
  - Descarga de mundos inactivos para liberar memoria
  - Presupuesto de memoria: <4GB total

#### Tests:
- [ ] Generación de mundo basada en bioma funcional
- [ ] 3-4 misiones distribuidas correctamente
- [ ] Extracción solo accesible tras completar misiones
- [ ] Terreno dual contouring se ve natural
- [ ] Streaming de mundos sin tiempos de carga largos

---

### 🏠 Fase 6: Bases Subterráneas Persistentes (3-4 semanas)

**Objetivo**: Bases personales expandibles con comercio y cultivo

#### Features Core:
- [ ] **Sistema de Bases Subterráneas**
  - Base personal persistente para cada jugador
  - Construcción voxel para expansión
  - Todas las modificaciones persisten entre sesiones

- [ ] **Puestos de Comercio**
  - Intercambio de recursos jugador-a-jugador
  - Órdenes de compra/venta
  - Solicitudes de recursos entre jugadores

- [ ] **Sistema de Cultivo**
  - Granjas para generar recursos
  - Diferentes tipos de cultivos
  - Crecimiento en tiempo real

- [ ] **Sistema de Teleportación**
  - Viaje entre mundos de misión y bases
  - Solo permitido tras completar misiones o en zonas seguras
  - Teleportación de emergencia durante invasiones
  - Preserva inventario durante teleportación

#### Tests:
- [ ] Base persiste entre sesiones
- [ ] Construcción voxel funciona en bases
- [ ] Comercio entre jugadores operativo
- [ ] Cultivo genera recursos correctamente
- [ ] Teleportación funciona sin pérdida de inventario

---

### ⚔️ Fase 7: Sistema de Invasión de Bases (2-3 semanas)

**Objetivo**: Tensión y gameplay cooperativo/competitivo

#### Features Core:
- [ ] **Invasiones de Enemigos**
  - Ataques periódicos a bases de jugadores
  - Enemigos adaptativos según defensas de la base
  - Recompensas por defensa exitosa

- [ ] **Invasiones PvP (Opcional)**
  - Jugadores pueden invadir bases de otros (opt-in)
  - Formación de equipos para defensa/ataque
  - Respeta preferencias PvP del jugador

- [ ] **Notificaciones y Defensa**
  - Notificación al propietario durante ataques
  - Teleportación de emergencia a base bajo ataque
  - Sistema de puntos de defensa

#### Tests:
- [ ] Enemigos atacan bases periódicamente
- [ ] PvP invasiones solo con consentimiento
- [ ] Formación de equipos funciona
- [ ] Notificaciones y teleportación de emergencia operativas

---

## � Multoijugador - Fases 8-9

### 🌍 Fase 8: Networking Básico (4-5 semanas)
### 🌍 Fase 8: Networking Básico (4-5 semanas)

**Objetivo**: 8 jugadores pueden jugar juntos en mundos de misión y bases
**Objetivo**: 8 jugadores pueden jugar juntos en mundos de misión y bases

#### Features Core:
- [ ] **Setup Lightyear**
  - Servidor dedicado
  - Cliente con predicción
  - Configuración de 8 jugadores

- [ ] **Sincronización Multi-Mundo**
  - Sincronización entre mundos de misión y bases
  - Estado de jugador persistente entre mundos
  - Inventario sincronizado durante teleportación
- [ ] **Sincronización Multi-Mundo**
  - Sincronización entre mundos de misión y bases
  - Estado de jugador persistente entre mundos
  - Inventario sincronizado durante teleportación

- [ ] **Sincronización de Enemigos**
  - Servidor autoritativo
  - Posición, estado
  - Vida, muerte
  - Spawning sincronizado en múltiples mundos
  - Spawning sincronizado en múltiples mundos

- [ ] **Sincronización de Voxels Multi-Mundo**
  - Delta compression para cambios de voxels
- [ ] **Sincronización de Voxels Multi-Mundo**
  - Delta compression para cambios de voxels
  - Batch updates (cada 100ms)
  - Interest management por mundo
  - Persistencia de cambios en bases
  - Interest management por mundo
  - Persistencia de cambios en bases

- [ ] **Fuego Amigo y PvP**
- [ ] **Fuego Amigo y PvP**
  - Daño entre jugadores habilitado
  - Indicadores de equipo (marcadores)
  - PvP opcional en invasiones de bases
  - PvP opcional en invasiones de bases

#### Optimizaciones:
- [ ] Delta compression para voxels
- [ ] Interest management por mundo activo
- [ ] Interest management por mundo activo
- [ ] Bandwidth limiting (<10MB/s por jugador)
- [ ] Compresión de datos de mundo inactivo
- [ ] Compresión de datos de mundo inactivo

#### Tests:
- [ ] 8 jugadores sin lag (<100ms latencia)
- [ ] Destrucción de voxels sincronizada en múltiples mundos
- [ ] Teleportación entre mundos sin desincronización
- [ ] Invasiones PvP funcionales
- [ ] Destrucción de voxels sincronizada en múltiples mundos
- [ ] Teleportación entre mundos sin desincronización
- [ ] Invasiones PvP funcionales

---

### ⚡ Fase 9: Optimización de Red (2-3 semanas)
### ⚡ Fase 9: Optimización de Red (2-3 semanas)

**Objetivo**: Multijugador fluido y eficiente con múltiples mundos
**Objetivo**: Multijugador fluido y eficiente con múltiples mundos

#### Features Core:
- [ ] **Client-Side Prediction Multi-Mundo**
  - Predicción de movimiento en diferentes tipos de mundo
- [ ] **Client-Side Prediction Multi-Mundo**
  - Predicción de movimiento en diferentes tipos de mundo
  - Rollback en caso de desincronización
  - Interpolación suave durante teleportación
  - Interpolación suave durante teleportación

- [ ] **Optimización de Bandwidth Multi-Mundo**
  - Compresión agresiva para datos de mundo
  - Solo enviar cambios (delta) por mundo activo
  - Priorización de datos críticos por proximidad
- [ ] **Optimización de Bandwidth Multi-Mundo**
  - Compresión agresiva para datos de mundo
  - Solo enviar cambios (delta) por mundo activo
  - Priorización de datos críticos por proximidad

- [ ] **Session Management Avanzado**
  - Lobby system con selección de misiones
  - Matchmaking basado en progreso
  - Reconexión automática con restauración de mundo
- [ ] **Session Management Avanzado**
  - Lobby system con selección de misiones
  - Matchmaking basado en progreso
  - Reconexión automática con restauración de mundo

#### Tests:
- [ ] <100ms latencia promedio en múltiples mundos
- [ ] <5MB/s bandwidth por jugador total
- [ ] Reconexión sin pérdida de progreso o posición en mundo
- [ ] <100ms latencia promedio en múltiples mundos
- [ ] <5MB/s bandwidth por jugador total
- [ ] Reconexión sin pérdida de progreso o posición en mundo

---

## 🌲 Mundo Abierto y Progresión - Fases 10-12
## 🌲 Mundo Abierto y Progresión - Fases 10-12

### 🗺️ Fase 10: Mapa Overworld y Progreso (3-4 semanas)
### 🗺️ Fase 10: Mapa Overworld y Progreso (3-4 semanas)

**Objetivo**: Sistema de progresión global con mapa overworld
**Objetivo**: Sistema de progresión global con mapa overworld

#### Features Core:
- [ ] **Mapa Overworld**
  - Mapa global que muestra progreso desbloqueado
  - Regiones con diferentes biomas
  - Sets de misiones (3-4 misiones por set)
  - Desbloqueo progresivo de áreas

- [ ] **Sistema de Progreso**
  - Completar set de misiones desbloquea nueva región
  - Progreso persistente entre sesiones
  - Dificultad escalable según progreso
  - Múltiples jugadores pueden progresar independientemente

- [ ] **Generación Procedural Avanzada**
- [ ] **Mapa Overworld**
  - Mapa global que muestra progreso desbloqueado
  - Regiones con diferentes biomas
  - Sets de misiones (3-4 misiones por set)
  - Desbloqueo progresivo de áreas

- [ ] **Sistema de Progreso**
  - Completar set de misiones desbloquea nueva región
  - Progreso persistente entre sesiones
  - Dificultad escalable según progreso
  - Múltiples jugadores pueden progresar independientemente

- [ ] **Generación Procedural Avanzada**
  - Noise-based terrain (FastNoise2)
  - Montañas, valles, llanuras en overworld
  - Biomas coherentes que influyen en mundos de misión
  - Montañas, valles, llanuras en overworld
  - Biomas coherentes que influyen en mundos de misión

#### Tests:
- [ ] Overworld muestra progreso correctamente
- [ ] Desbloqueo de regiones funciona
- [ ] Progreso persiste entre sesiones
- [ ] Biomas influyen en generación de misiones
- [ ] Overworld muestra progreso correctamente
- [ ] Desbloqueo de regiones funciona
- [ ] Progreso persiste entre sesiones
- [ ] Biomas influyen en generación de misiones

---

### �️ Fase 11: Clima y Ambiente (2-3 semanas)
### �️ Fase 11: Clima y Ambiente (2-3 semanas)

**Objetivo**: Mundos vivos con clima dinámico
**Objetivo**: Mundos vivos con clima dinámico

#### Features Core:
- [ ] **Ciclo Día/Noche**
  - 20 minutos real = 1 día en juego
  - Iluminación dinámica
  - Skybox dinámico por bioma
  - Skybox dinámico por bioma

- [ ] **Clima Dinámico por Bioma**
- [ ] **Clima Dinámico por Bioma**
  - Lluvia (reduce visibilidad)
  - Nieve (en bioma nieve)
  - Tormentas de arena (en desierto)
  - Niebla volcánica (en volcán)
  - Tormentas de arena (en desierto)
  - Niebla volcánica (en volcán)

- [ ] **Sistema de Agua**
  - Agua estática (lagos, ríos)
  - Natación
  - Ahogamiento (daño después de 30s bajo agua)

- [ ] **Animales por Bioma**
- [ ] **Animales por Bioma**
  - Neutrales (conejos, ciervos)
  - Hostiles (lobos, osos)
  - Drops de recursos específicos por bioma
  - Drops de recursos específicos por bioma

#### Tests:
- [ ] Ciclo día/noche sin drops de FPS
- [ ] Clima apropiado por bioma
- [ ] Natación y ahogamiento funcionales
- [ ] Animales spawean según bioma
- [ ] Clima apropiado por bioma
- [ ] Natación y ahogamiento funcionales
- [ ] Animales spawean según bioma

---

### 🏗️ Fase 12: Construcción Avanzada (3-4 semanas)
### 🏗️ Fase 12: Construcción Avanzada (3-4 semanas)

**Objetivo**: Sistema completo de construcción para bases y estructuras
**Objetivo**: Sistema completo de construcción para bases y estructuras

#### Features Core:
- [ ] **Modo Construcción en Bases**
- [ ] **Modo Construcción en Bases**
  - Colocar voxels desde inventario
  - Preview de colocación
  - Rotación de bloques
  - Herramientas de construcción especializadas
  - Herramientas de construcción especializadas

- [ ] **Estructuras Defensivas**
- [ ] **Estructuras Defensivas**
  - Muros defensivos
  - Torretas automáticas
  - Torretas automáticas
  - Trampas básicas
  - Puertas y sistemas de acceso
  - Puertas y sistemas de acceso

- [ ] **Física de Colapso Mejorada**
- [ ] **Física de Colapso Mejorada**
  - Edificios sin soporte colapsan
  - Simulación simplificada (voxels desaparecen)
  - Simulación simplificada (voxels desaparecen)
  - Drops de voxels al colapsar
  - Detección de integridad estructural
  - Detección de integridad estructural

#### Tests:
- [ ] Construcción fluida en bases
- [ ] Estructuras defensivas funcionales
- [ ] Construcción fluida en bases
- [ ] Estructuras defensivas funcionales
- [ ] Colapso de edificios sin crash
- [ ] Sincronización de construcciones en multiplayer

---

## 🎨 Polish y Optimización - Fases 15+
## 🎨 Polish y Optimización - Fases 15+

### ✨ Fase 15: Audio y VFX (2-3 semanas)
### ✨ Fase 15: Audio y VFX (2-3 semanas)

#### Features:
- [ ] Música dinámica por bioma y situación
- [ ] Música dinámica por bioma y situación
- [ ] Sonidos posicionales 3D
- [ ] Chat de voz posicional
- [ ] Efectos de partículas para destrucción masiva
- [ ] Chat de voz posicional
- [ ] Efectos de partículas para destrucción masiva
- [ ] Post-processing (bloom, color grading)
- [ ] Efectos visuales para teleportación
- [ ] Audio ambiental por tipo de mundo
- [ ] Efectos visuales para teleportación
- [ ] Audio ambiental por tipo de mundo

### 🔧 Fase 16: Optimización Final (ongoing)
### 🔧 Fase 16: Optimización Final (ongoing)

#### Targets Actualizados:
- [ ] 60 FPS con 500 enemigos en múltiples mundos
- [ ] <16ms frame time con greedy meshing
- [ ] <4GB RAM total para todos los mundos cargados
- [ ] <10MB/s bandwidth por jugador
- [ ] <5 segundos carga de mundo de misión
- [ ] <1 segundo teleportación entre mundos
#### Targets Actualizados:
- [ ] 60 FPS con 500 enemigos en múltiples mundos
- [ ] <16ms frame time con greedy meshing
- [ ] <4GB RAM total para todos los mundos cargados
- [ ] <10MB/s bandwidth por jugador
- [ ] <5 segundos carga de mundo de misión
- [ ] <1 segundo teleportación entre mundos

### 🚀 Fase 17: Contenido Adicional (futuro)
### 🚀 Fase 17: Contenido Adicional (futuro)

- [ ] Más tipos de enemigos por bioma
- [ ] Más biomas (pantano, tundra, cavernas)
- [ ] Dungeons subterráneos procedurales
- [ ] Clanes y guerras entre bases
- [ ] Trading automatizado entre bases
- [ ] Vehículos para exploración rápida
- [ ] Más armas y herramientas especializadas
- [ ] Eventos mundiales que afectan todos los jugadores
- [ ] Construcción colaborativa de mega-estructuras

- [ ] Más tipos de enemigos por bioma
- [ ] Más biomas (pantano, tundra, cavernas)
- [ ] Dungeons subterráneos procedurales
- [ ] Clanes y guerras entre bases
- [ ] Trading automatizado entre bases
- [ ] Vehículos para exploración rápida
- [ ] Más armas y herramientas especializadas
- [ ] Eventos mundiales que afectan todos los jugadores
- [ ] Construcción colaborativa de mega-estructuras

---

## 📊 Timeline Estimado Actualizado
## 📊 Timeline Estimado Actualizado

| Fase | Duración | Acumulado | Enfoque |
|------|----------|-----------|---------|
| ✅ Fase 1 | 4 semanas | 1 mes | Fundamentos |
| ✅ Fase 2A | 2 semanas | 1.5 meses | Chunks Dinámicos 32³ + LOD |
| Fase 2B | 2 semanas | 2 meses | Merge Automático |
| Fase 2C | 3 semanas | 2.75 meses | Greedy Meshing |
| Fase 2D | 3 semanas | 3.5 meses | PhysX 5.1 Integration |
| Fase 2E | 2 semanas | 4 meses | Drops + Inventario Optimizado |
| Fase 3 | 4 semanas | 5 meses | Enemigos |
| Fase 4 | 4 semanas | 6 meses | Armas y Crafting |
| **MVP Singleplayer** | | **6 meses** | |
| Fase 5 | 5 semanas | 7.25 meses | Mundos de Misión Procedurales |
| Fase 6 | 4 semanas | 8.25 meses | Bases Subterráneas |
| Fase 7 | 3 semanas | 9 meses | Invasiones de Bases |
| **MVP Arquitectura de Mundos** | | **9 meses** | |
| Fase 8 | 5 semanas | 10.25 meses | Networking Básico |
| Fase 9 | 3 semanas | 11 meses | Optimización de Red |
| **MVP Multiplayer** | | **11 meses** | |
| Fase 10 | 4 semanas | 10 meses | Overworld y Progreso |
| Fase 11 | 3 semanas | 10.75 meses | Clima y Ambiente |
| Fase 12 | 4 semanas | 11.75 meses | Construcción Avanzada |
| Fase 13 | 3 semanas | 12.5 meses | Niveles y Habilidades |
| Fase 14 | 3 semanas | 13.25 meses | Loot y Extracción |
| **Versión Completa** | | **~13.5 meses** | |
| Fase 15+ | Ongoing | - | Polish y Contenido |

---

## 🎯 Milestones Clave Actualizados
## 🎯 Milestones Clave Actualizados

### Milestone 1: Gameplay Loop Básico (Mes 4)
- ✅ Movimiento
- ✅ Destrucción de voxels con chunks 2048
- ✅ Greedy meshing para rendimiento
- ✅ Destrucción de voxels con chunks 2048
- ✅ Greedy meshing para rendimiento
- ✅ Inventario
- ✅ Enemigos básicos
- ✅ Combate
- ✅ Crafting

### Milestone 2: Arquitectura de Mundos (Mes 7)
- ✅ Mundos de misión procedurales
- ✅ Bases subterráneas persistentes
- ✅ Sistema de invasión de bases
- ✅ Dual contouring para terreno
- ✅ Streaming de mundos dinámico

### Milestone 3: Multiplayer Funcional (Mes 9)
- ✅ 8 jugadores en múltiples mundos
- ✅ Sincronización multi-mundo
- ✅ PvP en invasiones de bases
- ✅ Teleportación entre mundos
### Milestone 2: Arquitectura de Mundos (Mes 7)
- ✅ Mundos de misión procedurales
- ✅ Bases subterráneas persistentes
- ✅ Sistema de invasión de bases
- ✅ Dual contouring para terreno
- ✅ Streaming de mundos dinámico

### Milestone 3: Multiplayer Funcional (Mes 9)
- ✅ 8 jugadores en múltiples mundos
- ✅ Sincronización multi-mundo
- ✅ PvP en invasiones de bases
- ✅ Teleportación entre mundos

### Milestone 4: Mundo Completo (Mes 12)
- ✅ Mapa overworld con progresión
- ✅ Clima dinámico por bioma
- ✅ Construcción avanzada en bases
- ✅ Sistema de streaming optimizado
### Milestone 4: Mundo Completo (Mes 12)
- ✅ Mapa overworld con progresión
- ✅ Clima dinámico por bioma
- ✅ Construcción avanzada en bases
- ✅ Sistema de streaming optimizado

### Milestone 5: Progresión Completa (Mes 13.5)
- ✅ Niveles y habilidades
- ✅ Loot por bioma
- ✅ Sistema de extracción
- ✅ Misiones dinámicas
### Milestone 5: Progresión Completa (Mes 13.5)
- ✅ Niveles y habilidades
- ✅ Loot por bioma
- ✅ Sistema de extracción
- ✅ Misiones dinámicas

---

## 🔥 Prioridades de Optimización Actualizadas
## 🔥 Prioridades de Optimización Actualizadas

### Críticas (hacer temprano):
1. ✅ **Chunks 32³ Dinámicos** - Fase 2 (COMPLETADO - Estructura base lista)
2. **Merge Automático de Chunks** - Fase 2B (PRÓXIMO)
3. **Greedy Meshing** - Fase 2C (PRÓXIMO)
4. **PhysX 5.1 Integration** - Fase 2D (PRÓXIMO)
5. **Dual Contouring** - Fase 5 
6. **World Streaming** - Fase 5 
7. ✅ **DDA Raycast** - Completado ✅
8. ✅ **Face Culling Inteligente** - Completado ✅

### Importantes (hacer medio):
7. **Memory Management Multi-Mundo** - Fase 5-6 (NUEVO)
8. **Client Prediction Multi-Mundo** - Fase 9
9. **Chunk Streaming por Mundo** - Fase 8
10. **Compression de Mundos Inactivos** - Fase 9 (NUEVO)
7. **Memory Management Multi-Mundo** - Fase 5-6 (NUEVO)
8. **Client Prediction Multi-Mundo** - Fase 9
9. **Chunk Streaming por Mundo** - Fase 8
10. **Compression de Mundos Inactivos** - Fase 9 (NUEVO)

### Nice-to-have (hacer tarde):
11. **Mesh Shaders** - Fase 15+
12. **Variable Rate Shading** - Fase 15+
13. **Custom Allocators** - Fase 15+
14. **GPU-Driven Culling** - Fase 15+ (NUEVO)
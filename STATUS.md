# Estado Actual del Proyecto - Voxel Game

**Fecha de última actualización**: Abril 2026

## Resumen

Juego voxel en primera persona (estilo Minecraft) construido con Bevy 0.17 + Rapier3D. El mundo es terreno procedural infinito dividido en chunks de 32³. Actualmente funcional con generación asíncrona, sistema de biomas, greedy meshing, y un sistema LOD de dos capas (chunks reales cercanos + chunks LOD distantes).

**Rendimiento actual**: ~30-45 FPS (sin optimizaciones pendientes aplicadas)

---

## ✅ Sistemas Implementados y Funcionando

### 1. Generación de Terreno con Biomas
- **5 biomas**: Plains, Hills, Mountains, Valley, Plateau
- **FastNoiseLite** para ruido procedural con transiciones suaves entre biomas
- **Heightmap cacheado**: Se calcula una vez por columna XZ (1,089 evaluaciones) en lugar de por cada voxel (35,937), reduciendo cálculos ~33x
- **Rayon** para asignación de tipos de voxel en paralelo
- **Archivo**: `src/voxel/biomes.rs`, `src/voxel/dynamic_chunks.rs`

### 2. Chunks Asíncronos (Real Chunks)
- **Radio de carga**: 64 chunks horizontales
- **Radio de descarga**: 70 chunks
- **Rango vertical**: Y=-1 hasta Y=3 (5 niveles)
- **Generación en background**: `AsyncComputeTaskPool`, hasta 32 chunks por frame
- **Ordenados por distancia**: Los chunks más cercanos al jugador se cargan primero
- **Archivo**: `src/voxel/chunk_loading.rs`

### 3. Sistema LOD para Chunks Distantes
- **Rango**: 32 a 200 chunks de distancia
- **Tres niveles LOD**: Medium (16² grid), Low (8² grid), Minimal (4² grid)
- **Solo superficie**: Guarda altura + tipo de voxel por columna, sin volumen completo
- **Sin colisión física**: Mucho más barato de renderizar
- **Conversiones Real ↔ LOD**: Con histéresis (30/36 chunks) para evitar thrashing
- **Archivo**: `src/voxel/lod_chunks.rs`, `src/voxel/chunk_loading.rs`

### 4. Greedy Meshing
- **Reducción de triángulos**: 70-95% comparado con naive meshing
- **Verificación cross-chunk**: Elimina caras en las costuras entre chunks
- **Versión simple** (sin vecinos): Usada durante generación inicial y async tasks
- **Versión completa** (con vecinos): Usada al completar la tarea async para corregir seams
- **Archivo**: `src/voxel/greedy_meshing.rs`

### 5. Física y Colisiones
- **Rapier3D**: Colisores generados desde el mesh de cada chunk real
- **Solo chunks reales tienen colisión** — chunks LOD no tienen colider
- **Voxel breaking**: Raycast desde cámara, re-meshea chunks afectados
- **Archivo**: `src/physics/`, `src/voxel/destruction.rs`

### 6. Frustum Culling
- **Estado**: ✅ HABILITADO (en main.rs)
- **Método**: Distancia + ángulo (FOV 110°, distancia máx 200m)
- **Limitación conocida**: Solo aplica a `BaseChunk`. Los `LodChunk` **no son culleados** — este es un bug pendiente.
- **Archivo**: `src/voxel/frustum_culling.rs`

### 7. Estructuras de Datos Espaciales
- **ChunkMap**: `HashMap<IVec3, Entity>` — lookup O(1) por posición
- **ChunkOctree**: Búsquedas espaciales O(log n)
- **SpatialHashGrid**: Queries de radio horizontal eficientes
- **Archivos**: `src/voxel/octree.rs`, `src/voxel/spatial_hash.rs`

### 8. UI y Game States
- **Estados**: `MainMenu` → `InGame` → `Paused`
- **Menú principal**: Play / Settings
- **HUD**: Overlay de FPS y frame time (esquina superior izquierda)
- **Archivos**: `src/ui/`, `src/core/states.rs`, `src/debug/`

---

## ⚠️ Sistemas Deshabilitados (Código Presente, No Activo)

### Caché en Disco (`src/voxel/chunk_cache.rs`)
- **Razón**: I/O síncrono hacía todo muy lento
- **Futuro**: Necesita reescritura con I/O asíncrono en batch

### LOD Downsampling (`src/voxel/downsampling.rs`)
- **Razón**: Panic por overflow en el algoritmo de downsampling
- **Futuro**: Corregir el bug de overflow antes de habilitar

---

## 🐛 Bugs Conocidos

| Bug | Impacto | Prioridad |
|-----|---------|-----------|
| Frustum culling no cubre `LodChunk` | FPS más bajo de lo posible (GPU renderiza chunks LOD fuera de vista) | Alta |
| Chunks LOD se generan en el main thread | Stutters al cargar chunks distantes | Alta |
| Seams visuales al inicio | Cracks en bordes de chunks durante los primeros segundos | Baja |
| Frustum culling con bugs en edge cases | Chunks pueden desaparecer incorrectamente | Media |

---

## 📊 Constantes de Configuración

```rust
// src/voxel/chunk_loading.rs
CHUNK_LOAD_RADIUS = 64          // Radio de carga de chunks reales
CHUNK_UNLOAD_RADIUS = 70        // Radio de descarga
MAX_CHUNKS_PER_FRAME = 32       // Chunks generados por frame (async)
REAL_CHUNK_RADIUS = 32          // Hasta aquí: chunks con física
LOD_TO_REAL_DISTANCE = 30       // Umbral conversión LOD → Real
REAL_TO_LOD_DISTANCE = 36       // Umbral conversión Real → LOD (histéresis)
MAX_LOD_RADIUS = 200            // Límite máximo de chunks LOD

// src/core/constants.rs
BASE_CHUNK_SIZE = 32            // Tamaño de chunk en voxels
VOXEL_SIZE = 0.1                // Tamaño de voxel en metros (10cm)
```

---

## 🎮 Controles

| Tecla | Acción |
|-------|--------|
| WASD | Movimiento |
| Espacio | Saltar |
| Mouse | Mirar alrededor |
| Click Izquierdo (hold) | Romper voxel |
| ESC | Salir |

---

## 📁 Archivos Clave

```
src/
├── main.rs                        # Inicialización, registro de sistemas
├── core/
│   ├── constants.rs               # BASE_CHUNK_SIZE, VOXEL_SIZE, etc.
│   └── states.rs                  # GameState enum
├── voxel/
│   ├── dynamic_chunks.rs          # BaseChunk, generate_terrain()
│   ├── biomes.rs                  # BiomeGenerator, TerrainGenerator
│   ├── chunk_loading.rs           # Carga/descarga async, sistema LOD
│   ├── greedy_meshing.rs          # Algoritmo de meshing optimizado
│   ├── lod_chunks.rs              # LodChunk, mesh_lod_chunk()
│   ├── lod_system.rs              # Sistema de actualización LOD
│   ├── frustum_culling.rs         # Culling por distancia y ángulo
│   ├── destruction.rs             # Raycast + voxel breaking
│   ├── octree.rs                  # ChunkOctree para búsquedas espaciales
│   ├── spatial_hash.rs            # SpatialHashGrid
│   ├── voxel_types.rs             # VoxelType enum (Air, Dirt, Stone, etc.)
│   ├── chunk_cache.rs             # Caché en disco (DESHABILITADO)
│   └── downsampling.rs            # Downsampling LOD (DESHABILITADO)
├── player/                        # Controlador primera persona, cámara
├── physics/                       # Integración Rapier3D
├── ui/                            # Menú principal, HUD
└── debug/                         # Overlay de FPS/frame time
```

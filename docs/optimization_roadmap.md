# 🚀 Roadmap de Optimización - Voxel Game

## ✅ Implementado (Phases 1-4)

### Phase 1: Render Distance Aumentado
- ✅ Radio de carga: 32 chunks (~100m, ~3,200 chunks visibles)
- ✅ Puede aumentarse fácilmente a 64+ chunks
- ✅ Sistema de LOD con 5 niveles (Ultra → High → Medium → Low → Minimal)
- ✅ Colores de debug: verde → amarillo → naranja → rojo
- **Archivo:** `src/voxel/chunk_loading.rs`
- **Constante:** `CHUNK_LOAD_RADIUS = 32`

### Phase 2: Generación Asíncrona ⭐ CRÍTICO
- ✅ Chunks se generan en **background threads** (AsyncComputeTaskPool)
- ✅ No congela el juego durante generación
- ✅ 64 chunks por frame (muy rápido)
- ✅ Sistema de tareas con polling
- ✅ Logs de progreso cada 2 segundos
- **Archivos:** `src/voxel/chunk_loading.rs`
- **Sistemas:** `load_chunks_system()`, `complete_chunk_generation_system()`
- **Constante:** `MAX_CHUNKS_PER_FRAME = 64`

### Phase 3: Caché Persistente en Disco
- ✅ Sistema completo implementado
- ⚠️ **DESHABILITADO** porque hace todo muy lento (I/O síncrono)
- 💡 Necesita optimización (ver Phase 6)
- **Archivo:** `src/voxel/chunk_cache.rs`
- **Funciones:** `save_chunk_to_disk()`, `load_chunk_from_disk()`
- **Formato:** Binario con versión, ~35KB por chunk

### Phase 4: Mesh Downsampling
- ✅ Sistema completo implementado
- ✅ 3 niveles: 2x (16³), 4x (8³), 8x (4³)
- ✅ Algoritmo de "voxel más común"
- ✅ Greedy meshing adaptado
- ⚠️ **DESHABILITADO** temporalmente para mejor rendimiento inicial
- 💡 Funciona pero necesita optimización (ver Phase 7)
- **Archivos:** `src/voxel/downsampling.rs`, `src/voxel/greedy_meshing.rs`
- **Struct:** `DownsampledChunk`

### Extras Implementados
- ✅ **Octree** para búsquedas espaciales O(log n)
  - **Archivo:** `src/voxel/octree.rs`
- ✅ **Greedy Meshing** (reduce triángulos 70-95%)
  - **Archivo:** `src/voxel/greedy_meshing.rs`
- ✅ **Sistema de carga/descarga dinámica**
  - **Archivo:** `src/voxel/chunk_loading.rs`
- ✅ **Generación procedural con FastNoiseLite + Rayon**
  - **Archivo:** `src/voxel/dynamic_chunks.rs`

---

## 🚀 Por Implementar (Phases 5-10)

### Phase 5: Frustum Culling ⭐ ALTA PRIORIDAD
**Qué es:** Solo renderizar chunks que están en el campo de visión de la cámara

**Impacto:** 
- 50-75% menos chunks renderizados
- Mejora FPS dramáticamente
- Permite radios de carga mucho mayores

**Dificultad:** Media (2-3 horas)

**Implementación:**
1. Calcular frustum de la cámara (6 planos)
2. Usar Octree para query rápido de chunks en frustum
3. Marcar chunks como `Visibility::Hidden` si están fuera
4. Sistema que actualiza visibilidad cada frame

**Archivos a modificar:**
- Crear: `src/voxel/frustum_culling.rs`
- Modificar: `src/voxel/mod.rs`, `src/main.rs`

**Pseudocódigo:**
```rust
fn update_frustum_culling(
    camera: Query<&Transform, With<Camera>>,
    mut chunks: Query<(&BaseChunk, &mut Visibility)>,
) {
    let frustum = calculate_frustum(camera);
    for (chunk, mut visibility) in chunks.iter_mut() {
        if frustum.contains(chunk.position) {
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}
```

---

### Phase 6: Caché Optimizado ⭐ ALTA PRIORIDAD
**Problema actual:** Guardar cada chunk al disco es MUY lento (I/O síncrono)

**Soluciones:**

#### 6.1: Caché en Memoria (Prioridad 1)
- **Impacto:** Carga instantánea
- **Dificultad:** Fácil (1 hora)
- **Implementación:** HashMap<IVec3, BaseChunk> en memoria
```rust
#[derive(Resource)]
struct ChunkMemoryCache {
    chunks: HashMap<IVec3, BaseChunk>,
    max_size: usize, // Límite de memoria
}
```

#### 6.2: Guardado Asíncrono en Batch
- **Impacto:** Persistencia sin lag
- **Dificultad:** Media (2-3 horas)
- **Implementación:** 
  - Cola de chunks a guardar
  - Thread separado que guarda 100 chunks a la vez
  - No bloquea el juego

#### 6.3: Compresión
- **Impacto:** Archivos 5-10x más pequeños
- **Dificultad:** Fácil (1 hora)
- **Implementación:** Usar `zstd` o `lz4` crate
- **Tamaño:** ~35KB → ~3-7KB por chunk

#### 6.4: Formato Binario Optimizado
- **Impacto:** Menos bytes, más rápido
- **Dificultad:** Media (2 horas)
- **Implementación:**
  - RLE (Run-Length Encoding) para voxels repetidos
  - Solo guardar voxels sólidos
  - Paleta de colores compartida

**Archivos a modificar:**
- `src/voxel/chunk_cache.rs`
- Agregar: `Cargo.toml` (zstd dependency)

---

### Phase 7: Mesh Downsampling Optimizado
**Problema actual:** Regenerar mesh cada vez que cambia LOD es lento

**Soluciones:**

#### 7.1: Pre-generar todos los LODs
- **Impacto:** Cambios de LOD instantáneos
- **Dificultad:** Fácil (1 hora)
- **Implementación:**
```rust
struct ChunkMeshes {
    ultra: Handle<Mesh>,    // 32³
    high: Handle<Mesh>,     // 32³
    medium: Handle<Mesh>,   // 16³
    low: Handle<Mesh>,      // 8³
    minimal: Handle<Mesh>,  // 4³
}
```

#### 7.2: Cachear Meshes en Memoria
- **Impacto:** No regenerar nunca
- **Dificultad:** Fácil (30 min)
- **Implementación:** Guardar meshes downsampled en componente

#### 7.3: Solo Cambiar Handle
- **Impacto:** Cambio instantáneo
- **Dificultad:** Muy fácil (15 min)
- **Implementación:**
```rust
// En lugar de regenerar mesh:
mesh_handle.0 = chunk_meshes.get_lod(new_lod);
```

**Archivos a modificar:**
- `src/voxel/lod_system.rs`
- `src/voxel/chunk_loading.rs`

---

### Phase 8: Chunk Pooling ⭐ OPTIMIZACIÓN EXTREMA
**Qué es:** Reutilizar chunks en lugar de crear/destruir

**Impacto:** 
- Elimina allocations (Box::new())
- 2-3x más rápido
- Menos fragmentación de memoria

**Dificultad:** Media (2-3 horas)

**Implementación:**
```rust
#[derive(Resource)]
struct ChunkPool {
    available: Vec<BaseChunk>,
    max_size: usize,
}

impl ChunkPool {
    fn acquire(&mut self) -> BaseChunk {
        self.available.pop().unwrap_or_else(|| BaseChunk::empty())
    }
    
    fn release(&mut self, chunk: BaseChunk) {
        if self.available.len() < self.max_size {
            self.available.push(chunk);
        }
    }
}
```

**Archivos a crear:**
- `src/voxel/chunk_pool.rs`

**Archivos a modificar:**
- `src/voxel/chunk_loading.rs`
- `src/voxel/mod.rs`

---

### Phase 9: GPU Instancing para Chunks Lejanos
**Qué es:** Renderizar múltiples chunks con una sola draw call

**Impacto:** 
- 5-10x menos draw calls
- Mejor uso de GPU
- Permite 100+ chunks visibles sin lag

**Dificultad:** Alta (4-6 horas)

**Implementación:**
1. Agrupar chunks por LOD
2. Usar instanced rendering de Bevy
3. Pasar posiciones como instance data
4. Un mesh, múltiples posiciones

**Archivos a crear:**
- `src/voxel/instanced_rendering.rs`

**Archivos a modificar:**
- `src/voxel/lod_system.rs`
- `src/main.rs`

**Nota:** Bevy 0.17 tiene soporte nativo para instancing

---

### Phase 10: Generación Procedural Mejorada
**Optimizaciones:**

#### 10.1: Caché de Noise
- **Impacto:** No recalcular ruido
- **Dificultad:** Fácil (1 hora)
- **Implementación:** HashMap<(i32, i32), f32> para valores de noise

#### 10.2: Generación por Capas
- **Impacto:** Solo generar lo visible
- **Dificultad:** Media (2 horas)
- **Implementación:** 
  - Generar superficie primero
  - Generar interior solo si se destruye

#### 10.3: Biomas Pre-calculados
- **Impacto:** Lookup instantáneo
- **Dificultad:** Media (2-3 horas)
- **Implementación:**
  - Mapa de biomas 2D
  - Lookup table para propiedades
  - No calcular en runtime

**Archivos a modificar:**
- `src/voxel/dynamic_chunks.rs`

---

## 📊 Prioridades para Velocidad Extrema

### Corto Plazo (Máximo Impacto - 4-6 horas)
1. **Frustum Culling** (2-3h) - 50-75% menos chunks renderizados
2. **Caché en Memoria** (1h) - Carga instantánea
3. **Pre-generar LODs** (1h) - Cambios instantáneos

**Resultado esperado:**
- ✅ 64+ chunks de radio sin lag
- ✅ Solo renderizar lo visible
- ✅ Carga instantánea de chunks visitados
- ✅ 60+ FPS constante

### Mediano Plazo (Optimización Profunda - 6-10 horas)
4. **Chunk Pooling** (2-3h) - Eliminar allocations
5. **Guardado Asíncrono en Batch** (2-3h) - Persistencia sin lag
6. **Compresión de Caché** (1h) - Archivos más pequeños
7. **Habilitar Downsampling** (1h) - Reducir triángulos distantes

**Resultado esperado:**
- ✅ 128+ chunks de radio
- ✅ Persistencia sin impacto en FPS
- ✅ Uso de memoria optimizado
- ✅ 90+ FPS constante

### Largo Plazo (Rendimiento Extremo - 10-15 horas)
8. **GPU Instancing** (4-6h) - Menos draw calls
9. **Generación Procedural Mejorada** (3-4h) - Más rápido
10. **Streaming desde Disco** (3-4h) - Mundos infinitos

**Resultado esperado:**
- ✅ 256+ chunks de radio (Distant Horizons style)
- ✅ Mundos infinitos con persistencia
- ✅ 120+ FPS constante
- ✅ Listo para biomas, edificios, pueblos, etc.

---

## 🎯 Recomendación Inmediata

Para hacer el juego **extremadamente rápido** ahora mismo, implementar en este orden:

### Día 1 (4-6 horas)
1. ✅ **Frustum Culling** - Mayor impacto inmediato
2. ✅ **Caché en Memoria** - Carga instantánea
3. ✅ **Pre-generar LODs** - Cambios suaves

### Día 2 (6-8 horas)
4. ✅ **Chunk Pooling** - Eliminar allocations
5. ✅ **Habilitar Downsampling** - Ya está implementado
6. ✅ **Guardado Asíncrono** - Persistencia sin lag

### Día 3+ (Según necesidad)
7. ✅ **GPU Instancing** - Para 256+ chunks
8. ✅ **Generación Mejorada** - Para biomas complejos
9. ✅ **Streaming** - Para mundos infinitos

---

## 📝 Notas Importantes

### Estado Actual del Proyecto
- **Funciona:** Generación asíncrona, greedy meshing, LOD system
- **Deshabilitado:** Caché en disco (muy lento), downsampling (temporal)
- **Rendimiento actual:** ~60 FPS con 32 chunks de radio
- **Objetivo:** 120+ FPS con 256+ chunks de radio

### Archivos Clave
```
src/
├── voxel/
│   ├── chunk_loading.rs      # Sistema de carga asíncrona ⭐
│   ├── lod_system.rs          # Sistema de LOD
│   ├── greedy_meshing.rs      # Optimización de meshes
│   ├── dynamic_chunks.rs      # Generación procedural
│   ├── octree.rs              # Búsquedas espaciales
│   ├── chunk_cache.rs         # Caché (deshabilitado)
│   └── downsampling.rs        # Downsampling (deshabilitado)
├── core/
│   └── constants.rs           # Constantes globales
└── main.rs                    # Inicialización
```

### Constantes Importantes
```rust
// src/voxel/chunk_loading.rs
CHUNK_LOAD_RADIUS = 32          // Radio de carga (chunks)
CHUNK_UNLOAD_RADIUS = 40        // Radio de descarga
MAX_CHUNKS_PER_FRAME = 64       // Chunks generados por frame

// src/core/constants.rs
BASE_CHUNK_SIZE = 32            // Tamaño de chunk (voxels)
VOXEL_SIZE = 0.1                // Tamaño de voxel (metros)
LOD_DISTANCES = [32, 64, 128, 192, 256]  // Distancias LOD
```

### Para Aumentar Render Distance
1. Cambiar `CHUNK_LOAD_RADIUS` en `chunk_loading.rs`
2. Implementar Frustum Culling primero
3. Aumentar `MAX_CHUNKS_PER_FRAME` si es necesario
4. Monitorear FPS y ajustar

---

## 🔧 Comandos Útiles

```bash
# Compilar en modo release (más rápido)
cargo build --release

# Ejecutar con optimizaciones
cargo run --release

# Ver warnings de rendimiento
cargo clippy

# Limpiar caché (si está habilitado)
rm -rf world_cache/

# Profiling (requiere cargo-flamegraph)
cargo flamegraph
```

---

## 📚 Referencias

- **Greedy Meshing:** https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/
- **Frustum Culling:** https://learnopengl.com/Guest-Articles/2021/Scene/Frustum-Culling
- **Octree:** https://en.wikipedia.org/wiki/Octree
- **Distant Horizons Mod:** https://modrinth.com/mod/distanthorizons
- **Bevy Instancing:** https://bevyengine.org/examples/3d-rendering/instancing/

---

**Última actualización:** 2025-12-30
**Versión del juego:** 0.1.0
**Bevy Version:** 0.17.3

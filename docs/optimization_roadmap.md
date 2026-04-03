# Roadmap de Optimización de Rendimiento

**Última actualización**: Abril 2026
**Rendimiento actual**: ~30-45 FPS

---

## ✅ Optimizaciones Ya Implementadas

### Generación de Terreno
- **Heightmap cacheado por columna XZ**: Calcula el noise una vez por (x,z) y reutiliza para todos los Y del chunk. Reduce ~33x las llamadas a FastNoiseLite.
- **Rayon para tipos de voxel**: Los 32,768 voxels de un chunk se clasifican en paralelo.
- **Box allocation**: Arrays en heap para evitar stack overflow.

### Carga de Chunks
- **Generación asíncrona**: Real chunks se generan en background threads (`AsyncComputeTaskPool`), sin bloquear el main thread.
- **Máx. 32 chunks por frame**: Evita spikes de carga.
- **Ordenados por distancia**: Los chunks más cercanos al jugador se priorizan.
- **Carga en círculo**: Solo itera posiciones dentro del radio circular, no el cuadrado completo.
- **Descarga limitada**: Máx. 16 chunks por frame para no spikear.

### Meshing
- **Greedy meshing**: 70-95% reducción de triángulos combinando caras adyacentes del mismo material.
- **Skips chunks vacíos**: No se crea collider ni entidad para chunks de solo aire.

### Sistema LOD
- **LodChunk**: Solo almacena la superficie (altura + tipo), no el volumen completo.
- **Conversión con histéresis**: Umbrales diferentes para LOD→Real (30) y Real→LOD (36) evitan thrashing.

---

## 🔴 Bugs de Rendimiento Pendientes (Alta Prioridad)

### 1. LOD chunks bloquean el main thread
**Problema**: `LodChunk::generate_surface()` + `mesh_lod_chunk()` corren en el main thread durante `load_chunks_system`. Con un radio LOD de hasta 200 chunks, esto causa stutters visibles al explorar.

**Solución**: Mover la generación de LOD chunks al mismo patrón async que los Real chunks — spawn una task en `AsyncComputeTaskPool`, pollear el resultado en un sistema separado.

**Archivo**: `src/voxel/chunk_loading.rs`, bloque `ChunkType::Lod` en `load_chunks_system()`.

### 2. Frustum culling no cubre LodChunks
**Problema**: `update_frustum_culling` solo hace query sobre `BaseChunk`. Los `LodChunk` nunca son culleados — el GPU renderiza todo el horizonte LOD aunque esté detrás del jugador.

**Solución**: Añadir un segundo query en `update_frustum_culling` para entidades con `LodChunk`, aplicando la misma lógica de distancia+ángulo.

**Archivo**: `src/voxel/frustum_culling.rs`.

---

## 🟡 Optimizaciones Futuras (Medio Plazo)

### Caché en Memoria
Guardar chunks ya generados en un `HashMap<IVec3, BaseChunk>` en lugar de regenerarlos al re-visitar áreas. Actualmente cada chunk se regenera desde cero al cargarse.

### Caché en Disco (Reescritura)
El sistema actual (`chunk_cache.rs`) usa I/O síncrono y está deshabilitado. Necesita reescritura con I/O asíncrono en batch (guardar al cerrar el juego, cargar al iniciar).

### Downsampling LOD (Corrección)
`downsampling.rs` tiene un bug de overflow. Una vez corregido, permitiría meshes de LOD más baratos que el sistema de superficie actual.

### Chunk Pooling
Reutilizar entidades/datos de chunks en lugar de despawnear y crear nuevos. Reduciría allocations y fragmentación de memoria.

---

## 🟢 Optimizaciones Futuras (Largo Plazo)

### GPU Instancing
Renderizar múltiples chunks con una sola draw call agrupando por LOD level. Potencial de 5-10x menos draw calls.

### Caché de Noise
Guardar valores de noise para posiciones (x,z) ya calculadas. Evitaría recalcular biomas al re-cargar chunks cercanos.

### Generación por Capas
Generar solo la superficie inicialmente y expandir hacia el interior solo si el jugador destruye voxels. Reduciría trabajo inicial de carga.

---

## 📊 Impacto Estimado

| Optimización | FPS esperado | Dificultad |
|---|---|---|
| LOD async (bug #1) | +10-20 FPS | Media |
| Frustum culling LOD (bug #2) | +5-15 FPS | Fácil |
| Caché en memoria | +5-10 FPS en áreas visitadas | Fácil |
| Chunk pooling | +3-8 FPS | Media |
| GPU instancing | +15-30 FPS | Alta |

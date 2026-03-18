# World Limits and Size Configuration

## Current Limits

### Octree Bounds (main.rs)
```rust
ChunkOctree::new(BoundingBox::new(
    IVec3::new(-200, -10, -200),  // Min
    IVec3::new(200, 10, 200),     // Max
))
```

**Current Size:**
- **Horizontal**: 400 chunks × 400 chunks = 160,000 chunks
- **Vertical**: 20 chunks height
- **In Meters**: 1,280m × 1,280m × 64m (1.28 km × 1.28 km)
- **Total Volume**: 3.2 million chunks possible

### Active Loading Area
- **Load Radius**: 64 chunks (~204 meters) — Real chunks with physics
- **LOD Radius**: up to 200 chunks — Surface-only LOD chunks, no physics
- **Vertical Range**: Y=-1 to Y=3 (5 levels, ~16 meters)
- **Active Real Chunks**: ~6,400 chunks max (π × 64²)
- **Unload Radius**: 70 chunks

## How to Change World Size

### Option 1: Larger World (Recommended)

For a world 10x larger (12.8 km × 12.8 km):

```rust
// In main.rs, change:
.insert_resource(ChunkOctree::new(BoundingBox::new(
    IVec3::new(-2000, -10, -2000),  // 10x larger
    IVec3::new(2000, 10, 2000),
)))
```

### Option 2: Taller World (Mountains/Caves)

For taller mountains and deeper caves:

```rust
// In main.rs, change:
.insert_resource(ChunkOctree::new(BoundingBox::new(
    IVec3::new(-200, -20, -200),   // Deeper caves
    IVec3::new(200, 20, 200),      // Taller mountains
)))

// In chunk_loading.rs, change:
let y_min = -5;  // 5 chunks below (16 meters underground)
let y_max = 10;  // 10 chunks above (32 meters high)
```

### Option 3: Near-Infinite World

For a massive world (320 km × 320 km):

```rust
// In main.rs:
.insert_resource(ChunkOctree::new(BoundingBox::new(
    IVec3::new(-100000, -50, -100000),
    IVec3::new(100000, 50, 100000),
)))
```

**Note**: Octree size doesn't affect performance much - only loaded chunks matter!

## Performance Considerations

### Memory Usage

**Per Chunk:**
- BaseChunk: ~32 KB (32³ voxels)
- Mesh: ~10-50 KB (depends on complexity)
- Collider: ~5-20 KB
- **Total**: ~50-100 KB per chunk

**Active Chunks (400 loaded):**
- Memory: ~20-40 MB
- Acceptable for most systems

**Large World (10,000 chunks loaded):**
- Memory: ~500 MB - 1 GB
- May need optimization

### Octree Performance

The octree is O(log n) for searches:
- 1,000 chunks: ~10 operations
- 1,000,000 chunks: ~20 operations
- 1,000,000,000 chunks: ~30 operations

**Conclusion**: Octree bounds can be HUGE without performance impact!

## Recommended Configurations

### Small World (Testing/Multiplayer)
```rust
// 640m × 640m (good for 10-20 players)
BoundingBox::new(
    IVec3::new(-100, -10, -100),
    IVec3::new(100, 10, 100),
)
```

### Medium World (Single Player)
```rust
// 3.2 km × 3.2 km (exploration game)
BoundingBox::new(
    IVec3::new(-500, -20, -500),
    IVec3::new(500, 20, 500),
)
```

### Large World (Open World)
```rust
// 32 km × 32 km (Minecraft-like)
BoundingBox::new(
    IVec3::new(-5000, -30, -5000),
    IVec3::new(5000, 30, 5000),
)
```

### Massive World (Near-Infinite)
```rust
// 320 km × 320 km (exploration/survival)
BoundingBox::new(
    IVec3::new(-50000, -50, -50000),
    IVec3::new(50000, 50, 50000),
)
```

## Vertical Limits

### Current Setup
- **Active Range**: Y=-1 to Y=3 (5 chunks, 16 meters)
- **Good for**: Rolling hills, small mountains

### For Tall Mountains
```rust
// In chunk_loading.rs:
let y_min = -2;   // 6.4m underground
let y_max = 8;    // 25.6m high mountains
```

### For Deep Caves
```rust
// In chunk_loading.rs:
let y_min = -10;  // 32m deep caves
let y_max = 5;    // 16m mountains
```

### For Both
```rust
// In chunk_loading.rs:
let y_min = -10;  // 32m deep
let y_max = 15;   // 48m high
// Note: This loads 25 vertical chunks = 5x more chunks!
```

## World Border Behavior

### Current Behavior
- **No hard border**: Chunks can generate outside octree bounds
- **Octree limitation**: Spatial queries only work within bounds
- **Recommendation**: Keep octree bounds larger than load radius

### Adding Hard Borders

To prevent generation outside bounds:

```rust
// In chunk_loading.rs, add check:
for chunk_pos in &chunks_needed {
    // Check if within world bounds
    if chunk_pos.x < -200 || chunk_pos.x > 200 ||
       chunk_pos.y < -10 || chunk_pos.y > 10 ||
       chunk_pos.z < -200 || chunk_pos.z > 200 {
        continue; // Skip chunks outside bounds
    }
    
    if !chunk_map.chunks.contains_key(chunk_pos) {
        load_queue.to_load.push(*chunk_pos);
    }
}
```

## Multiplayer Considerations

### Server World Size
- **Small Server (10 players)**: 1-2 km² is enough
- **Medium Server (50 players)**: 5-10 km²
- **Large Server (100+ players)**: 20+ km²

### Interest Management
- Each player loads chunks in their area
- Server tracks which chunks are "active" (have players nearby)
- Inactive chunks can be unloaded from server memory
- Use chunk streaming for very large worlds

## Future: Truly Infinite Worlds

For truly infinite worlds (like Minecraft):

1. **Remove octree bounds check**
2. **Use chunk streaming**: Load/unload from disk
3. **Use world regions**: Divide world into 1km² regions
4. **Procedural generation**: Generate on-demand
5. **Chunk caching**: Save to disk, load when needed

This requires:
- Efficient disk I/O (currently disabled)
- Chunk compression (LZ4)
- Region-based file storage
- Async loading/saving

## Summary

**Current Limits:**
- Octree: 1.28 km × 1.28 km × 64m
- Active: 51m radius, 16m height
- Can be easily increased!

**Recommended:**
- Set octree bounds 2-3x larger than max player distance
- Adjust vertical range based on terrain height
- For near-infinite: Use ±50,000 chunks (320 km)

**Performance:**
- Octree size doesn't affect FPS
- Only loaded chunks affect performance
- Current: ~400 chunks loaded = good performance

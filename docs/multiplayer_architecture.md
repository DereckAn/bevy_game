# Multiplayer Architecture for Voxel Game

## Overview

This document explains how the voxel game will work in multiplayer, focusing on the separation between client-side rendering and server-side simulation.

## Core Principle: Client-Server Separation

```
CLIENT (Rendering)          SERVER (Simulation)
├─ Frustum Culling         ├─ NO Frustum Culling
├─ LOD System              ├─ Full Resolution Only
├─ Render Distance         ├─ Interest Management
└─ Visual Optimizations    └─ Physics & Logic
```

## 1. Frustum Culling (CLIENT-SIDE ONLY)

### What It Does
- Hides chunks that are **outside the camera's view**
- Improves FPS by reducing rendering workload
- Chunks still exist in memory, just not rendered

### Why Client-Side Only?
- Each player has their own camera
- Each player sees different things
- Server doesn't have a "camera" - it simulates everything

### Implementation
```rust
// CLIENT: Frustum culling system
pub fn update_frustum_culling(
    camera_query: Query<&Transform, With<Camera>>,
    mut chunk_query: Query<(&BaseChunk, &mut Visibility)>,
) {
    // Hide chunks outside camera view
    // Only affects rendering, not simulation
}
```

## 2. Server Architecture

### Interest Management (NOT Frustum Culling)

The server uses **distance-based interest management**:

```rust
// SERVER: Interest management
pub fn update_player_interest_areas(
    players: Query<(&PlayerId, &Transform)>,
    chunks: Res<ChunkManager>,
) {
    for (player_id, transform) in players.iter() {
        // Load chunks within INTEREST_RADIUS (e.g., 32 chunks)
        let interest_area = calculate_interest_area(transform.translation);
        
        // Send chunk updates to this player
        for chunk_pos in interest_area {
            if let Some(chunk) = chunks.get(chunk_pos) {
                send_chunk_to_player(player_id, chunk);
            }
        }
    }
}
```

### Key Differences

| Aspect | Client Frustum Culling | Server Interest Management |
|--------|------------------------|----------------------------|
| **Based On** | Camera direction | Player position |
| **Shape** | Pyramid (view cone) | Sphere (radius) |
| **Purpose** | Rendering optimization | Network optimization |
| **Affects** | What you SEE | What you RECEIVE |

### Example Scenario

```
Player at position (0, 0, 0) looking NORTH:

CLIENT (Frustum Culling):
✓ Renders chunks to the NORTH (in view)
✗ Hides chunks to the SOUTH (behind camera)
✗ Hides chunks to the EAST/WEST (outside FOV)

SERVER (Interest Management):
✓ Sends chunks in ALL directions within 32 chunk radius
✓ Includes chunks behind player (they might turn around)
✓ Simulates physics in all these chunks
```

## 3. Chunk Synchronization Flow

```
1. SERVER: Player joins
   └─> Load chunks in 32-chunk radius around player
   
2. SERVER: Send chunks to client
   └─> Compress chunk data
   └─> Send via network
   
3. CLIENT: Receive chunks
   └─> Decompress and store in memory
   └─> Generate meshes
   
4. CLIENT: Render chunks
   └─> Apply frustum culling
   └─> Only render visible chunks
   
5. PLAYER: Breaks a voxel
   └─> CLIENT: Send action to server
   └─> SERVER: Validate action
   └─> SERVER: Update chunk
   └─> SERVER: Broadcast to nearby players
   └─> CLIENTS: Update their local chunks
```

## 4. Optimization Strategies

### Client Optimizations
- **Frustum Culling**: Hide chunks outside view (50-75% FPS boost)
- **LOD System**: Lower detail for distant chunks
- **Mesh Pooling**: Reuse mesh allocations
- **Async Loading**: Generate chunks in background

### Server Optimizations
- **Interest Management**: Only send relevant chunks to each player
- **Chunk Pooling**: Reuse chunk allocations
- **Delta Updates**: Only send changed voxels, not entire chunks
- **Compression**: LZ4 compress chunk data before sending
- **Priority System**: Send chunks closer to player first

### Network Optimizations
- **Chunk Compression**: Reduce bandwidth
- **Delta Encoding**: Only send changes
- **Batching**: Group multiple updates
- **Prediction**: Client predicts voxel breaks before server confirms

## 5. Example Multiplayer Setup

### Server Code
```rust
// Server doesn't use frustum culling
fn setup_server(mut app: App) {
    app
        .add_systems(Update, (
            update_player_interest_areas,
            simulate_chunk_physics,
            handle_voxel_modifications,
            broadcast_chunk_updates,
        ))
        // NO frustum culling system
        // NO rendering systems
}
```

### Client Code
```rust
// Client uses frustum culling
fn setup_client(mut app: App) {
    app
        .add_systems(Update, (
            receive_chunks_from_server,
            update_frustum_culling,  // CLIENT ONLY
            update_chunk_lod,         // CLIENT ONLY
            render_visible_chunks,    // CLIENT ONLY
            send_player_actions,
        ))
}
```

## 6. Handling Edge Cases

### Player Turns Around Quickly
- **Problem**: Chunks behind player weren't loaded
- **Solution**: Server sends chunks in ALL directions (interest radius)
- **Client**: Chunks already in memory, just need to unhide them

### Player Moves Fast
- **Problem**: New chunks need to load quickly
- **Solution**: 
  - Server: Prioritize chunks in movement direction
  - Client: Async loading + show low-res LOD first

### Multiple Players in Same Area
- **Problem**: Server needs to simulate many chunks
- **Solution**:
  - Merge interest areas (don't duplicate work)
  - Prioritize chunks with more players
  - Use spatial partitioning (octree)

## 7. Performance Targets

### Client (Single Player)
- **Target FPS**: 60-144 FPS
- **Render Distance**: 16-32 chunks
- **Visible Chunks**: ~50-60% of loaded (frustum culling)

### Server (10 Players)
- **Target TPS**: 20-60 ticks per second
- **Active Chunks**: ~500-1000 chunks
- **Network**: <100 KB/s per player

### Server (100 Players)
- **Target TPS**: 20 ticks per second
- **Active Chunks**: ~3000-5000 chunks
- **Network**: <50 KB/s per player (aggressive compression)

## 8. Future Enhancements

### Advanced Client Features
- **Occlusion Culling**: Hide chunks behind mountains
- **Portal Culling**: Only render visible through doorways
- **Temporal Coherence**: Reuse previous frame data

### Advanced Server Features
- **Chunk Streaming**: Load/unload chunks dynamically
- **Distributed Servers**: Multiple servers for large worlds
- **Chunk Caching**: Redis/database for persistent worlds

## Summary

**Frustum Culling = Client-Side Rendering Optimization**
- Each client decides what to render based on their camera
- Doesn't affect gameplay or physics
- Essential for good FPS

**Interest Management = Server-Side Network Optimization**
- Server decides what chunks to send to each player
- Based on distance, not view direction
- Essential for good network performance

**They work together but serve different purposes!**

---

**Key Takeaway**: Frustum culling is perfectly fine for multiplayer because it's a client-side rendering optimization. The server doesn't care what the client renders - it just simulates the world and sends relevant data.

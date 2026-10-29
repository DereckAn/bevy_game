//! Terreno lejano trazado por rayos (fase 4 de `docs/raymarch_far_terrain_plan.md`).
//!
//! Anillos de teselas de 64×64 columnas alrededor del jugador (clipmap): cada
//! nivel dobla el tamaño de celda del anterior, así el número de teselas por
//! nivel es constante y la distancia de visión crece exponencialmente. Cada
//! tesela es una caja cuyo shader (`assets/shaders/far_terrain.wgsl`, port de
//! `raymarch::trace`) traza el rayo de cada píxel por sus columnas.
//!
//! Las teselas se generan en el pool asíncrono y nacen ocultas. Cada grupo de
//! teselas que se solapan (nuevas + las viejas a las que reemplazan) se
//! intercambia en un mismo frame, y solo cuando todas sus nuevas están
//! preparadas en la GPU: nunca hay hueco ni doble cobertura.
//! F6 tiñe las teselas por nivel.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey, PreparedMaterial};
use bevy::prelude::*;
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::mesh::RenderMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, Face, RenderPipelineDescriptor, ShaderType,
    SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::render::{Render, RenderApp, RenderSystems};
use bevy::shader::ShaderRef;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

use crate::core::{BASE_CHUNK_SIZE, GameState, VOXEL_SIZE, WORLD_CHUNK_RADIUS, WorldKind, WorldSeed};
use crate::player::Player;
use crate::voxel::raymarch::{TileHeights, MIP_COUNT, TILE_CELLS};
use crate::voxel::{voxel_color, TerrainGenerator, VoxelType, CHUNK_LOAD_RADIUS};

const SHADER_PATH: &str = "shaders/far_terrain.wgsl";
/// Celda del nivel 0 (m): 4 voxels. El nivel k usa `0.4 × 2^k`.
const LEVEL0_CELL_M: f32 = 0.4;
/// Niveles del clipmap: el último (celda de 51 m) llega a ~30 km.
const LEVELS: usize = 8;
/// Cada nivel cubre [ancla − R, ancla + R) teselas. Par, para que los bordes
/// caigan sobre teselas enteras del nivel siguiente.
const RING_RADIUS: i32 = 10;
/// Teselas generándose a la vez: el pool asíncrono lo comparte la carga de chunks.
const MAX_TASKS_IN_FLIGHT: usize = 32;

pub struct FarTerrainPlugin;

impl Plugin for FarTerrainPlugin {
    fn build(&self, app: &mut App) {
        let readiness = FarReadiness::default();
        app.add_plugins(MaterialPlugin::<FarTileMaterial>::default())
            .init_resource::<FarTiles>()
            .init_resource::<FarTint>()
            .insert_resource(readiness.clone())
            .add_systems(
                Update,
                (
                    update_far_rings,
                    stream_far_tiles,
                    swap_far_tiles,
                    hide_far_tiles_under_near_field,
                    toggle_far_tint,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(OnEnter(GameState::MainMenu), teardown_far_tiles);
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(readiness)
                .add_systems(Render, mark_ready_far_tiles.in_set(RenderSystems::Cleanup));
        }
    }
}

/// Una tesela: nivel + índice en la rejilla de teselas de ese nivel.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct TileKey {
    level: usize,
    idx: IVec2,
}

#[derive(Component)]
pub struct FarTile;

struct TileEntry {
    entity: Entity,
    material: AssetId<FarTileMaterial>,
    /// Visible (cubre su zona). Mientras es `false` la tesela existe oculta,
    /// esperando a que su grupo de intercambio esté listo.
    is_live: bool,
}

#[derive(Resource, Default)]
struct FarTiles {
    anchors: Option<[IVec2; LEVELS]>,
    desired: HashSet<TileKey>,
    queue: VecDeque<TileKey>,
    pending: HashMap<TileKey, Task<TileHeights>>,
    tiles: HashMap<TileKey, TileEntry>,
}

/// Qué teselas tienen material (y por tanto textura) y malla preparados en el
/// mundo de render. Compartido entre ambos mundos, que corren en paralelo.
#[derive(Resource, Clone, Default)]
struct FarReadiness(Arc<Mutex<Readiness>>);

#[derive(Default)]
struct Readiness {
    watching: Vec<(AssetId<FarTileMaterial>, AssetId<Mesh>)>,
    ready: HashSet<AssetId<FarTileMaterial>>,
}

impl Readiness {
    fn forget(&mut self, material: AssetId<FarTileMaterial>) {
        self.watching.retain(|&(watched, _)| watched != material);
        self.ready.remove(&material);
    }
}

/// F6: teñir las teselas por nivel (depuración).
#[derive(Resource, Default)]
struct FarTint(bool);

/// Uniform de una tesela. Alturas y caja en unidades de celda, como en `trace`.
#[derive(Clone, ShaderType)]
pub struct FarTileUniform {
    /// xyz = esquina mínima de la tesela en el mundo (m), w = tamaño de celda (m).
    pub origin: Vec4,
    /// x = fondo de la caja, y = techo de la caja (celdas), z = nivel,
    /// w = 1 si se tiñe por nivel (F6).
    pub bounds: Vec4,
    /// Material de la superficie: rgb = color base LINEAL (blanco para el pasto,
    /// cuyo verde pone la paleta), w = discriminante de `VoxelType` (0..255),
    /// igual que el vertex color del terreno cercano.
    // NOTE: uno por tesela porque hoy `WorldKind` es global; pasar a textura
    // por celda cuando lleguen los biomas en mosaico.
    pub surface: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct FarTileMaterial {
    // Bindings 100+: `pbr_functions` importa `pbr_bindings`, que declara los
    // del StandardMaterial desde el 0 en este mismo grupo.
    #[uniform(100)]
    pub tile: FarTileUniform,
    /// `R32Float` 64×64 con la cadena de máximos en sus mips (se lee con
    /// `textureLoad`, sin sampler: R32Float no es filtrable).
    #[texture(101, sample_type = "float", filterable = false)]
    pub heights: Handle<Image>,
}

impl Material for FarTileMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    /// Caras TRASERAS de la caja: el shader calcula la entrada del rayo, así que
    /// sigue funcionando aunque la cámara quede dentro de la caja.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

/// Recalcula el conjunto de teselas cuando el ancla de algún nivel cambia,
/// descarta el trabajo que ya no hace falta y encola lo que falta (las finas
/// primero: son las más cercanas).
fn update_far_rings(
    mut commands: Commands,
    player: Query<&Transform, With<Player>>,
    mut far: ResMut<FarTiles>,
    readiness: Res<FarReadiness>,
) {
    let Ok(transform) = player.single() else {
        return;
    };
    let anchors = level_anchors(transform.translation.xz());
    if far.anchors == Some(anchors) {
        return;
    }
    let FarTiles { anchors: old_anchors, desired, queue, pending, tiles } = &mut *far;
    *old_anchors = Some(anchors);
    *desired = desired_tiles(&anchors);
    // Soltar una tarea la cancela.
    pending.retain(|key, _| desired.contains(key));
    // Las que nunca llegaron a mostrarse y ya no hacen falta se van sin más; las
    // vivas las retira `swap_far_tiles` cuando su reemplazo esté listo.
    let mut readiness = readiness.0.lock().unwrap();
    tiles.retain(|key, tile| {
        let keep = tile.is_live || desired.contains(key);
        if !keep {
            commands.entity(tile.entity).despawn();
            readiness.forget(tile.material);
        }
        keep
    });
    let mut missing: Vec<TileKey> = desired
        .iter()
        .filter(|key| !tiles.contains_key(key) && !pending.contains_key(key))
        .copied()
        .collect();
    missing.sort_by_key(|key| (key.level, (key.idx - anchors[key.level]).length_squared()));
    *queue = missing.into();
}

/// Lanza tareas de generación (acotadas) y crea, ocultas, las teselas cuyas
/// alturas ya están; desde ese momento se vigila su preparación en la GPU.
#[allow(clippy::too_many_arguments)]
fn stream_far_tiles(
    mut commands: Commands,
    mut far: ResMut<FarTiles>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FarTileMaterial>>,
    world_seed: Res<WorldSeed>,
    world_kind: Res<WorldKind>,
    tint: Res<FarTint>,
    readiness: Res<FarReadiness>,
) {
    let (seed, kind) = (world_seed.0, *world_kind);
    let pool = AsyncComputeTaskPool::get();
    while far.pending.len() < MAX_TASKS_IN_FLIGHT {
        let Some(key) = far.queue.pop_front() else {
            break;
        };
        far.pending
            .insert(key, pool.spawn(async move { generate_tile(key, seed, kind) }));
    }

    let finished: Vec<(TileKey, TileHeights)> = far
        .pending
        .iter_mut()
        .filter_map(|(key, task)| future::block_on(future::poll_once(task)).map(|tile| (*key, tile)))
        .collect();
    let surface_type = VoxelType::from_depth(1.0, 0.0, kind);
    for (key, tile) in finished {
        far.pending.remove(&key);
        let (entity, material, mesh) = spawn_tile(
            &mut commands,
            &mut meshes,
            &mut images,
            &mut materials,
            key,
            &tile,
            surface_type,
            tint.0,
        );
        readiness.0.lock().unwrap().watching.push((material, mesh));
        far.tiles.insert(key, TileEntry { entity, material, is_live: false });
    }
}

/// Mundo de render, al final del frame: marca listas las teselas cuyo material
/// ya está preparado (Bevy solo lo prepara cuando su textura está en la GPU) y
/// cuya malla también.
fn mark_ready_far_tiles(
    readiness: Res<FarReadiness>,
    materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    meshes: Res<RenderAssets<RenderMesh>>,
) {
    let mut readiness = readiness.0.lock().unwrap();
    let Readiness { watching, ready } = &mut *readiness;
    watching.retain(|&(material, mesh)| {
        let is_ready = materials.get(material).is_some() && meshes.get(mesh).is_some();
        if is_ready {
            ready.insert(material);
        }
        !is_ready
    });
}

/// Aplica los intercambios que `plan_swaps` da por listos: en el mismo frame se
/// muestran las nuevas y se despawnean las viejas de cada grupo.
fn swap_far_tiles(
    mut commands: Commands,
    mut far: ResMut<FarTiles>,
    readiness: Res<FarReadiness>,
    mut visibility: Query<&mut Visibility, With<FarTile>>,
) {
    let FarTiles { desired, tiles, .. } = &mut *far;
    let live: HashSet<TileKey> = tiles.iter().filter(|(_, tile)| tile.is_live).map(|(key, _)| *key).collect();
    if live == *desired {
        return;
    }
    let mut readiness = readiness.0.lock().unwrap();
    let (show, retire) = plan_swaps(&live, desired, |key| {
        tiles.get(key).is_some_and(|tile| readiness.ready.contains(&tile.material))
    });
    for key in show {
        if let Some(tile) = tiles.get_mut(&key) {
            tile.is_live = true;
            if let Ok(mut tile_visibility) = visibility.get_mut(tile.entity) {
                *tile_visibility = Visibility::Inherited;
            }
        }
    }
    for key in retire {
        if let Some(tile) = tiles.remove(&key) {
            commands.entity(tile.entity).despawn();
            readiness.forget(tile.material);
        }
    }
}

/// Provisional hasta la máscara de cobertura (fase 5, plan §1.6): oculta las
/// teselas vivas de nivel 0 que caen enteras dentro del campo cercano (LOD +
/// chunks reales, con un chunk de margen) y dentro del mundo, para que no se
/// dibujen encima del terreno cercano.
fn hide_far_tiles_under_near_field(
    player: Query<&Transform, With<Player>>,
    far: Res<FarTiles>,
    mut visibility: Query<&mut Visibility, With<FarTile>>,
) {
    let Ok(transform) = player.single() else {
        return;
    };
    let chunk_m = BASE_CHUNK_SIZE as f32 * VOXEL_SIZE;
    let near_radius = (CHUNK_LOAD_RADIUS - 1) as f32 * chunk_m;
    let world_half = WORLD_CHUNK_RADIUS as f32 * chunk_m;
    let player_xz = transform.translation.xz();
    for (key, tile) in &far.tiles {
        if key.level != 0 || !tile.is_live {
            continue;
        }
        let min = key.idx.as_vec2() * tile_m(0);
        let corners = [min, min + Vec2::X * tile_m(0), min + Vec2::Y * tile_m(0), min + tile_m(0)];
        let is_covered = corners.iter().all(|c| {
            c.distance(player_xz) <= near_radius && c.abs().max_element() <= world_half
        });
        let wanted = if is_covered { Visibility::Hidden } else { Visibility::Inherited };
        if let Ok(mut tile_visibility) = visibility.get_mut(tile.entity) {
            if *tile_visibility != wanted {
                *tile_visibility = wanted;
            }
        }
    }
}

fn toggle_far_tint(
    keys: Res<ButtonInput<KeyCode>>,
    mut tint: ResMut<FarTint>,
    tiles: Query<&MeshMaterial3d<FarTileMaterial>>,
    mut materials: ResMut<Assets<FarTileMaterial>>,
) {
    if !keys.just_pressed(KeyCode::F6) {
        return;
    }
    tint.0 = !tint.0;
    for handle in &tiles {
        if let Some(material) = materials.get_mut(handle) {
            material.tile.bounds.w = tint.0 as u8 as f32;
        }
    }
    info!("Teñido por nivel del terreno lejano: {}", tint.0);
}

fn teardown_far_tiles(
    mut commands: Commands,
    mut far: ResMut<FarTiles>,
    readiness: Res<FarReadiness>,
    tiles: Query<Entity, With<FarTile>>,
) {
    for entity in &tiles {
        commands.entity(entity).despawn();
    }
    // Soltar las tareas en vuelo las cancela.
    *far = FarTiles::default();
    *readiness.0.lock().unwrap() = Readiness::default();
}

fn cell_m(level: usize) -> f32 {
    LEVEL0_CELL_M * (1u32 << level) as f32
}

fn tile_m(level: usize) -> f32 {
    cell_m(level) * TILE_CELLS as f32
}

/// Ancla de cada nivel, en teselas de ese nivel: el jugador redondeado a la
/// rejilla del nivel SIGUIENTE (×2), así el borde de cada nivel cae sobre
/// teselas enteras del de fuera y su hueco encaja sin solaparse.
fn level_anchors(player_xz: Vec2) -> [IVec2; LEVELS] {
    std::array::from_fn(|k| (player_xz / tile_m(k + 1)).round().as_ivec2() * 2)
}

/// Teselas que deben existir: el nivel 0 entero y, en los demás, el anillo sin
/// el cuadrado que ya cubre el nivel de dentro.
fn desired_tiles(anchors: &[IVec2; LEVELS]) -> HashSet<TileKey> {
    let mut tiles = HashSet::new();
    for (level, &anchor) in anchors.iter().enumerate() {
        let hole = (level > 0).then(|| {
            let inner = anchors[level - 1];
            ((inner - RING_RADIUS) / 2, (inner + RING_RADIUS) / 2)
        });
        for z in anchor.y - RING_RADIUS..anchor.y + RING_RADIUS {
            for x in anchor.x - RING_RADIUS..anchor.x + RING_RADIUS {
                let idx = IVec2::new(x, z);
                let is_in_hole = hole.is_some_and(|(lo, hi)| idx.cmpge(lo).all() && idx.cmplt(hi).all());
                if !is_in_hole {
                    tiles.insert(TileKey { level, idx });
                }
            }
        }
    }
    tiles
}

/// Qué teselas mostrar y cuáles retirar este frame. Las que cambian (entrantes:
/// deseadas y no vivas; salientes: vivas y no deseadas) se agrupan por solape
/// —con rejillas alineadas, una fina solo solapa con la gruesa que la contiene—
/// y cada grupo se intercambia entero, solo cuando todas sus entrantes están
/// listas. Como las vivas y las deseadas son, cada una, una partición sin
/// solapes, intercambiar grupos cerrados mantiene la cobertura exacta.
fn plan_swaps(
    live: &HashSet<TileKey>,
    desired: &HashSet<TileKey>,
    is_ready: impl Fn(&TileKey) -> bool,
) -> (Vec<TileKey>, Vec<TileKey>) {
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }

    let changed: Vec<TileKey> = desired.difference(live).chain(live.difference(desired)).copied().collect();
    let index: HashMap<TileKey, usize> = changed.iter().enumerate().map(|(i, key)| (*key, i)).collect();
    let mut parent: Vec<usize> = (0..changed.len()).collect();
    for (i, key) in changed.iter().enumerate() {
        for level in key.level + 1..LEVELS {
            let scale = IVec2::splat(1 << (level - key.level));
            let ancestor = TileKey { level, idx: key.idx.div_euclid(scale) };
            if let Some(&j) = index.get(&ancestor) {
                let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                parent[a] = b;
            }
        }
    }

    let mut blocked = HashSet::new();
    for (i, key) in changed.iter().enumerate() {
        if desired.contains(key) && !is_ready(key) {
            blocked.insert(root(&mut parent, i));
        }
    }
    let (mut show, mut retire) = (Vec::new(), Vec::new());
    for (i, key) in changed.iter().enumerate() {
        if blocked.contains(&root(&mut parent, i)) {
            continue;
        }
        if desired.contains(key) {
            show.push(*key);
        } else {
            retire.push(*key);
        }
    }
    (show, retire)
}

fn generate_tile(key: TileKey, seed: i32, kind: WorldKind) -> TileHeights {
    let cell = cell_m(key.level);
    let origin = key.idx.as_vec2() * tile_m(key.level);
    let mut terrain_gen = TerrainGenerator::new(seed, kind);
    build_tile(|x, z| {
        terrain_gen.biome_gen.generate_height(origin.x + x * cell, origin.y + z * cell) / cell
    })
}

/// Alturas de la tesela (centro de cada columna, redondeadas a celdas enteras:
/// columnas de tope plano) con el fondo de la caja bajado hasta los vecinos.
/// `height_at(x, z)` en celdas, con x/z en celdas desde la esquina de la tesela.
fn build_tile(mut height_at: impl FnMut(f32, f32) -> f32) -> TileHeights {
    let heights = (0..TILE_CELLS * TILE_CELLS)
        .map(|i| height_at((i % TILE_CELLS) as f32 + 0.5, (i / TILE_CELLS) as f32 + 0.5).round())
        .collect();
    let mut tile = TileHeights::new(heights);
    let border_min = neighbour_centres()
        .map(|(x, z)| height_at(x, z))
        .fold(f32::INFINITY, f32::min);
    tile.lower_bottom_to(border_min);
    tile
}

/// Centros de las columnas vecinas junto al borde, en celdas de esta tesela, para
/// los tres vecinos posibles (rejillas alineadas): más fino (celda 0.5, centros
/// en 0.25 + 0.5·i), mismo nivel (0.5 + i) y más grueso (celda 2, centros en
/// impares). Incluye los de las esquinas.
fn neighbour_centres() -> impl Iterator<Item = (f32, f32)> {
    let n = TILE_CELLS as f32;
    // (media celda del vecino = distancia de su centro al borde, celda del vecino)
    [(0.25f32, 0.5f32), (0.5, 1.0), (1.0, 2.0)]
        .into_iter()
        .flat_map(move |(half, size)| {
            (-1..=(n / size) as i32).flat_map(move |i| {
                let t = half + i as f32 * size;
                [(t, -half), (t, n + half), (-half, t), (n + half, t)]
            })
        })
}

#[allow(clippy::too_many_arguments)]
fn spawn_tile(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    images: &mut Assets<Image>,
    materials: &mut Assets<FarTileMaterial>,
    key: TileKey,
    tile: &TileHeights,
    surface_type: VoxelType,
    is_tinted: bool,
) -> (Entity, AssetId<FarTileMaterial>, AssetId<Mesh>) {
    let cell = cell_m(key.level);
    let size = tile_m(key.level);
    let top = tile.mips[MIP_COUNT - 1][0];
    let origin = Vec3::new(key.idx.x as f32 * size, 0.0, key.idx.y as f32 * size);
    let center = origin + Vec3::new(size * 0.5, (tile.bottom + top) * 0.5 * cell, size * 0.5);
    let [r, g, b, _] = voxel_color(surface_type);

    let material = materials.add(FarTileMaterial {
        tile: FarTileUniform {
            origin: origin.extend(cell),
            bounds: Vec4::new(tile.bottom, top, key.level as f32, is_tinted as u8 as f32),
            surface: Vec4::new(r, g, b, surface_type as u8 as f32),
        },
        heights: images.add(heights_image(tile)),
    });
    let mesh = meshes.add(Cuboid::new(size, (top - tile.bottom) * cell, size));
    let ids = (material.id(), mesh.id());

    let entity = commands
        .spawn((
            FarTile,
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(center),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
        ))
        .id();
    (entity, ids.0, ids.1)
}

/// Textura `R32Float` con un mip por nivel de la cadena de máximos, en orden
/// (Bevy sube todos los mips de `data` si `mip_level_count` lo indica).
fn heights_image(tile: &TileHeights) -> Image {
    let size = Extent3d {
        width: TILE_CELLS as u32,
        height: TILE_CELLS as u32,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_uninit(
        size,
        TextureDimension::D2,
        TextureFormat::R32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = MIP_COUNT as u32;
    image.data = Some(tile.mips.iter().flatten().flat_map(|h| h.to_le_bytes()).collect());
    image
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::raymarch::trace;

    const PLAYER: Vec2 = Vec2::new(1234.5, -987.6);

    /// Cuántas teselas de `tiles` contienen el punto `p` (una por nivel como mucho).
    fn tiles_containing(tiles: &HashSet<TileKey>, p: Vec2) -> usize {
        (0..LEVELS)
            .filter(|&level| {
                let idx = (p / tile_m(level)).floor().as_ivec2();
                tiles.contains(&TileKey { level, idx })
            })
            .count()
    }

    /// Puntos de una rejilla de `steps`² sobre un cuadrado de lado 2·`half` m
    /// alrededor de `center` (offsets "raros" para no caer en bordes de tesela).
    fn grid_around(center: Vec2, half: f32, steps: i32) -> Vec<Vec2> {
        let step = 2.0 * half / steps as f32;
        (0..steps * steps)
            .map(|i| center - half + Vec2::new((i % steps) as f32 + 0.37, (i / steps) as f32 + 0.61) * step)
            .collect()
    }

    fn points_not_covered_exactly_once(half: f32, steps: i32) -> usize {
        let tiles = desired_tiles(&level_anchors(PLAYER));
        grid_around(PLAYER, half, steps)
            .into_iter()
            .filter(|&p| tiles_containing(&tiles, p) != 1)
            .count()
    }

    /// Simula que el jugador pasa de `from` a `to` y las teselas nuevas van
    /// estando listas en tandas de `batch` (orden mezclado fijo). Tras cada
    /// frame cuenta los puntos cubiertos por más de una tesela viva, o por
    /// ninguna donde tanto la configuración vieja como la nueva los cubren. Suma
    /// 1 si al final las vivas no son exactamente las deseadas.
    fn coverage_errors_during_move(from: Vec2, to: Vec2, points: &[Vec2], batch: usize) -> usize {
        let old = desired_tiles(&level_anchors(from));
        let desired = desired_tiles(&level_anchors(to));
        let mut live = old.clone();
        let mut incoming: Vec<TileKey> = desired.difference(&old).copied().collect();
        incoming.sort_by_key(|k| (k.idx.x * 7919 + k.idx.y * 104_729 + k.level as i32 * 31).rem_euclid(97));
        let mut ready = HashSet::new();
        let mut errors = 0;
        for tiles in incoming.chunks(batch) {
            ready.extend(tiles.iter().copied());
            let (show, retire) = plan_swaps(&live, &desired, |k| ready.contains(k));
            for key in retire {
                live.remove(&key);
            }
            live.extend(show);
            errors += points
                .iter()
                .filter(|&&p| {
                    let n = tiles_containing(&live, p);
                    let is_owed = tiles_containing(&old, p) == 1 && tiles_containing(&desired, p) == 1;
                    n > 1 || (n == 0 && is_owed)
                })
                .count();
        }
        errors + usize::from(live != desired)
    }

    #[test]
    fn near_ground_is_covered_by_exactly_one_tile() {
        assert_eq!(points_not_covered_exactly_once(300.0, 160), 0);
    }

    #[test]
    fn ground_out_to_29_km_is_covered_by_exactly_one_tile() {
        // Alcance mínimo garantizado: (R − 1) teselas del último nivel.
        let reach = (RING_RADIUS - 1) as f32 * tile_m(LEVELS - 1);
        assert_eq!(points_not_covered_exactly_once(reach, 280), 0);
    }

    #[test]
    fn level_zero_covers_the_whole_near_field() {
        // El campo cercano llega a 70 chunks (descarga de LODs) = 224 m. Peor
        // caso de un cuadrado: en la dirección de un eje.
        let tiles = desired_tiles(&level_anchors(PLAYER));
        let edges = [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y].map(|d| PLAYER + d * 224.0);
        let level0_idx = |p: Vec2| (p / tile_m(0)).floor().as_ivec2();
        assert!(edges.iter().all(|&p| tiles.contains(&TileKey { level: 0, idx: level0_idx(p) })));
    }

    #[test]
    fn short_move_swaps_tiles_without_overlap_or_holes() {
        // El caso de la revisión: de x = 0 a x = 26 cambia el ancla del nivel 0.
        let points = grid_around(Vec2::ZERO, 600.0, 150);
        assert_eq!(coverage_errors_during_move(Vec2::ZERO, Vec2::new(26.0, 0.0), &points, 3), 0);
    }

    #[test]
    fn long_move_swaps_every_level_without_overlap_or_holes() {
        let (from, to) = (Vec2::ZERO, Vec2::new(7000.0, -3000.0));
        let points = grid_around(Vec2::ZERO, 25_000.0, 150);
        assert_eq!(coverage_errors_during_move(from, to, &points, 40), 0);
    }

    #[test]
    fn side_reaches_down_to_finer_neighbour_column_centres() {
        // Vecino más fino al −X con columnas a 10 justo en sus centros
        // (x = −0.25, z = 0.25 + 0.5·i); todo lo demás a 100.
        let tile = build_tile(|x, z| {
            let is_finer_centre = x == -0.25 && (z * 4.0).rem_euclid(2.0) == 1.0;
            if is_finer_centre { 10.0 } else { 100.0 }
        });
        let hit = trace(&tile, Vec3::new(-5.0, 10.5, 20.25), Vec3::X);
        assert_eq!(hit.map(|h| h.normal), Some(IVec3::NEG_X));
    }
}

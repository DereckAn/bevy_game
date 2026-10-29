//! Terreno lejano trazado por rayos (fase 2 de `docs/raymarch_far_terrain_plan.md`).
//!
//! Por ahora UNA tesela estática de nivel 0 en la banda LOD, para comprobar el
//! shader (`assets/shaders/far_terrain.wgsl`, port de `raymarch::trace`) contra
//! la malla LOD: F4 oculta las teselas LOD y deja la lejana sola. Color plano de
//! depuración; la iluminación y la niebla llegan en la fase 3.

use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, Face, RenderPipelineDescriptor, ShaderType,
    SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;

use crate::core::{GameState, WorldKind, WorldSeed};
use crate::voxel::raymarch::{TileHeights, MIP_COUNT, TILE_CELLS};
use crate::voxel::TerrainGenerator;

const SHADER_PATH: &str = "shaders/far_terrain.wgsl";
/// Celda del nivel 0 (m): 4 voxels.
const LEVEL0_CELL_M: f32 = 0.4;
/// Esquina de la tesela de prueba: 40 chunks al +X del origen (128 m), dentro de
/// la banda LOD (102–205 m) y alineada a la rejilla de chunks.
const TEST_TILE_ORIGIN: Vec3 = Vec3::new(128.0, 0.0, -12.8);

pub struct FarTerrainPlugin;

impl Plugin for FarTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FarTileMaterial>::default())
            .add_systems(
                OnTransition {
                    exited: GameState::MainMenu,
                    entered: GameState::InGame,
                },
                spawn_test_far_tile,
            )
            .add_systems(OnEnter(GameState::MainMenu), despawn_far_tiles);
    }
}

#[derive(Component)]
pub struct FarTile;

/// Uniform de una tesela. Alturas y caja en unidades de celda, como en `trace`.
#[derive(Clone, ShaderType)]
pub struct FarTileUniform {
    /// xyz = esquina mínima de la tesela en el mundo (m), w = tamaño de celda (m).
    pub origin: Vec4,
    /// x = fondo de la caja, y = techo de la caja (celdas).
    pub bounds: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct FarTileMaterial {
    #[uniform(0)]
    pub tile: FarTileUniform,
    /// `R32Float` 64×64 con la cadena de máximos en sus mips (se lee con
    /// `textureLoad`, sin sampler: R32Float no es filtrable).
    #[texture(1, sample_type = "float", filterable = false)]
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

fn spawn_test_far_tile(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FarTileMaterial>>,
    world_seed: Res<WorldSeed>,
    world_kind: Res<WorldKind>,
) {
    let cell = LEVEL0_CELL_M;
    let mut terrain_gen = TerrainGenerator::new(world_seed.0, *world_kind);
    // Altura del centro de cada celda, redondeada a celdas enteras: columnas de
    // tope plano, el aspecto de voxel grande.
    let heights = (0..TILE_CELLS * TILE_CELLS)
        .map(|i| {
            let x = TEST_TILE_ORIGIN.x + ((i % TILE_CELLS) as f32 + 0.5) * cell;
            let z = TEST_TILE_ORIGIN.z + ((i / TILE_CELLS) as f32 + 0.5) * cell;
            (terrain_gen.biome_gen.generate_height(x, z) / cell).round()
        })
        .collect();
    let tile = TileHeights::new(heights);
    let top = tile.mips[MIP_COUNT - 1][0];

    let size = TILE_CELLS as f32 * cell;
    let box_height = (top - tile.bottom) * cell;
    let center =
        TEST_TILE_ORIGIN + Vec3::new(size * 0.5, (tile.bottom + top) * 0.5 * cell, size * 0.5);

    let material = materials.add(FarTileMaterial {
        tile: FarTileUniform {
            origin: TEST_TILE_ORIGIN.extend(cell),
            bounds: Vec4::new(tile.bottom, top, 0.0, 0.0),
        },
        heights: images.add(heights_image(&tile)),
    });

    commands.spawn((
        FarTile,
        Mesh3d(meshes.add(Cuboid::new(size, box_height, size))),
        MeshMaterial3d(material),
        Transform::from_translation(center),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

fn despawn_far_tiles(mut commands: Commands, tiles: Query<Entity, With<FarTile>>) {
    for entity in &tiles {
        commands.entity(entity).despawn();
    }
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
    image.data = Some(
        tile.mips
            .iter()
            .flatten()
            .flat_map(|h| h.to_le_bytes())
            .collect(),
    );
    image
}

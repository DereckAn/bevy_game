//! Árboles LOD analíticos: una caja por árbol, un mesh/material por columna.
//! El fragment shader intersecta tronco y copa y escribe profundidad real.
use super::{LodChunk, LodLevel, TerrainGenerator};
use crate::{
    core::{BASE_CHUNK_SIZE, GameState, VOXEL_SIZE, WorldKind, WorldSeed},
    vegetation::trees::{TREE_CELL_SIZE, TreeKind, tree_in_cell},
};
use bevy::{
    asset::RenderAssetUsages,
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

pub struct RaycastTreesPlugin;
impl Plugin for RaycastTreesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<TreeMaterial>::default())
            .init_resource::<TreeMaterialHandle>()
            .add_systems(Update, attach_trees.run_if(in_state(GameState::InGame)));
    }
}
#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
pub struct TreeMaterial {
    #[uniform(100)]
    colors: TreeColors,
}
#[derive(Clone, Default, ShaderType)]
struct TreeColors {
    wood: Vec4,
    pine_wood: Vec4,
    leaves: Vec4,
    needles: Vec4,
}
impl Material for TreeMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/raycast_trees.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/raycast_trees.wgsl".into()
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?];
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}
#[derive(Resource)]
struct TreeMaterialHandle(Handle<TreeMaterial>);
impl FromWorld for TreeMaterialHandle {
    fn from_world(world: &mut World) -> Self {
        let color = |kind| Vec4::from_array(super::voxel_color(kind));
        use super::VoxelType;
        Self(
            world
                .resource_mut::<Assets<TreeMaterial>>()
                .add(TreeMaterial {
                    colors: TreeColors {
                        wood: color(VoxelType::Wood),
                        pine_wood: color(VoxelType::PineWood),
                        leaves: color(VoxelType::Leaves),
                        needles: color(VoxelType::PineNeedles),
                    },
                }),
        )
    }
}
fn attach_trees(
    mut commands: Commands,
    chunks: Query<(Entity, &LodChunk), Added<LodChunk>>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<TreeMaterialHandle>,
    seed: Res<WorldSeed>,
    kind: Res<WorldKind>,
) {
    for (entity, chunk) in &chunks {
        if let Some(mesh) = tree_mesh(chunk, seed.0, *kind) {
            commands.entity(entity).with_child((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.0.clone()),
                Transform::default(),
                NotShadowCaster,
                NotShadowReceiver,
            ));
        }
    }
}

fn tree_mesh(chunk: &LodChunk, seed: i32, kind: WorldKind) -> Option<Mesh> {
    #[cfg(feature = "tracy")]
    let _span = info_span!("lod_tree_proxy_build").entered();
    if chunk.lod_level == LodLevel::Minimal {
        return None;
    }
    let n = BASE_CHUNK_SIZE as i32;
    let lo = chunk.position.xz() * n;
    let hi = lo + n;
    let mut terrain = TerrainGenerator::new(seed, kind);
    let (mut positions, mut data, mut species, mut indices) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for z in lo.y.div_euclid(TREE_CELL_SIZE)..=(hi.y - 1).div_euclid(TREE_CELL_SIZE) {
        for x in lo.x.div_euclid(TREE_CELL_SIZE)..=(hi.x - 1).div_euclid(TREE_CELL_SIZE) {
            let Some(tree) = tree_in_cell(x, z, seed, kind) else {
                continue;
            };
            if !matches!(tree.kind, TreeKind::Pine | TreeKind::Oak)
                || tree.world_x < lo.x
                || tree.world_x >= hi.x
                || tree.world_z < lo.y
                || tree.world_z >= hi.y
            {
                continue;
            }
            let wx = tree.world_x as f32 * VOXEL_SIZE;
            let wz = tree.world_z as f32 * VOXEL_SIZE;
            let y = (terrain.biome_gen.generate_height(wx, wz) / VOXEL_SIZE).floor() * VOXEL_SIZE
                + VOXEL_SIZE;
            append_proxy(
                Vec3::new(wx, y, wz),
                tree.height() as f32 * VOXEL_SIZE,
                tree.kind == TreeKind::Oak,
                &mut positions,
                &mut data,
                &mut species,
                &mut indices,
            );
        }
    }
    if positions.is_empty() {
        return None;
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    // COLOR carries the tree base and height, not a colour. UV carries species.
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, species);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}
fn append_proxy(
    base: Vec3,
    height: f32,
    oak: bool,
    p: &mut Vec<[f32; 3]>,
    data: &mut Vec<[f32; 4]>,
    species: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let first = p.len() as u32;
    // Outward winding, seen from outside; the material rasterizes back faces.
    for v in [
        Vec3::new(-0.23, 0., -0.23),
        Vec3::new(0.23, 0., -0.23),
        Vec3::new(0.23, 0., 0.23),
        Vec3::new(-0.23, 0., 0.23),
        Vec3::new(-0.23, 1., -0.23),
        Vec3::new(0.23, 1., -0.23),
        Vec3::new(0.23, 1., 0.23),
        Vec3::new(-0.23, 1., 0.23),
    ] {
        p.push((base + v * height).to_array());
        data.push(base.extend(height).to_array());
        species.push([if oak { 1.0 } else { 0.0 }, 0.0]);
    }
    for face in [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [3, 7, 4, 0],
    ] {
        indices.extend([face[0], face[1], face[2], face[0], face[2], face[3]].map(|i| first + i));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proxy_is_outward_and_contains_tree() {
        let (mut p, mut d, mut s, mut i) = (vec![], vec![], vec![], vec![]);
        append_proxy(Vec3::ZERO, 5., false, &mut p, &mut d, &mut s, &mut i);
        for t in i.chunks_exact(3) {
            let [a, b, c] = [
                Vec3::from(p[t[0] as usize]),
                Vec3::from(p[t[1] as usize]),
                Vec3::from(p[t[2] as usize]),
            ];
            assert!((b - a).cross(c - a).dot((a + b + c) / 3. - Vec3::Y * 2.5) > 0.);
        }
        assert_eq!((p.len(), d.len(), s.len(), i.len()), (8, 8, 8, 36));
    }
    #[test]
    fn lod_child_uses_shared_material_and_despawns_with_parent() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<TreeMaterial>>()
            .init_resource::<TreeMaterialHandle>()
            .insert_resource(WorldSeed(42))
            .insert_resource(WorldKind::Normal)
            .add_systems(Update, attach_trees);
        let parent = app
            .world_mut()
            .spawn(LodChunk::new(IVec3::ZERO, LodLevel::Medium))
            .id();
        app.update();
        let children = app
            .world()
            .get::<Children>(parent)
            .expect("seed 42 has trees");
        assert_eq!(children.len(), 1);
        let child = children[0];
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<TreeMaterial>>(child)
                .unwrap()
                .0,
            app.world().resource::<TreeMaterialHandle>().0
        );
        app.update();
        assert_eq!(app.world().get::<Children>(parent).unwrap().len(), 1);
        app.world_mut().entity_mut(parent).despawn();
        assert!(app.world().get_entity(child).is_err());
    }
    #[test]
    fn trees_have_unique_owner_even_at_negative_chunk_edges() {
        let mut bases = std::collections::HashSet::new();
        for z in -2..2 {
            for x in -2..2 {
                let chunk = LodChunk::new(IVec3::new(x, 0, z), LodLevel::Medium);
                if let Some(mesh) = tree_mesh(&chunk, 42, WorldKind::Normal) {
                    let bevy::mesh::VertexAttributeValues::Float32x4(data) =
                        mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap()
                    else {
                        panic!()
                    };
                    for tree in data.chunks_exact(8) {
                        let base = tree[0];
                        assert!(bases.insert((base[0].to_bits(), base[2].to_bits())));
                        assert!(base[0] >= x as f32 * 3.2 && base[0] < (x + 1) as f32 * 3.2);
                    }
                }
            }
        }
        assert!(!bases.is_empty());
    }
}

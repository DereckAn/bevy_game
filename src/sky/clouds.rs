//! Domo de cielo: un domo gigante centrado en la cámara cuyo material compone,
//! por dirección de visión, estrellas + luna + nubes FBM sobre la atmósfera.
//! Barato (ruido 2D en el fragment shader) y dinámico: las nubes derivan con el
//! tiempo y su `coverage` será la palanca del clima; la luna sale opuesta al sol
//! y las estrellas aparecen de noche. Ver `assets/shaders/clouds.wgsl`.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use super::time_of_day::{sun_direction, TimeOfDay};

const SHADER_PATH: &str = "shaders/clouds.wgsl";
/// Radio del domo. Dentro del far plane por defecto (1000) y muy por fuera del
/// radio de carga de terreno (~205 m), así el terreno siempre lo ocluye.
const DOME_RADIUS: f32 = 800.0;

/// Marca el domo de nubes (lo movemos con la cámara y le pasamos uniforms).
#[derive(Component)]
pub struct CloudDome;

/// Uniforms del material de nubes. Dos `vec4` para alineación limpia (std140).
#[derive(Clone, ShaderType)]
pub struct CloudUniform {
    /// xyz = dirección hacia el sol.
    pub sun_direction: Vec4,
    /// x = tiempo (deriva), y = coverage [0,1].
    pub params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct CloudMaterial {
    #[uniform(0)]
    pub data: CloudUniform,
}

impl Material for CloudMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // El shader compone estrellas/luna/nubes y emite color premultiplicado.
        AlphaMode::Premultiplied
    }

    /// La cámara está DENTRO del domo, así que hay que ver sus caras internas.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Crea el domo de nubes al empezar partida.
pub fn spawn_cloud_dome(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CloudMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(DOME_RADIUS).mesh().uv(32, 18));
    let material = materials.add(CloudMaterial {
        data: CloudUniform {
            sun_direction: Vec4::new(0.0, 1.0, 0.0, 0.0),
            params: Vec4::new(0.0, 0.4, 0.0, 0.0),
        },
    });

    commands.spawn((
        CloudDome,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::default(),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

/// Elimina el domo al volver al menú (no es chunk ni luz, así que `teardown_world`
/// no lo toca).
pub fn despawn_cloud_dome(mut commands: Commands, domes: Query<Entity, With<CloudDome>>) {
    for entity in &domes {
        commands.entity(entity).despawn();
    }
}

/// Mantiene el domo centrado en la cámara y actualiza tiempo y dirección del sol.
pub fn update_cloud_material(
    time: Res<Time>,
    tod: Res<TimeOfDay>,
    mut materials: ResMut<Assets<CloudMaterial>>,
    mut dome: Query<(&MeshMaterial3d<CloudMaterial>, &mut Transform), With<CloudDome>>,
    camera: Query<&Transform, (With<Camera3d>, Without<CloudDome>)>,
) {
    let Ok((mat_handle, mut dome_tf)) = dome.single_mut() else {
        return;
    };
    if let Ok(cam_tf) = camera.single() {
        dome_tf.translation = cam_tf.translation;
    }
    if let Some(mat) = materials.get_mut(&mat_handle.0) {
        mat.data.sun_direction = sun_direction(tod.fraction).extend(0.0);
        mat.data.params.x = time.elapsed_secs();
        // ponytail: coverage fija (cielo soleado con algo de nube). La palanca
        // del clima la conectará el sistema Weather cuando exista.
        mat.data.params.y = 0.4;
    }
}

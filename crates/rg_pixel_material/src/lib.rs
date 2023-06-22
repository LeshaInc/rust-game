use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

pub struct PixelMaterialPlugin;

impl Plugin for PixelMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MaterialPlugin::<PixelMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(Update, update_globals);
    }
}

#[derive(Debug, Clone, Component, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "7e80d778-3cb8-4ec2-95bf-ceb03ce277e0"]
pub struct PixelMaterial {
    #[uniform(0)]
    pub color: Color,
    // TODO: shader globals
    #[uniform(0)]
    pub dither_offset: UVec2,
    #[texture(1)]
    pub dither_matrix: Option<Handle<Image>>,
}

impl Default for PixelMaterial {
    fn default() -> Self {
        PixelMaterial {
            color: Color::WHITE,
            dither_matrix: None,
            dither_offset: UVec2::ZERO,
        }
    }
}

impl Material for PixelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/pixel.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }
}

#[derive(Debug, Resource)]
pub struct GlobalDitherMatrix(pub Handle<Image>);

#[derive(Debug, Default, Resource)]
pub struct GlobalDitherOffset(pub UVec2);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let dither_offset = GlobalDitherOffset::default();
    commands.insert_resource(dither_offset);

    let dither_matrix = GlobalDitherMatrix(asset_server.load("images/bayer4x4.png"));
    commands.insert_resource(dither_matrix);
}

fn update_globals(
    mut materials: ResMut<Assets<PixelMaterial>>,
    dither_offset: Res<GlobalDitherOffset>,
    dither_matrix: Res<GlobalDitherMatrix>,
) {
    for (_, material) in materials.iter_mut() {
        material.dither_offset = dither_offset.0;
        material.dither_matrix = Some(dither_matrix.0.clone());
    }
}

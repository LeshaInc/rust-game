use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

pub struct PixelMaterialPlugin;

impl Plugin for PixelMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MaterialPlugin::<PixelMaterial>::default());
    }
}

#[derive(Debug, Clone, AsBindGroup, TypeUuid, TypePath)]
#[uuid = "7e80d778-3cb8-4ec2-95bf-ceb03ce277e0"]
pub struct PixelMaterial {
    #[uniform(0)]
    pub color: Color,
}

impl Material for PixelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/pixel.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
    }
}

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension};

pub struct ArrayTexturePlugin;

impl Plugin for ArrayTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, build_array_textures);
    }
}

#[derive(Debug, Component)]
pub struct BuildArrayTexture {
    pub target: Handle<Image>,
    pub layers: Vec<Handle<Image>>,
}

fn build_array_textures(
    q_array_textures: Query<(Entity, &BuildArrayTexture)>,
    mut asset_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    if asset_events.read().count() == 0 {
        return;
    }

    for (entity, array_texture) in q_array_textures.iter() {
        if array_texture.layers.is_empty() {
            warn!("array texture can't be empty");
            continue;
        }

        let available = array_texture
            .layers
            .iter()
            .all(|handle| images.contains(handle));
        if !available {
            continue;
        }

        let first = images.get(&array_texture.layers[0]).unwrap();
        let format = first.texture_descriptor.format;

        if format.is_compressed() {
            warn!("compressed array textures aren't supported");
            continue;
        }

        let size = Extent3d {
            depth_or_array_layers: array_texture.layers.len() as u32,
            ..first.texture_descriptor.size
        };

        let mut data: Vec<u8> = Vec::with_capacity(
            (size.width as usize) * (size.height as usize) * (size.depth_or_array_layers as usize),
        );

        for handle in &array_texture.layers {
            let image = images.get(handle).unwrap();
            data.extend(&image.data);
        }

        let image = Image::new(size, TextureDimension::D2, data, format);
        images.insert(array_texture.target.clone(), image);
        commands.entity(entity).despawn();
    }
}

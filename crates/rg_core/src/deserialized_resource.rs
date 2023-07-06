use std::marker::PhantomData;

use bevy::asset::{self, Asset, AssetLoader, AssetPath, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::utils::BoxedFuture;
use serde::de::DeserializeOwned;

pub struct DeserializedResourcePlugin<R: DeserializedResource> {
    path: AssetPath<'static>,
    marker: PhantomData<R>,
}

impl<R: DeserializedResource> DeserializedResourcePlugin<R> {
    pub fn new<'a>(path: impl Into<AssetPath<'a>>) -> DeserializedResourcePlugin<R> {
        DeserializedResourcePlugin {
            path: path.into().to_owned(),
            marker: PhantomData,
        }
    }
}

impl<R: DeserializedResource> Plugin for DeserializedResourcePlugin<R> {
    fn build(&self, app: &mut App) {
        app.add_asset::<R>()
            .add_asset_loader(RonLoader::<R>(PhantomData))
            .add_systems(Update, handle_events::<R>);
    }

    fn finish(&self, app: &mut App) {
        let asset_server = app.world.resource::<AssetServer>();
        let handle = asset_server.load(self.path.clone());
        app.insert_resource(ResourceHandle::<R> { handle });
    }
}

pub trait DeserializedResource: DeserializeOwned + Resource + Asset + Clone {
    const EXTENSION: &'static str;
}

struct RonLoader<R: DeserializedResource>(PhantomData<R>);

impl<R: DeserializedResource> AssetLoader for RonLoader<R> {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), asset::Error>> {
        Box::pin(async move {
            let asset = ron::de::from_bytes::<R>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &[R::EXTENSION]
    }
}

#[derive(Resource)]
struct ResourceHandle<R: DeserializedResource> {
    handle: Handle<R>,
}

fn handle_events<R: DeserializedResource>(
    mut events: EventReader<AssetEvent<R>>,
    resources: Res<Assets<R>>,
    resource_handle: Res<ResourceHandle<R>>,
    mut commands: Commands,
) {
    for event in &mut events {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if &resource_handle.handle != handle {
                    continue;
                }

                let Some(resource) = resources.get(handle) else {
                    continue;
                };

                commands.insert_resource(resource.clone());
            }
            AssetEvent::Removed { .. } => {}
        }
    }
}

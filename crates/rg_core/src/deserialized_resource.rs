use std::marker::PhantomData;

use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, AssetPath, AsyncReadExt, LoadContext};
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
            path: path.into().clone_owned(),
            marker: PhantomData,
        }
    }
}

impl<R: DeserializedResource> Plugin for DeserializedResourcePlugin<R> {
    fn build(&self, app: &mut App) {
        app.init_asset::<R>()
            .init_asset_loader::<RonLoader<R>>()
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

impl<R: DeserializedResource> Default for RonLoader<R> {
    fn default() -> Self {
        RonLoader(PhantomData)
    }
}

impl<R: DeserializedResource> AssetLoader for RonLoader<R> {
    type Asset = R;
    type Settings = ();
    type Error = anyhow::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let asset = ron::de::from_bytes::<R>(&bytes)?;
            Ok(asset)
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
    for event in events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                if &resource_handle.handle.id() != id {
                    continue;
                }

                let Some(resource) = resources.get(*id) else {
                    continue;
                };

                commands.insert_resource(resource.clone());
            }
            _ => {}
        }
    }
}

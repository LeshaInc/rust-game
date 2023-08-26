use bevy::prelude::*;
use rg_core::progress::ProgressReader;

rg_core::progress_stages! {
    pub enum WorldgenStage {
        Init => "Initializing world generator...",
        Island => "Generating the island...",
        Height => "Raising mountains...",
        Rivers => "Forming rivers...",
        Shores => "Generating shores...",
        Biomes => "Generating biomes...",
        Topography => "Mapping the world...",
        Saving => "Saving the world...",
    }
}

#[derive(Resource, Deref)]
pub struct WorldgenProgress(pub ProgressReader<WorldgenStage>);

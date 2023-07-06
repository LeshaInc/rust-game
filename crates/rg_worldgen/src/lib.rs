mod elevation;
mod island_shaping;
mod rivers;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use rand::SeedableRng;
use rand_pcg::Pcg32;
use rg_core::{DeserializedResource, DeserializedResourcePlugin, Grid};
use rivers::RiversSettings;
use serde::Deserialize;

use crate::elevation::compute_elevation;
pub use crate::elevation::ElevationSettings;
use crate::island_shaping::shape_island;
pub use crate::island_shaping::IslandSettings;
use crate::rivers::generate_rivers;

pub struct WorldgenPlugin;

impl Plugin for WorldgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DeserializedResourcePlugin::<WorldgenSettings>::new(
            "default.worldgen.ron",
        ))
        .add_systems(
            Update,
            worldgen
                .run_if(resource_exists::<WorldSeed>())
                .run_if(resource_exists::<WorldgenSettings>())
                .run_if(
                    not(resource_exists::<WorldMaps>())
                        .or_else(resource_changed::<WorldgenSettings>())
                        .or_else(resource_changed::<WorldSeed>()),
                ),
        );
    }
}

#[derive(Debug, Copy, Clone, Resource)]
pub struct WorldSeed(pub u64);

#[derive(Debug, Copy, Clone, Resource, Deserialize, TypePath, TypeUuid)]
#[uuid = "9642a5f8-7606-4775-b5bc-6fda6d73bd84"]
pub struct WorldgenSettings {
    pub island: IslandSettings,
    pub elevation: ElevationSettings,
    pub rivers: RiversSettings,
}

impl DeserializedResource for WorldgenSettings {
    const EXTENSION: &'static str = "worldgen.ron";
}

#[derive(Debug, Resource)]
pub struct WorldMaps {
    pub elevation: Arc<Grid<f32>>,
}

fn worldgen(seed: Res<WorldSeed>, settings: Res<WorldgenSettings>, mut commands: Commands) {
    let mut rng = Pcg32::seed_from_u64(seed.0);

    // TODO: offload to async pool

    let island = shape_island(&mut rng, &settings.island);
    let mut elevation = compute_elevation(&island, &settings.elevation);
    let rivers = generate_rivers(&mut rng, &mut elevation, &settings.rivers);

    elevation.debug_save(&format!("/tmp/elevation.png"));
    rivers.debug_save(&format!("/tmp/rivers.png"));

    commands.insert_resource(WorldMaps {
        elevation: Arc::new(elevation),
    });
}

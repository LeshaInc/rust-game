mod elevation;
mod island_shaping;
mod progress;
mod rivers;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use progress::WorldgenProgressUiPlugin;
use rand::SeedableRng;
use rand_pcg::Pcg32;
use rg_core::{DeserializedResource, DeserializedResourcePlugin, Grid};
use rivers::RiversSettings;
use serde::Deserialize;

use crate::elevation::compute_elevation;
pub use crate::elevation::ElevationSettings;
use crate::island_shaping::shape_island;
pub use crate::island_shaping::IslandSettings;
pub use crate::progress::{WorldgenProgress, WorldgenStage};
use crate::rivers::generate_rivers;

pub struct WorldgenPlugin;

impl Plugin for WorldgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<WorldgenState>()
            .add_plugins(DeserializedResourcePlugin::<WorldgenSettings>::new(
                "default.worldgen.ron",
            ))
            .add_plugins(WorldgenProgressUiPlugin)
            .add_systems(
                PreUpdate,
                (
                    schedule_task
                        .run_if(resource_exists::<WorldSeed>())
                        .run_if(resource_exists::<WorldgenSettings>())
                        .run_if(not(resource_exists::<WorldgenTask>())),
                    update_task.run_if(resource_exists::<WorldgenTask>()),
                )
                    .run_if(in_state(WorldgenState::InProgress)),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, States)]
pub enum WorldgenState {
    #[default]
    InProgress,
    Done,
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

#[derive(Resource)]
struct WorldgenTask(pub Task<WorldMaps>);

fn schedule_task(seed: Res<WorldSeed>, settings: Res<WorldgenSettings>, mut commands: Commands) {
    let pool = AsyncComputeTaskPool::get();
    let seed = seed.0;
    let settings = settings.clone();
    let progress = WorldgenProgress::default();
    commands.insert_resource(progress.clone());

    let task = pool.spawn(async move {
        let _scope = info_span!("worldgen").entered();

        let mut rng = Pcg32::seed_from_u64(seed);
        let island = shape_island(&mut rng, &settings.island, &progress);
        let mut elevation = compute_elevation(&island, &settings.elevation, &progress);
        let rivers = generate_rivers(&mut rng, &mut elevation, &settings.rivers, &progress);

        island.debug_save(&format!("/tmp/island.png"));
        elevation.debug_save(&format!("/tmp/elevation.png"));
        rivers.debug_save(&format!("/tmp/rivers.png"));

        WorldMaps {
            elevation: Arc::new(elevation),
        }
    });

    commands.insert_resource(WorldgenTask(task));
}

fn update_task(
    mut task: ResMut<WorldgenTask>,
    mut next_state: ResMut<NextState<WorldgenState>>,
    mut commands: Commands,
) {
    if let Some(res) = future::block_on(future::poll_once(&mut task.0)) {
        commands.insert_resource(res);
        commands.remove_resource::<WorldgenTask>();
        commands.remove_resource::<WorldgenProgress>();
        next_state.set(WorldgenState::Done);
    }
}

mod biomes;
mod height;
mod island;
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

use crate::biomes::generate_biome_map;
pub use crate::biomes::Biome;
use crate::height::generate_height_map;
pub use crate::height::HeightSettings;
use crate::island::generate_island_map;
pub use crate::island::IslandSettings;
pub use crate::progress::{WorldgenProgress, WorldgenStage};
use crate::rivers::generate_river_map;

pub const WORLD_SCALE: f32 = 2.0;

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
    pub height: HeightSettings,
    pub rivers: RiversSettings,
}

impl DeserializedResource for WorldgenSettings {
    const EXTENSION: &'static str = "worldgen.ron";
}

#[derive(Debug)]
pub struct WorldMaps {
    pub seed: u64,
    pub height_map: Grid<f32>,
    pub river_map: Grid<f32>,
    pub biome_map: Grid<Biome>,
}

#[derive(Debug, Deref, Clone, Resource)]
pub struct SharedWorldMaps(pub Arc<WorldMaps>);

#[derive(Resource)]
struct WorldgenTask(pub Task<WorldMaps>);

fn schedule_task(seed: Res<WorldSeed>, settings: Res<WorldgenSettings>, mut commands: Commands) {
    let pool = AsyncComputeTaskPool::get();
    let seed = seed.0;
    let settings = *settings;
    let progress = WorldgenProgress::default();
    commands.insert_resource(progress.clone());

    let task = pool.spawn(async move {
        let _scope = info_span!("worldgen").entered();

        let mut rng = Pcg32::seed_from_u64(seed);
        let island_map = generate_island_map(&mut rng, &progress, &settings.island);
        island_map.debug_save(&format!("/tmp/island_map.png"));

        let mut height_map =
            generate_height_map(&mut rng, &progress, &settings.height, &island_map);
        height_map.debug_save(&format!("/tmp/init_height_map.png"));

        let river_map = generate_river_map(&mut rng, &progress, &settings.rivers, &mut height_map);
        let biome_map = generate_biome_map(&mut rng, &progress, &height_map);

        height_map.debug_save(&format!("/tmp/height_map.png"));
        river_map.debug_save(&format!("/tmp/river_map.png"));

        WorldMaps {
            seed,
            height_map,
            river_map,
            biome_map,
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
        commands.insert_resource(SharedWorldMaps(Arc::new(res)));
        commands.remove_resource::<WorldgenTask>();
        commands.remove_resource::<WorldgenProgress>();
        next_state.set(WorldgenState::Done);
    }
}

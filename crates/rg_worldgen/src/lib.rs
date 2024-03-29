mod biomes;
mod height;
mod island;
mod progress;
mod rivers;
mod shores;
mod topography;

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rand::SeedableRng;
use rand_pcg::Pcg32;
use rg_core::progress::new_progress_tracker;
use rg_worldgen_api::{
    NoiseMaps, SharedWorldMaps, WorldMaps, WorldSeed, WorldgenApiPlugin, WorldgenProgress,
    WorldgenSettings, WorldgenStage, WorldgenState,
};

use crate::biomes::generate_biome_map;
use crate::height::generate_height_map;
use crate::island::generate_island_map;
use crate::progress::WorldgenProgressUiPlugin;
use crate::rivers::generate_river_map;
use crate::shores::generate_shore_map;
use crate::topography::generate_topographic_map;

pub struct WorldgenPlugin;

impl Plugin for WorldgenPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WorldgenApiPlugin)
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

#[derive(Resource)]
struct WorldgenTask(pub Task<WorldMaps>);

fn schedule_task(seed: Res<WorldSeed>, settings: Res<WorldgenSettings>, mut commands: Commands) {
    let pool = AsyncComputeTaskPool::get();
    let seed = seed.0;
    let settings = *settings;

    let tmp_dir = std::env::temp_dir();
    let (progress_reader, mut progress) = new_progress_tracker(
        cfg!(debug_assertions).then(|| tmp_dir.join("worldgen_progress.bin")),
        Some(include_bytes!("progress.bin")),
    );

    commands.insert_resource(WorldgenProgress(progress_reader));

    let task = pool.spawn(async move {
        let _scope = info_span!("worldgen").entered();

        let tmp_dir = &tmp_dir;
        let path = tmp_dir.join("world.bin");

        if path.exists() {
            match WorldMaps::load(&path) {
                Ok(world_maps) => return world_maps,
                Err(e) => {
                    warn!("{e:?}");
                }
            }
        }

        let mut rng = Pcg32::seed_from_u64(seed);
        let noise_maps = NoiseMaps::new(&mut rng, &settings.noise);

        let island_map = generate_island_map(
            &mut rng,
            &mut progress.stage(WorldgenStage::Island),
            &settings.island,
            &noise_maps,
        );

        let mut height_map = generate_height_map(
            &mut progress.stage(WorldgenStage::Height),
            &settings.height,
            &noise_maps,
            &island_map,
        );

        let river_map = generate_river_map(
            &mut rng,
            &mut progress.stage(WorldgenStage::Rivers),
            &settings.rivers,
            &island_map,
            &mut height_map,
        );

        let shore_map = generate_shore_map(
            &mut progress.stage(WorldgenStage::Shores),
            &island_map,
            &river_map,
        );

        let biome_map = generate_biome_map(
            &mut progress.stage(WorldgenStage::Biomes),
            &noise_maps,
            &height_map,
        );

        let topographic_map = generate_topographic_map(
            &mut progress.stage(WorldgenStage::Topography),
            &settings.topography,
            &height_map,
        );

        let maps = [
            ("island_map", &island_map),
            ("height_map", &height_map),
            ("river_map", &river_map),
            ("shore_map", &shore_map),
        ];

        let mut saving_stage = progress.stage(WorldgenStage::Saving);

        saving_stage.multi_task(4, |task| {
            rayon::scope(|s| {
                for (name, grid) in maps {
                    let task = &task;
                    s.spawn(move |_| {
                        grid.debug_save(tmp_dir.join(format!("{name}.png")));
                        task.subtask_completed();
                    });
                }
            });
        });

        saving_stage.task(|| topographic_map.debug_save(tmp_dir.join("topographic_map.png")));

        let world_maps = WorldMaps {
            seed,
            noise_maps,
            height_map,
            river_map,
            shore_map,
            biome_map,
        };

        saving_stage.task(|| world_maps.save(path).unwrap());
        progress.finish();

        world_maps
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

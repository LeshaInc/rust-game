mod biomes;
mod height;
mod island;
mod noise_maps;
mod progress;
mod rivers;
mod shores;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use progress::WorldgenProgressUiPlugin;
use rand::SeedableRng;
use rand_pcg::Pcg32;
use rg_core::progress::new_progress_tracker;
use rg_core::{DeserializedResource, DeserializedResourcePlugin, Grid};
use rivers::RiversSettings;
use serde::{Deserialize, Serialize};

use crate::biomes::generate_biome_map;
pub use crate::biomes::Biome;
use crate::height::generate_height_map;
pub use crate::height::HeightSettings;
use crate::island::generate_island_map;
pub use crate::island::IslandSettings;
pub use crate::noise_maps::{NoiseMaps, NoiseSettings};
pub use crate::progress::{WorldgenProgress, WorldgenStage};
use crate::rivers::generate_river_map;
use crate::shores::generate_shore_map;

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
    pub noise: NoiseSettings,
    pub island: IslandSettings,
    pub height: HeightSettings,
    pub rivers: RiversSettings,
}

impl DeserializedResource for WorldgenSettings {
    const EXTENSION: &'static str = "worldgen.ron";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMaps {
    pub seed: u64,
    pub noise_maps: NoiseMaps,
    pub height_map: Grid<f32>,
    pub river_map: Grid<f32>,
    pub shore_map: Grid<f32>,
    pub biome_map: Grid<Biome>,
}

impl WorldMaps {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<WorldMaps> {
        let _scope = info_span!("load").entered();

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let world_maps = rmp_serde::decode::from_read(reader)?;
        Ok(world_maps)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let _scope = info_span!("save").entered();

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        rmp_serde::encode::write_named(&mut writer, self)?;
        writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Deref, Clone, Resource)]
pub struct SharedWorldMaps(pub Arc<WorldMaps>);

#[derive(Resource)]
struct WorldgenTask(pub Task<WorldMaps>);

fn schedule_task(seed: Res<WorldSeed>, settings: Res<WorldgenSettings>, mut commands: Commands) {
    let pool = AsyncComputeTaskPool::get();
    let seed = seed.0;
    let settings = *settings;

    let (progress_reader, mut progress) = new_progress_tracker(
        cfg!(debug_assertions).then(|| "/tmp/worldgen_progress.bin"),
        Some(include_bytes!("progress.bin")),
    );

    commands.insert_resource(WorldgenProgress(progress_reader));

    let task = pool.spawn(async move {
        let _scope = info_span!("worldgen").entered();

        let path = Path::new("/tmp/world.bin");

        if path.exists() {
            match WorldMaps::load("/tmp/world.bin") {
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
                        grid.debug_save(&format!("/tmp/{name}.png"));
                        task.subtask_completed();
                    });
                }
            });
        });

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

mod elevation;
mod island_shaping;
mod rivers;

use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
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
            (
                schedule_task
                    .run_if(not(resource_exists::<WorldgenTask>()))
                    .run_if(resource_exists::<WorldSeed>())
                    .run_if(resource_exists::<WorldgenSettings>())
                    .run_if(not(resource_exists::<WorldMaps>())),
                print_progress.run_if(resource_exists::<WorldgenProgress>()),
            ),
        )
        .add_systems(
            PreUpdate,
            update_task.run_if(resource_exists::<WorldgenTask>()),
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

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WorldgenStage {
    Island = 0,
    Elevation,
    Rivers,
}

#[derive(Debug, Default, Clone, Resource)]
pub struct WorldgenProgress(Arc<AtomicU16>);

impl WorldgenProgress {
    pub fn set(&self, stage: WorldgenStage, progress: u8) {
        let val = (stage as u16) << 8 | (progress as u16);
        self.0.store(val, Relaxed)
    }

    pub fn get(&self) -> (WorldgenStage, u8) {
        let val = self.0.load(Relaxed);
        let stage = match val >> 8 {
            0 => WorldgenStage::Island,
            1 => WorldgenStage::Elevation,
            _ => WorldgenStage::Rivers,
        };
        let progress = val as u8;
        (stage, progress)
    }
}

#[derive(Debug, Resource)]
pub struct WorldMaps {
    pub elevation: Arc<Grid<f32>>,
}

#[derive(Resource)]
struct WorldgenTask(pub Task<WorldMaps>);

fn schedule_task(seed: Res<WorldSeed>, settings: Res<WorldgenSettings>, mut commands: Commands) {
    println!("SCHEDULE!!!");

    let pool = AsyncComputeTaskPool::get();
    let seed = seed.0;
    let settings = settings.clone();
    let progress = WorldgenProgress::default();
    commands.insert_resource(progress.clone());

    let task = pool.spawn(async move {
        let mut rng = Pcg32::seed_from_u64(seed);
        let island = shape_island(&mut rng, &settings.island, &progress);
        let mut elevation = compute_elevation(&island, &settings.elevation, &progress);
        let rivers = generate_rivers(&mut rng, &mut elevation, &settings.rivers, &progress);

        elevation.debug_save(&format!("/tmp/elevation.png"));
        rivers.debug_save(&format!("/tmp/rivers.png"));

        WorldMaps {
            elevation: Arc::new(elevation),
        }
    });

    commands.insert_resource(WorldgenTask(task));
}

fn update_task(mut task: ResMut<WorldgenTask>, mut commands: Commands) {
    if let Some(res) = future::block_on(future::poll_once(&mut task.0)) {
        commands.insert_resource(res);
        commands.remove_resource::<WorldgenTask>();
        commands.remove_resource::<WorldgenProgress>();
    }
}

fn print_progress(progress: Res<WorldgenProgress>) {
    println!("{:?}", progress.get());
}

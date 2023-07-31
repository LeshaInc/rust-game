mod biomes;
mod height;
mod island;
mod progress;
mod rivers;
mod shores;

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
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
    pub shore_map: Grid<f32>,
    pub biome_map: Grid<Biome>,
}

impl WorldMaps {
    const SIGNATURE: &[u8; 8] = b"RG_WORLD";
    const VERSION: u32 = 0;

    pub fn load(path: impl AsRef<Path>) -> io::Result<WorldMaps> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        Self::decode(&mut reader)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        self.encode(&mut writer)
    }

    pub fn decode<R: Read>(reader: &mut R) -> io::Result<WorldMaps> {
        let _scope = info_span!("decode").entered();

        let mut buf = [0; 8];
        reader.read_exact(&mut buf)?;
        if &buf != Self::SIGNATURE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid signature",
            ));
        }

        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        let version = u32::from_ne_bytes(buf);
        if version != Self::VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "invalid version (got {version}, expected {})",
                    Self::VERSION
                ),
            ));
        }

        let mut buf = [0; 8];
        reader.read_exact(&mut buf)?;
        let seed = u64::from_ne_bytes(buf);

        let height_map = Grid::decode(reader)?;
        let river_map = Grid::decode(reader)?;
        let shore_map = Grid::decode(reader)?;
        let biome_map = Grid::decode(reader)?;

        Ok(WorldMaps {
            seed,
            height_map,
            river_map,
            shore_map,
            biome_map,
        })
    }

    pub fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let _scope = info_span!("encode").entered();

        writer.write_all(Self::SIGNATURE)?;
        writer.write_all(&Self::VERSION.to_ne_bytes())?;
        writer.write_all(&self.seed.to_ne_bytes())?;
        self.height_map.encode(writer)?;
        self.river_map.encode(writer)?;
        self.shore_map.encode(writer)?;
        self.biome_map.encode(writer)?;

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
    let progress = WorldgenProgress::default();
    commands.insert_resource(progress.clone());

    let task = pool.spawn(async move {
        let _scope = info_span!("worldgen").entered();

        match WorldMaps::load("/tmp/world.bin") {
            Ok(world_maps) => return world_maps,
            Err(e) => {
                warn!("{e:?}");
            }
        }

        let mut rng = Pcg32::seed_from_u64(seed);
        let island_map = generate_island_map(&mut rng, &progress, &settings.island);
        let mut height_map =
            generate_height_map(&mut rng, &progress, &settings.height, &island_map);
        let init_height_map = height_map.clone();
        let river_map = generate_river_map(&mut rng, &progress, &settings.rivers, &mut height_map);
        let shore_map = generate_shore_map(&progress, &island_map, &river_map);
        let biome_map = generate_biome_map(&mut rng, &progress, &height_map);

        progress.set(WorldgenStage::Saving, 0);

        rayon::scope(|s| {
            s.spawn(|_| island_map.debug_save(&format!("/tmp/island_map.png")));
            s.spawn(|_| init_height_map.debug_save(&format!("/tmp/init_height_map.png")));
            s.spawn(|_| height_map.debug_save(&format!("/tmp/height_map.png")));
            s.spawn(|_| river_map.debug_save(&format!("/tmp/river_map.png")));
            s.spawn(|_| shore_map.debug_save(&format!("/tmp/shore_map.png")));
        });

        let world_maps = WorldMaps {
            seed,
            height_map,
            river_map,
            shore_map,
            biome_map,
        };

        world_maps.save("/tmp/world.bin").unwrap();
        progress.set(WorldgenStage::Saving, 100);

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

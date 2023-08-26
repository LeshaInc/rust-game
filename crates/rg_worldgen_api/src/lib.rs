pub mod progress;
pub mod settings;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use bytemuck::{CheckedBitPattern, NoUninit};
use rand::Rng;
use rg_core::grid::Grid;
use rg_core::noise::FbmNoise;
use rg_core::DeserializedResourcePlugin;
use serde::{Deserialize, Serialize};

pub use self::progress::*;
pub use self::settings::*;

pub const WORLD_SCALE: f32 = 2.0;

pub struct WorldgenApiPlugin;

impl Plugin for WorldgenApiPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<WorldgenState>()
            .add_plugins(DeserializedResourcePlugin::<WorldgenSettings>::new(
                "default.worldgen.ron",
            ))
            .insert_resource(WorldSeed(0));
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, NoUninit, CheckedBitPattern)]
#[repr(u8)]
pub enum Biome {
    Ocean,
    Plains,
    Forest,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseMaps {
    pub island: FbmNoise,
    pub height: FbmNoise,
    pub height_warp: FbmNoise<2>,
    pub biomes: FbmNoise,
    pub grass: FbmNoise,
}

impl NoiseMaps {
    pub fn new<R: Rng>(rng: &mut R, settings: &NoiseSettings) -> NoiseMaps {
        NoiseMaps {
            island: FbmNoise::new(rng, &settings.island),
            height: FbmNoise::new(rng, &settings.height),
            height_warp: FbmNoise::new(rng, &settings.height_warp),
            biomes: FbmNoise::new(rng, &settings.biomes),
            grass: FbmNoise::new(rng, &settings.grass),
        }
    }
}

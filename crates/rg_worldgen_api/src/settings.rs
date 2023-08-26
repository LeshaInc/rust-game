use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use rg_core::noise::FbmNoiseSettings;
use rg_core::DeserializedResource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Resource, Deserialize, TypePath, TypeUuid)]
#[uuid = "9642a5f8-7606-4775-b5bc-6fda6d73bd84"]
pub struct WorldgenSettings {
    pub noise: NoiseSettings,
    pub island: IslandSettings,
    pub height: HeightSettings,
    pub rivers: RiversSettings,
    pub topography: TopographySettings,
}

impl DeserializedResource for WorldgenSettings {
    const EXTENSION: &'static str = "worldgen.ron";
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct IslandSettings {
    pub size: UVec2,
    pub cutoff: f32,
    pub reshape_margin: f32,
    pub reshape_radius: f32,
    pub reshape_alpha: f32,
    pub min_island_area: f32,
    pub min_total_area: f32,
    pub max_total_area: f32,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct HeightSettings {
    pub beach_size: f32,
    pub land_height: f32,
    pub peak_height: f32,
    pub ocean_depth: f32,
    pub warp_dist: f32,
    pub mountain_power: f32,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct RiversSettings {
    pub point_radius: f32,
    pub inertia: f32,
    pub evaporation: f32,
    pub erosion: f32,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct TopographySettings {
    pub max_height: f32,
    pub iso_step: f32,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct NoiseSettings {
    pub island: FbmNoiseSettings,
    pub height: FbmNoiseSettings,
    pub height_warp: FbmNoiseSettings,
    pub biomes: FbmNoiseSettings,
    pub grass: FbmNoiseSettings,
}

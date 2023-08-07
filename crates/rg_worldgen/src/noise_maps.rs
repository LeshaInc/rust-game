use rand::Rng;
use rg_core::{FbmNoise, FbmNoiseSettings};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct NoiseSettings {
    pub island: FbmNoiseSettings,
    pub height: FbmNoiseSettings,
    pub height_warp: FbmNoiseSettings,
    pub biomes: FbmNoiseSettings,
    pub grass: FbmNoiseSettings,
}

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

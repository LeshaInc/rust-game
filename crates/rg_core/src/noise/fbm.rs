use std::f32::consts::TAU;

use bevy::prelude::*;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{Noise, SimplexNoise};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct FbmNoiseSettings {
    frequency: f32,
    #[serde(default = "default_octaves")]
    octaves: usize,
    #[serde(default = "default_persistence")]
    persistence: f32,
    #[serde(default = "default_lacunarity")]
    lacunarity: f32,
}

fn default_octaves() -> usize {
    5
}

fn default_persistence() -> f32 {
    0.5
}

fn default_lacunarity() -> f32 {
    2.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbmNoise<const N: usize = 1, S: Noise<N> = SimplexNoise<N>> {
    octaves: Vec<Octave<N, S>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Octave<const N: usize, S: Noise<N> = SimplexNoise<N>> {
    source: S,
    rotation: Vec2,
    offset: Vec2,
    frequency: f32,
    amplitude: f32,
}

impl<const N: usize, S: Noise<N>> FbmNoise<N, S> {
    pub fn new<R: Rng + ?Sized>(rng: &mut R, options: &FbmNoiseSettings) -> FbmNoise<N, S>
    where
        Standard: Distribution<S>,
    {
        let mut frequency = options.frequency;
        let mut amplitude = 1.0;
        let mut total_amplitude = 0.0;

        let mut octaves = (0..options.octaves)
            .map(|_| {
                let octave = Octave {
                    source: rng.gen(),
                    rotation: Vec2::from_angle(rng.gen_range(0.0..TAU)),
                    offset: Vec2::new(rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0)),
                    frequency,
                    amplitude,
                };

                total_amplitude += amplitude;
                amplitude *= options.persistence;
                frequency *= options.lacunarity;

                octave
            })
            .collect::<Vec<_>>();

        for octave in &mut octaves {
            octave.amplitude /= total_amplitude;
        }

        FbmNoise { octaves }
    }
}

impl<const N: usize, S: Noise<N>> Noise<N> for FbmNoise<N, S> {
    fn get(&self, pos: Vec2) -> [f32; N] {
        let mut res = [0.0; N];

        for octave in &self.octaves {
            let val = octave
                .source
                .get(octave.rotation.rotate(pos) * octave.frequency + octave.offset);
            for (res, val) in res.iter_mut().zip(val) {
                *res += val * octave.amplitude;
            }
        }

        res
    }
}

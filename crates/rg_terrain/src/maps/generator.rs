use std::sync::Arc;

use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use rg_core::chunk::{tile_pos_to_world, CHUNK_TILES};
use rg_core::grid::Grid;
use rg_core::noise::Noise;
use rg_core::DeserializedResource;
use rg_worldgen_api::{WorldMaps, WORLD_SCALE};
use serde::Deserialize;

use super::{ChunkMaps, SharedChunkMaps};
use crate::Tile;

#[derive(Debug, Copy, Clone, Resource, Deserialize, TypePath, TypeUuid, Asset)]
#[uuid = "d4b77ce0-db8c-477e-b771-deb43ca107c2"]
pub struct ChunkGenSettings {
    pub noise_height: f32,
    pub terrace_height: f32,
    pub terrace_slope: f32,
    pub shore_power: f32,
    pub river_depth: f32,
}

impl DeserializedResource for ChunkGenSettings {
    const EXTENSION: &'static str = "chunkgen.ron";
}

pub fn generate_maps(
    settings: &ChunkGenSettings,
    chunk_pos: IVec2,
    world_maps: &WorldMaps,
) -> SharedChunkMaps {
    let _span = info_span!("generate_maps").entered();

    let height_map = generate_height_map(settings, chunk_pos, world_maps);
    let tile_map = generate_tile_map(chunk_pos, world_maps, &height_map);
    let grass_density_map = generate_grass_density_map(chunk_pos, world_maps, &tile_map);
    let water_map = generate_water_map(chunk_pos, world_maps);

    SharedChunkMaps(Arc::new(ChunkMaps {
        height_map,
        tile_map,
        grass_density_map,
        water_map,
    }))
}

fn generate_height_map(
    settings: &ChunkGenSettings,
    chunk_pos: IVec2,
    world_maps: &WorldMaps,
) -> Grid<f32> {
    let _span = info_span!("generate_height_map").entered();

    let overscan = 16;
    let size = UVec2::splat(CHUNK_TILES) + overscan * 2;
    let origin = -IVec2::splat(overscan as i32);

    let blur_map = Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(IVec2::ZERO, chunk_pos, cell);
        let noise = world_maps.noise_maps.island.get(pos)[0];
        5.0 * (2.0 * noise - 1.0).max(0.0) + 1.0
    });

    let mut height_map = Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(IVec2::ZERO, chunk_pos, cell);

        let mut height = world_maps.height_map.sample(pos / WORLD_SCALE);
        let shore = world_maps.shore_map.sample(pos / WORLD_SCALE);

        let noise = world_maps.noise_maps.height.get(pos)[0];
        height += (1.0 - shore) * noise * settings.noise_height;

        let snapped = {
            let mut v = height / settings.terrace_height;
            let x = v.fract() - 0.5;
            v = v.floor() + (settings.terrace_slope * 2.0 * x / (1.0 - x * x)).tanh() * 0.5 + 0.5;
            v * settings.terrace_height
        };

        let alpha = shore.powf(settings.shore_power);
        height = snapped * (1.0 - alpha) + height * alpha;

        height
    });

    height_map.variable_gaussian_blur(&blur_map, 1.0, 6.0);

    for (cell, height) in height_map.entries_mut() {
        let pos = tile_pos_to_world(IVec2::ZERO, chunk_pos, cell);
        let river = world_maps.river_map.sample(pos / WORLD_SCALE);
        *height -= river * settings.river_depth;
    }

    height_map
}

fn generate_tile_map(
    chunk_pos: IVec2,
    world_maps: &WorldMaps,
    height_map: &Grid<f32>,
) -> Grid<Tile> {
    let _span = info_span!("generate_tile_map").entered();

    let size = UVec2::splat(CHUNK_TILES);
    Grid::from_fn(size, |cell| {
        if height_map[cell] < 0.0 {
            return Tile::Sand;
        }

        let pos = tile_pos_to_world(IVec2::ZERO, chunk_pos, cell);
        let river = world_maps.river_map.sample(pos / WORLD_SCALE);

        if river > 0.1 {
            return Tile::Sand;
        }

        Tile::Grass
    })
}

fn generate_grass_density_map(
    chunk_pos: IVec2,
    world_maps: &WorldMaps,
    tile_map: &Grid<Tile>,
) -> Grid<f32> {
    let _span = info_span!("generate_grass_density_map").entered();

    let size = UVec2::splat(CHUNK_TILES);
    Grid::from_fn(size, |cell| {
        if tile_map[cell] != Tile::Grass {
            return 0.0;
        }

        for (_, neighbor) in tile_map.neighborhood_8(cell) {
            if tile_map[neighbor] != Tile::Grass {
                return 0.0;
            }
        }

        let pos = tile_pos_to_world(IVec2::ZERO, chunk_pos, cell);
        world_maps.noise_maps.grass.get(pos)[0]
    })
}

fn generate_water_map(chunk_pos: IVec2, world_maps: &WorldMaps) -> Grid<f32> {
    let _span = info_span!("generate_water_map").entered();

    let overscan = 1;
    let size = UVec2::splat(CHUNK_TILES) + overscan * 2;
    let origin = -IVec2::splat(overscan as i32);

    Grid::from_fn_with_origin(size, origin, |cell| {
        let pos = tile_pos_to_world(IVec2::ZERO, chunk_pos, cell);
        let river = world_maps.river_map.sample(pos / WORLD_SCALE);
        let height = world_maps.height_map.sample(pos / WORLD_SCALE);

        if height < 0.0 {
            return 0.0;
        }

        if river > 0.0 {
            return (height - (2.0 / 3.0)).max(0.0);
        }

        f32::NAN
    })
}

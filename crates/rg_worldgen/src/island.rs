use bevy::prelude::*;
use rand::Rng;
use rg_core::progress::ProgressWriter;
use rg_core::{EdtSettings, Grid};
use serde::Deserialize;

use crate::{NoiseMaps, WorldgenStage};

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct IslandSettings {
    pub size: UVec2,
    pub cutoff: f32,
    pub reshape_margin: f32,
    pub reshape_radius: f32,
    pub reshape_alpha: f32,
    pub min_area: f32,
    pub max_area: f32,
}

pub fn generate_island_map<R: Rng>(
    rng: &mut R,
    progress: &mut ProgressWriter<WorldgenStage>,
    settings: &IslandSettings,
    noise_maps: &NoiseMaps,
) -> Grid<f32> {
    let _scope = info_span!("generate_island_map").entered();

    loop {
        let size = settings.size / 8;
        let mut grid = Grid::new(size, 0.0);

        progress.task(|| grid.add_noise(&noise_maps.island));
        progress.task(|| voronoi_reshape(rng, &mut grid, settings));

        let mut grid = grid.to_bool(settings.cutoff);

        progress.task(|| keep_one_island(&mut grid));

        progress.task(|| random_zoom(rng, &mut grid));
        progress.task(|| random_zoom(rng, &mut grid));
        progress.task(|| random_zoom(rng, &mut grid));

        progress.task(|| erode(&mut grid));
        progress.task(|| erode(&mut grid));
        progress.task(|| erode(&mut grid));

        progress.task(|| smooth(&mut grid));
        progress.task(|| smooth(&mut grid));

        progress.task(|| keep_one_island(&mut grid));

        if !is_island_area_good(&grid, settings) {
            // TODO: this shouldn't exist
            continue;
        }

        return progress.task(|| generate_sdf(&grid));
    }
}

fn voronoi_reshape<R: Rng>(rng: &mut R, grid: &mut Grid<f32>, settings: &IslandSettings) {
    let _scope = info_span!("voronoi_reshape").entered();

    let size = grid.size().as_vec2();
    let margin = f32::min(size.x, size.y) * settings.reshape_margin;

    let mut points = [Vec2::ZERO; 32];
    for point in points.iter_mut().skip(1) {
        point.x = rng.gen_range(margin..=(size.x - margin));
        point.y = rng.gen_range(margin..=(size.y - margin));
    }

    points[0] = size * 0.5;

    for (cell, value) in grid.entries_mut() {
        let pos = cell.as_vec2();
        let sq_dist = points
            .iter()
            .map(|p| p.distance_squared(pos))
            .fold(f32::INFINITY, f32::min);
        let inv_dist = 1.0 - sq_dist.sqrt() / (size.x.min(size.y) * settings.reshape_radius);

        let alpha = settings.reshape_alpha;
        *value = *value * (1.0 - alpha) + inv_dist * alpha;
    }
}

fn keep_one_island(grid: &mut Grid<bool>) {
    let _scope = info_span!("keep_one_island").entered();

    loop {
        let (freq, labels) = connected_components(grid);
        if freq.len() <= 2 {
            break;
        }

        let island_label = freq.iter().filter(|v| v.1).max_by_key(|v| v.2).unwrap().0;
        let water_label = freq.iter().filter(|v| !v.1).max_by_key(|v| v.2).unwrap().0;

        for (cell, value) in grid.entries_mut() {
            let label = labels[cell];
            if label != island_label && label != water_label {
                *value = !*value;
            }
        }
    }
}

fn connected_components(grid: &Grid<bool>) -> (Vec<(u32, bool, u32)>, Grid<u32>) {
    let _scope = info_span!("connected_components").entered();

    let mut frequencies = Vec::with_capacity(32);
    let mut labels = Grid::new(grid.size(), u32::MAX);
    let mut num_labels = 0;

    let mut stack = Vec::with_capacity(grid.data().len());

    for cell in grid.cells() {
        if labels[cell] != u32::MAX {
            continue;
        }

        let label = num_labels;
        num_labels += 1;

        let mut frequency = 0;

        stack.clear();
        stack.push(cell);

        while let Some(cell) = stack.pop() {
            if labels[cell] == label {
                continue;
            }

            frequency += 1;
            labels[cell] = label;

            for (_, neighbor) in grid.neighborhood_4(cell) {
                if grid[cell] == grid[neighbor] {
                    stack.push(neighbor);
                }
            }
        }

        frequencies.push((label, grid[cell], frequency));
    }

    (frequencies, labels)
}

fn is_island_area_good(grid: &Grid<bool>, settings: &IslandSettings) -> bool {
    let _scope = info_span!("is_island_area_good").entered();

    let size = grid.size().as_vec2();
    let area = grid.data().iter().filter(|v| **v).count();
    let percentage = area as f32 / (size.x * size.y);

    settings.min_area <= percentage && percentage <= settings.max_area
}

fn random_zoom<R: Rng>(rng: &mut R, grid: &mut Grid<bool>) {
    let _scope = info_span!("random_zoom").entered();

    let mut res = Grid::new(grid.size() * 2, false);

    for cell in grid.cells() {
        let mut sum = grid[cell] as u8;
        let mut count = 1.0;

        for (_, neighbor) in grid.neighborhood_4(cell) {
            sum += grid[neighbor] as u8;
            count += 1.0;
        }

        let p = (sum as f64) / count;
        res[2 * cell + IVec2::new(0, 0)] = rng.gen_bool(p);
        res[2 * cell + IVec2::new(0, 1)] = rng.gen_bool(p);
        res[2 * cell + IVec2::new(1, 0)] = rng.gen_bool(p);
        res[2 * cell + IVec2::new(1, 1)] = rng.gen_bool(p);
    }

    *grid = res;
}

fn erode(grid: &mut Grid<bool>) {
    let _scope = info_span!("erode").entered();
    *grid = Grid::par_from_fn(grid.size(), |cell| {
        if !grid[cell] {
            return false;
        }

        for (_, neighbor) in grid.neighborhood_4(cell) {
            if !grid[neighbor] {
                return false;
            }
        }

        true
    });
}

fn smooth(grid: &mut Grid<bool>) {
    let _scope = info_span!("smooth").entered();
    *grid = Grid::par_from_fn(grid.size(), |cell| {
        let mut num_true = 0;
        let mut num_false = 0;

        for (_, neighbor) in grid.neighborhood_8(cell) {
            if grid[neighbor] {
                num_true += 1;
            } else {
                num_false += 1;
            }
        }

        num_true > num_false
    });
}

fn generate_sdf(island: &Grid<bool>) -> Grid<f32> {
    let _scope = info_span!("generate_sdf").entered();

    let (edt, inv_edt) = rayon::join(
        || {
            island.compute_edt(EdtSettings {
                invert: false,
                normalize: false,
                padding: 0,
            })
        },
        || {
            island.compute_edt(EdtSettings {
                invert: true,
                normalize: false,
                padding: 128,
            })
        },
    );

    Grid::from_fn(island.size(), |cell| {
        if island[cell] {
            edt[cell]
        } else {
            -inv_edt[cell]
        }
    })
}

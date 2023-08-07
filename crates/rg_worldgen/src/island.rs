use bevy::prelude::*;
use rand::Rng;
use rg_core::progress::ProgressStage;
use rg_core::{EdtSettings, Grid};
use serde::Deserialize;

use crate::NoiseMaps;

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

pub fn generate_island_map<R: Rng>(
    rng: &mut R,
    progress: &mut ProgressStage,
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
        progress.task(|| remove_holes(&mut grid));

        progress.task(|| random_zoom(rng, &mut grid));
        progress.task(|| random_zoom(rng, &mut grid));
        progress.task(|| random_zoom(rng, &mut grid));

        progress.task(|| remove_holes(&mut grid));
        progress.task(|| remove_small_islands(settings, &mut grid));

        if !check_total_area(&grid, settings) {
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

fn remove_holes(grid: &mut Grid<bool>) {
    let _scope = info_span!("remove_holes").entered();

    let mut island = Grid::new(grid.size(), true);
    floodfill(grid, false, &mut island, false, IVec2::ZERO);

    *grid = island;
}

fn remove_small_islands(settings: &IslandSettings, grid: &mut Grid<bool>) {
    let _scope = info_span!("remove_small_islands").entered();

    let size = grid.size();
    let mut visited = Grid::new(size, false);

    for cell in visited.cells() {
        if visited[cell] || !grid[cell] {
            continue;
        }

        let count = floodfill(&grid, true, &mut visited, true, cell);
        let area = (count as f32) / (size.x as f32) / (size.y as f32);
        if area < settings.min_island_area {
            floodfill(&visited, true, grid, false, cell);
        }
    }
}

fn floodfill(
    src: &Grid<bool>,
    src_value: bool,
    dst: &mut Grid<bool>,
    dst_value: bool,
    pos: IVec2,
) -> usize {
    let inside = |dst: &Grid<bool>, x, y| {
        let pos = IVec2::new(x, y);
        src.get(pos) == Some(&src_value) && dst.get(pos) != Some(&dst_value)
    };

    let mut count = 0;

    let mut set = |dst: &mut Grid<bool>, x, y| {
        let pos = IVec2::new(x, y);
        dst.set(pos, dst_value);
        count += 1;
    };

    if !inside(dst, pos.x, pos.y) {
        return 0;
    }

    let mut stack = Vec::new();
    stack.push((pos.x, pos.x, pos.y, 1));
    stack.push((pos.x, pos.x, pos.y - 1, -1));

    while let Some((mut x1, x2, y, dy)) = stack.pop() {
        let mut x = x1;

        if inside(dst, x, y) {
            while inside(dst, x - 1, y) {
                set(dst, x - 1, y);
                x -= 1;
            }

            if x < x1 {
                stack.push((x, x1 - 1, y - dy, -dy));
            }
        }

        while x1 <= x2 {
            while inside(dst, x1, y) {
                set(dst, x1, y);
                x1 += 1;
            }

            if x1 > x {
                stack.push((x, x1 - 1, y + dy, dy));
            }

            if x1 - 1 > x2 {
                stack.push((x2 + 1, x1 - 1, y - dy, -dy));
            }

            x1 += 1;
            while x1 < x2 && !inside(dst, x1, y) {
                x1 += 1;
            }

            x = x1;
        }
    }

    count
}

fn check_total_area(grid: &Grid<bool>, settings: &IslandSettings) -> bool {
    let _scope = info_span!("check_total_area").entered();

    let size = grid.size().as_vec2();
    let count = grid.data().iter().filter(|v| **v).count();
    let area = count as f32 / (size.x * size.y);

    settings.min_total_area <= area && area <= settings.max_total_area
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

use bevy::prelude::{UVec2, Vec2};
use rand::Rng;
use rg_core::Grid;

pub fn shape_island<R: Rng>(rng: &mut R, size: UVec2) -> Grid<bool> {
    loop {
        let mut grid = Grid::new(size, 0.0);
        let scale = size.x.min(size.y) as f32 / 6000.0;
        grid.add_fbm_noise(rng, scale, 1.0, 8);
        voronoi_reshape(rng, &mut grid);

        let mut grid = grid.to_binary(0.4);
        keep_one_island(&mut grid);

        if is_valid(&grid) {
            return grid;
        }
    }
}

fn voronoi_reshape<R: Rng>(rng: &mut R, grid: &mut Grid<f32>) {
    let size = grid.size().as_vec2();
    let margin = f32::min(size.x, size.y) * 0.3;

    let mut points = [Vec2::ZERO; 20];
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
        let inv_dist = 1.0 - sq_dist.sqrt() / (size.x.min(size.y) * 0.3);

        let alpha = 0.5;
        *value = *value * (1.0 - alpha) + inv_dist * alpha;
    }
}

fn keep_one_island(grid: &mut Grid<bool>) {
    loop {
        let (freq, labels) = connected_components(&grid);
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

fn connected_components(grid: &Grid<bool>) -> (Vec<(u16, bool, u16)>, Grid<u16>) {
    let mut frequencies = Vec::with_capacity(32);
    let mut labels = Grid::new(grid.size(), u16::MAX);
    let mut num_labels = 0;

    let mut stack = Vec::with_capacity(grid.data().len());

    for cell in grid.cells() {
        if labels[cell] != u16::MAX {
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

            for (_, neighbor) in grid.neighborhood_8(cell) {
                if grid[cell] == grid[neighbor] {
                    stack.push(neighbor);
                }
            }
        }

        frequencies.push((label, grid[cell], frequency));
    }

    (frequencies, labels)
}

fn is_valid(grid: &Grid<bool>) -> bool {
    let size = grid.size().as_vec2();
    let area = grid.data().iter().filter(|v| **v).count();
    let percentage = area as f32 / (size.x * size.y);

    0.4 <= percentage && percentage <= 0.6
}

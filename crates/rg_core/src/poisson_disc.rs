use std::f32::consts::SQRT_2;

use bevy::prelude::{info_span, IVec2, Vec2};
use rand::{Rng, SeedableRng};

use crate::Grid;

const MAX_TRIES: u32 = 8;

#[derive(Debug)]
pub struct PoissonDiscSampling {
    pub cell_size: f32,
    pub grid: Grid<Vec2>,
    pub points: Vec<Vec2>,
}

impl PoissonDiscSampling {
    pub fn new<R: Rng>(rng: &mut R, size: Vec2, min_radius: f32) -> PoissonDiscSampling {
        let _span = info_span!("poisson disc sampling").entered();

        let min_radius_squared = min_radius.powi(2);

        let cell_size = min_radius / SQRT_2;
        let mut grid = Grid::new((size / cell_size).ceil().as_uvec2(), Vec2::NAN);

        let mut points = Vec::new();
        let mut active_set = Vec::new();

        let seed = sample_seed(rng) * size;
        points.push(seed);

        generate_borders(&mut points, min_radius, size);

        for &point in &points {
            active_set.push(point);
            let cell = (point / cell_size).as_ivec2();
            grid[cell] = point;
        }

        'outer: while !active_set.is_empty() {
            let active_idx = active_set.len() - 1;
            let active = active_set[active_idx];

            for _ in 0..MAX_TRIES {
                let neighbor = active + sample_disc(rng, min_radius);

                if neighbor.x < 0.0
                    || neighbor.y < 0.0
                    || neighbor.x >= size.x
                    || neighbor.y >= size.y
                {
                    continue;
                }

                let neighbor_cell = (neighbor / cell_size).as_ivec2();
                let mut is_valid = true;

                'check: for sx in -1..=1 {
                    for sy in -1..=1 {
                        let mut pos = neighbor;
                        let mut cell = neighbor_cell + IVec2::new(sx, sy);

                        if cell.x >= grid.size().x as i32 {
                            cell.x = grid.size().x as i32 - cell.x;
                            pos.x = size.x - pos.x;
                        }

                        if cell.y >= grid.size().y as i32 {
                            cell.y = grid.size().y as i32 - cell.y;
                            pos.y = size.y - pos.y;
                        }

                        if let Some(v) = grid.get(cell) {
                            if !v.is_nan() && v.distance_squared(pos) < min_radius_squared {
                                is_valid = false;
                                break 'check;
                            }
                        }
                    }
                }

                if is_valid {
                    active_set.push(neighbor);
                    points.push(neighbor);
                    grid[neighbor_cell] = neighbor;
                    continue 'outer;
                }
            }

            active_set.swap_remove(active_idx);
        }

        PoissonDiscSampling {
            cell_size,
            grid,
            points,
        }
    }
}

fn generate_borders(points: &mut Vec<Vec2>, min_radius: f32, size: Vec2) {
    let mut rng = rand_pcg::Pcg32::seed_from_u64(0);

    let top_left = Vec2::new(rng.gen_range(0.0..0.5), rng.gen_range(0.0..0.5)) * min_radius;
    points.push(top_left);

    // left border
    let mut prev = top_left;
    loop {
        let x = rng.gen_range(0.0..0.5 * min_radius);
        let dist = rng.gen_range(2.0..4.0) * min_radius;
        let y = prev.y + dist.hypot(prev.x - x);
        let point = Vec2::new(x, y);
        if point.y >= size.y {
            break;
        }
        points.push(point);
        prev = point;
    }

    // top border
    let mut prev = top_left;
    loop {
        let y = rng.gen_range(0.0..0.5 * min_radius);
        let dist = rng.gen_range(2.0..4.0) * min_radius;
        let x = prev.x + dist.hypot(prev.y - y);
        let point = Vec2::new(x, y);
        if point.x >= size.x {
            break;
        }
        points.push(point);
        prev = point;
    }
}

fn sample_seed<R: Rng>(rng: &mut R) -> Vec2 {
    Vec2::new(rng.gen_range(0.45..0.55), rng.gen_range(0.45..0.55))
}

fn sample_disc<R: Rng>(rng: &mut R, min_radius: f32) -> Vec2 {
    let mut vector;
    loop {
        vector = Vec2::new(rng.gen_range(-1.0..=1.0), rng.gen_range(-1.0..=1.0));
        let length_sq = vector.length_squared();
        if (0.5..1.0).contains(&length_sq) {
            break;
        }
    }
    vector * min_radius * 2.0
}

use std::f32::consts::SQRT_2;

use bevy::prelude::{IVec2, Vec2};
use rand::Rng;

use crate::Grid;

const MAX_TRIES: u32 = 16;

#[derive(Debug)]
pub struct PoissonDiscSampling {
    pub cell_size: f32,
    pub grid: Grid<Vec2>,
    pub points: Vec<Vec2>,
}

impl PoissonDiscSampling {
    pub fn new<R: Rng>(rng: &mut R, size: Vec2, min_radius: f32) -> PoissonDiscSampling {
        let min_radius_squared = min_radius.powi(2);

        let cell_size = min_radius / SQRT_2;
        let mut grid = Grid::new((size / cell_size).ceil().as_uvec2(), Vec2::NAN);

        let mut points = Vec::new();
        let mut active_set = Vec::new();

        let seed = sample_seed(rng) * size;
        active_set.push(seed);
        points.push(seed);
        grid[(seed / cell_size).as_ivec2()] = seed;

        'outer: while !active_set.is_empty() {
            let active_idx = active_set.len() - 1;
            let active = active_set[active_idx];

            for _ in 0..MAX_TRIES {
                let neighbor = active + sample_disc(rng, min_radius);

                if neighbor.x < 0.0
                    || neighbor.y < 0.0
                    || neighbor.x >= size.x - min_radius * 0.5
                    || neighbor.y >= size.y - min_radius * 0.5
                {
                    continue;
                }

                let neighbor_cell = (neighbor / cell_size).as_ivec2();
                let mut is_valid = true;

                'check: for sx in -1..=1 {
                    for sy in -1..=1 {
                        let cell = neighbor_cell + IVec2::new(sx, sy);
                        if let Some(v) = grid.get(cell) {
                            if !v.is_nan() && v.distance_squared(neighbor) < min_radius_squared {
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

fn sample_seed<R: Rng>(rng: &mut R) -> Vec2 {
    Vec2::new(rng.gen_range(0.25..=0.75), rng.gen_range(0.25..=0.75))
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

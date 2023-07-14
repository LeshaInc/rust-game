use std::f32::consts::SQRT_2;

use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

use crate::Grid;

#[derive(Debug)]
pub struct PoissonDiscSampling {
    pub cell_size: f32,
    pub grid: Grid<Vec2>,
    pub points: Vec<Vec2>,
}

impl PoissonDiscSampling {
    pub fn new<R: Rng>(
        rng: &mut R,
        size: Vec2,
        min_dist: f32,
        max_tries: u32,
    ) -> PoissonDiscSampling {
        Self::new_tileable(rng.gen(), IVec2::ZERO, size, min_dist, max_tries)
    }

    pub fn new_tileable(
        seed: u64,
        chunk_pos: IVec2,
        size: Vec2,
        min_dist: f32,
        max_tries: u32,
    ) -> PoissonDiscSampling {
        let _span = info_span!("poisson disc sampling").entered();

        let mut rng =
            Pcg32::seed_from_u64(seed | (chunk_pos.x as u64) | (chunk_pos.y as u64) << 32);

        let min_dist_squared = min_dist.powi(2);

        let cell_size = min_dist / SQRT_2;
        let grid_size = ((size * 3.0) / cell_size).ceil().as_uvec2();
        let mut grid = Grid::new(grid_size, Vec2::NAN);

        let mut points = Vec::new();
        let mut active_set: Vec<Vec2> = Vec::new();

        for (dir, mask) in [
            (IVec2::ZERO, BVec2::new(true, true)),
            (IVec2::X, BVec2::new(false, true)),
            (IVec2::Y, BVec2::new(true, false)),
        ] {
            generate_borders(
                &mut points,
                seed,
                chunk_pos + dir,
                min_dist,
                size,
                size * dir.as_vec2(),
                mask,
            );
        }

        for &point in &points {
            let cell = (point / cell_size).as_ivec2();
            grid[cell] = point;
        }

        points.retain(|pt| pt.x >= 0.0 && pt.y >= 0.0 && pt.x < size.x && pt.y < size.y);

        'outer: for i in 0..max_tries {
            let center = Vec2::new(rng.gen_range(0.3..0.7), rng.gen_range(0.3..0.7)) * size;
            let center_cell = (center / cell_size).as_ivec2();

            if i < max_tries - 1 {
                for sx in -1..=1 {
                    for sy in -1..=1 {
                        let cell = center_cell + IVec2::new(sx, sy);
                        if let Some(v) = grid.get(cell) {
                            if !v.is_nan() && v.distance_squared(center) < min_dist_squared {
                                continue 'outer;
                            }
                        }
                    }
                }
            }
            points.push(center);
            active_set.push(center);
            grid[center_cell] = center;
            break;
        }

        'outer: while !active_set.is_empty() {
            let active_idx = active_set.len() - 1;
            let active = active_set[active_idx];

            for _ in 0..max_tries {
                let neighbor = active + sample_disc(&mut rng, min_dist);

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
                        let cell = neighbor_cell + IVec2::new(sx, sy);
                        if let Some(v) = grid.get(cell) {
                            if !v.is_nan() && v.distance_squared(neighbor) < min_dist_squared {
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

fn generate_borders(
    points: &mut Vec<Vec2>,
    seed: u64,
    chunk_pos: IVec2,
    min_dist: f32,
    size: Vec2,
    offset: Vec2,
    mask: BVec2,
) {
    let min_dist2 = min_dist.powi(2);

    let mut rng = Pcg32::seed_from_u64(seed ^ (chunk_pos.x as u64) ^ ((chunk_pos.y as u64) << 32));
    let top_left = Vec2::new(rng.gen(), rng.gen()) * 0.5 * min_dist;
    points.push(top_left + offset);

    let mut bottom_rng =
        Pcg32::seed_from_u64(seed ^ (chunk_pos.x as u64) ^ (((chunk_pos.y + 1) as u64) << 32));
    let bottom = size * Vec2::Y + Vec2::new(bottom_rng.gen(), bottom_rng.gen()) * 0.5 * min_dist;

    let mut right_rng =
        Pcg32::seed_from_u64(seed ^ ((chunk_pos.x + 1) as u64) ^ ((chunk_pos.y as u64) << 32));
    let right = size * Vec2::X + Vec2::new(right_rng.gen(), right_rng.gen()) * 0.5 * min_dist;

    if !mask.x {
        points.push(bottom + offset);
    }

    if mask.x {
        let mut prev = top_left;
        loop {
            let y = rng.gen_range(0.0..min_dist * 0.7);
            let dist = rng.gen_range(1.1 * min_dist..1.5 * min_dist);
            let x = prev.x + dist.hypot(prev.y - y);
            let point = Vec2::new(x, y);
            if point.x >= size.x || (point - right).length_squared() < min_dist2 {
                break;
            }
            points.push(point + offset);
            prev = point;
        }
    }

    if mask.y {
        let mut prev = top_left;
        loop {
            let x = rng.gen_range(0.0..min_dist * 0.7);
            let dist = rng.gen_range(1.1 * min_dist..1.5 * min_dist);
            let y = prev.y + dist.hypot(prev.x - x);
            let point = Vec2::new(x, y);
            if point.y >= size.y || (point - bottom).length_squared() < min_dist2 {
                break;
            }
            points.push(point + offset);
            prev = point;
        }
    }
}

fn sample_disc<R: Rng>(rng: &mut R, min_dist: f32) -> Vec2 {
    let mut vector;
    loop {
        vector = Vec2::new(rng.gen_range(-1.0..=1.0), rng.gen_range(-1.0..=1.0));
        let length_sq = vector.length_squared();
        if (0.5..1.0).contains(&length_sq) {
            break;
        }
    }
    vector * min_dist * 2.0
}

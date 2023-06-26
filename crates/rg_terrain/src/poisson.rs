use std::f32::consts::SQRT_2;

use bevy::prelude::*;
use turborand::rng::Rng;
use turborand::TurboRand;

const MAX_TRIES: u32 = 10;

pub struct PoissonGrid {
    cell_size: f32,
    resolution: usize,
    points: Vec<Vec2>,
}

impl PoissonGrid {
    pub fn new(cell_size: f32) -> PoissonGrid {
        let resolution = (1.0 / cell_size).ceil() as usize;
        PoissonGrid {
            cell_size,
            resolution,
            points: vec![Vec2::NAN; 2 * resolution * resolution],
        }
    }

    pub fn resolution(&self) -> usize {
        self.resolution
    }

    pub fn cell_pos(&self, point: Vec2) -> IVec2 {
        (point / self.cell_size).floor().as_ivec2()
    }

    pub fn cell_index(&self, cell: IVec2) -> usize {
        (cell.x as usize) * self.resolution + (cell.y as usize)
    }

    pub fn is_inside(&self, cell: IVec2) -> bool {
        cell.x >= 0
            && (cell.x as usize) < self.resolution
            && cell.y >= 0
            && (cell.y as usize) < self.resolution
    }

    pub fn add(&mut self, point: Vec2) {
        let cell = self.cell_pos(point);
        let index = self.cell_index(cell);
        self.points[index] = point;
    }

    pub fn get(&self, cell: IVec2) -> Option<Vec2> {
        if self.is_inside(cell) {
            let index = self.cell_index(cell);
            let point = self.points[index];
            Some(point).filter(|p| !p.x.is_nan())
        } else {
            None
        }
    }

    pub fn has_near(&self, point: Vec2, min_radius: f32) -> bool {
        let cell = self.cell_pos(point);

        let min_radius_sq = min_radius * min_radius;

        for sx in -1..=1 {
            for sy in -1..=1 {
                let neighbor = cell + IVec2::new(sx, sy);
                if !self.is_inside(neighbor) {
                    continue;
                }

                let index = self.cell_index(neighbor);
                let neighbor = self.points[index];
                if neighbor.distance_squared(point) < min_radius_sq {
                    return true;
                }
            }
        }

        false
    }
}

pub fn poisson_disc_sampling(rng: &mut Rng, min_radius: f32) -> PoissonGrid {
    let _span = info_span!("poisson disc sampling").entered();

    let cell_size = min_radius / SQRT_2;
    let mut grid = PoissonGrid::new(cell_size);

    let mut active_set = Vec::new();

    let seed = sample_seed(rng);
    active_set.push(seed);
    grid.add(seed);

    'outer: while !active_set.is_empty() {
        let active_idx = active_set.len() - 1;
        let active = active_set[active_idx];

        for _ in 0..MAX_TRIES {
            let neighbor = active + sample_disc(rng, min_radius);

            if neighbor.x < 0.0
                || neighbor.y < 0.0
                || neighbor.x >= 1.0 - min_radius * 0.5
                || neighbor.y >= 1.0 - min_radius * 0.5
            {
                continue;
            }

            if !grid.has_near(neighbor, min_radius) {
                active_set.push(neighbor);
                grid.add(neighbor);
                continue 'outer;
            }
        }

        active_set.swap_remove(active_idx);
    }

    grid
}

fn sample_seed(rng: &mut Rng) -> Vec2 {
    Vec2::new(rng.f32(), rng.f32()) * 0.5 + 0.25
}

fn sample_disc(rng: &mut Rng, min_radius: f32) -> Vec2 {
    let mut vector;
    loop {
        vector = Vec2::new(rng.f32_normalized(), rng.f32_normalized());
        let length_sq = vector.length_squared();
        if (0.5..1.0).contains(&length_sq) {
            break;
        }
    }
    vector * min_radius * 2.0
}

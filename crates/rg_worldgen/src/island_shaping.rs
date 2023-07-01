use bevy::prelude::{UVec2, Vec2};
use rand::Rng;
use rg_core::Grid;

pub fn shape_island<R: Rng>(rng: &mut R, size: UVec2) -> Grid<bool> {
    let mut grid = Grid::new(size, 0.0);

    grid.add_fbm_noise(rng, 0.005, 1.0, 8);

    voronoi_reshape(rng, &mut grid);

    let mut grid = grid.to_binary(0.5);
    grid.erode();

    grid
}

fn voronoi_reshape<R: Rng>(rng: &mut R, grid: &mut Grid<f32>) {
    let size = grid.size().as_vec2();
    let margin = f32::min(size.x, size.y) * 0.4;

    let mut points = [Vec2::ZERO; 20];
    for point in &mut points {
        point.x = rng.gen_range(margin..=(size.x - margin));
        point.y = rng.gen_range(margin..=(size.y - margin));
    }

    for (cell, value) in grid.entries_mut() {
        let pos = cell.as_vec2();
        let sq_dist = points
            .iter()
            .map(|p| p.distance_squared(pos))
            .fold(f32::INFINITY, f32::min);
        let inv_dist = 1.0 - sq_dist.sqrt() / 300.0;

        let alpha = 0.5;
        *value = *value * (1.0 - alpha) + inv_dist * alpha;
    }
}

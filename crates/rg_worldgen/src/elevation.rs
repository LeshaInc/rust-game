use bevy::prelude::IVec2;
use rg_core::Grid;

pub fn compute_elevation(island: &Grid<bool>) -> Grid<f32> {
    let data = edt::edt_fmm(
        island.data(),
        (island.size().x as usize, island.size().y as usize),
        false,
    );

    let data = data.into_iter().map(|v| v as f32).collect::<Vec<_>>();

    let mut grid = Grid::from_data(island.size(), &data);

    blur(&mut grid, 10);
    blur(&mut grid, 10);

    grid
}

fn blur(grid: &mut Grid<f32>, kernel_size: i32) {
    let size = grid.size().as_ivec2();
    let mut res = grid.clone();

    for y in 0..size.y {
        for x in kernel_size..size.x - kernel_size {
            let cell = IVec2::new(x, y);

            let mut sum = 0.0;
            for sx in -kernel_size..=kernel_size {
                sum += grid[cell + IVec2::new(sx, 0)];
            }

            res[cell] = sum / (2 * kernel_size + 1) as f32;
        }
    }

    for x in 0..size.x {
        for y in kernel_size..size.y - kernel_size {
            let cell = IVec2::new(x, y);

            let mut sum = 0.0;
            for sy in -kernel_size..=kernel_size {
                sum += res[cell + IVec2::new(0, sy)];
            }

            grid[cell] = sum / (2 * kernel_size + 1) as f32;
        }
    }
}

use std::f32::consts::PI;

use rg_core::Grid;

pub fn compute_elevation(island: &Grid<bool>) -> Grid<f32> {
    let mut elevation = island
        .to_f32()
        .resize(island.size() / 4)
        .to_bool(0.5)
        .compute_edt()
        .resize(island.size());

    reshape(&mut elevation, island);
    elevation.blur(3);
    elevation.blur(3);

    elevation
}

fn reshape(elevation: &mut Grid<f32>, island: &Grid<bool>) {
    for (cell, height) in elevation.entries_mut() {
        if !island[cell] {
            *height = 0.0;
        }

        let beach_size = 0.3;
        let inland_height = 0.3;

        *height = if *height < beach_size {
            (0.5 - 0.5 * (*height * PI / beach_size).cos()) * inland_height
        } else {
            height.powi(4) + inland_height
        };
    }
}

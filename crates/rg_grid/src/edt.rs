use bevy::prelude::*;
use rayon::prelude::*;

use crate::Grid;

impl Grid<bool> {
    pub fn compute_edt(&self, settings: EdtSettings) -> Grid<f32> {
        let _scope = info_span!("compute_edt").entered();

        let mut tmp_grid = Grid::from_fn(self.size + settings.padding * 2, |cell| {
            let orig_cell = cell - (settings.padding as i32);
            if settings.invert ^ *self.clamped_get(orig_cell) {
                f32::MAX
            } else {
                0.0
            }
        });

        dt2d_float(&mut tmp_grid);

        let mut res_grid = Grid::from_fn(self.size, |cell| {
            tmp_grid
                .clamped_get(cell + (settings.padding as i32))
                .sqrt()
        });

        if settings.normalize {
            res_grid.map_range_inplace(0.0, 1.0);
        }

        res_grid.with_origin(self.origin)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EdtSettings {
    pub invert: bool,
    pub normalize: bool,
    pub padding: u32,
}

fn dt1d_float(d: &mut [f32], v: &mut [i32], z: &mut [f32], f: &[f32]) {
    d.fill(0.0);
    v.fill(0);
    z.fill(0.0);

    let mut k = 0;
    v[0] = 0;
    z[0] = f32::MIN;
    z[1] = f32::MAX;

    for q in 1..f.len() {
        let mut s = ((f[q] + (q * q) as f32) - (f[v[k] as usize] + (v[k] * v[k]) as f32))
            / (2 * q as i32 - 2 * v[k]) as f32;
        while s <= z[k] {
            k -= 1;
            s = ((f[q] + (q * q) as f32) - (f[v[k] as usize] + (v[k] * v[k]) as f32))
                / (2 * q as i32 - 2 * v[k]) as f32;
        }
        k += 1;
        v[k] = q as i32;
        z[k] = s;
        z[k + 1] = f32::MAX;
    }

    k = 0;
    for q in 0..f.len() {
        while z[k + 1] <= q as f32 {
            k += 1;
        }
        d[q] = ((q as i32 - v[k]) * (q as i32 - v[k])) as f32 + f[v[k] as usize];
    }
}

fn dt2d_float(grid: &mut Grid<f32>) {
    let max_size = UVec2::splat(grid.size.max_element());

    let mut d = Grid::new(max_size, 0.0);
    let mut v = Grid::new(max_size, 0);
    let mut z = Grid::new(max_size + 1, 0.0);
    let mut f = Grid::new(max_size, 0.0);

    for _ in 0..2 {
        (
            grid.par_rows_mut(),
            d.par_rows_mut(),
            v.par_rows_mut(),
            z.par_rows_mut(),
            f.par_rows_mut(),
        )
            .into_par_iter()
            .for_each(|(row, d, v, z, f)| {
                let d = &mut d[..row.len()];
                let v = &mut v[..row.len()];
                let z = &mut z[..row.len() + 1];
                let f = &mut f[..row.len()];
                f.copy_from_slice(row);
                dt1d_float(d, v, z, f);
                row.copy_from_slice(d);
            });

        grid.transpose_in_place();
    }
}

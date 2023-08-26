use std::path::Path;

use bevy::prelude::*;
use bytemuck::cast_slice;

use super::Grid;
use crate::noise::Noise;

impl Grid<f32> {
    pub fn add_noise<N: Noise<1> + Sync>(&mut self, noise: &N) {
        let _scope = info_span!("add_noise").entered();

        self.par_map_inplace(|cell, value| {
            *value += noise.get(cell.as_vec2())[0];
        });
    }

    pub fn sample(&self, pos: Vec2) -> f32 {
        let ipos = pos.as_ivec2();
        let fpos = pos - ipos.as_vec2();

        let tl = *self.clamped_get(ipos + IVec2::new(0, 0));
        let tr = *self.clamped_get(ipos + IVec2::new(1, 0));
        let bl = *self.clamped_get(ipos + IVec2::new(0, 1));
        let br = *self.clamped_get(ipos + IVec2::new(1, 1));

        let vals = [tl, tr, bl, br];
        if vals.iter().any(|v| v.is_nan()) {
            return *vals.iter().find(|v| !v.is_nan()).unwrap_or(&f32::NAN);
        }

        lerp(lerp(tl, tr, fpos.x), lerp(bl, br, fpos.x), fpos.y)
    }

    pub fn sample_grad(&self, pos: Vec2) -> Vec2 {
        let l = self.sample(pos - Vec2::X);
        let r = self.sample(pos + Vec2::X);
        let t = self.sample(pos - Vec2::Y);
        let b = self.sample(pos + Vec2::Y);
        Vec2::new((r - l) * 0.5, (b - t) * 0.5)
    }

    pub fn resize(&self, new_size: UVec2) -> Grid<f32> {
        let _scope = info_span!("resize").entered();

        let mut res = Grid::new(new_size, 0.0);
        let scale = self.size.as_vec2() / new_size.as_vec2();

        for cell in res.cells() {
            let pos = cell.as_vec2() * scale;
            res[cell] = self.sample(pos);
        }

        res
    }

    pub fn min_value(&self) -> f32 {
        self.values().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_value(&self) -> f32 {
        self.values().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn to_bool(&self, cutoff: f32) -> Grid<bool> {
        self.map(|_, &value| value > cutoff)
    }

    pub fn map_range(&self, new_min: f32, new_max: f32) -> Grid<f32> {
        let mut grid = self.clone();
        grid.map_range_inplace(new_min, new_max);
        grid
    }

    pub fn map_range_inplace(&mut self, new_min: f32, new_max: f32) {
        let min = self.min_value();
        let max = self.max_value();
        for val in self.values_mut() {
            *val = (*val - min) / (max - min) * (new_max - new_min) + new_min;
        }
    }

    pub fn debug_save(&self, path: impl AsRef<Path>) {
        if !cfg!(debug_assertions) {
            return;
        }

        let _scope = info_span!("debug_save").entered();

        let min_value = self.min_value();
        let max_value = self.max_value();

        let colors = self.par_map(|_, &v| {
            let min_color = Color::rgb_u8(40, 138, 183).as_rgba_linear();
            let mid_color = Color::rgb_u8(0, 0, 0).as_rgba_linear();
            let max_color = Color::rgb_u8(255, 255, 255).as_rgba_linear();

            let color = if v >= 0.0 {
                mid_color * (1.0 - v / max_value) + max_color * (v / max_value)
            } else {
                mid_color * (1.0 - v / min_value) + min_color * (v / min_value)
            };

            [
                (color.r() * 255.0) as u8,
                (color.g() * 255.0) as u8,
                (color.b() * 255.0) as u8,
            ]
        });

        image::save_buffer(
            path,
            cast_slice(&colors.data),
            self.size.x,
            self.size.y,
            image::ColorType::Rgb8,
        )
        .unwrap();
    }
}

// TODO: move this somewhere else
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

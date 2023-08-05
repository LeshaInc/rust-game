use std::f32::consts::LOG2_E;

use bevy::prelude::*;
use rayon::prelude::*;

use crate::Grid;

const MAX_KERNEL_SIZE: usize = 49;

impl Grid<f32> {
    pub fn blur(&mut self, kernel_size: i32) {
        let _scope = info_span!("blur").entered();

        let mut temp = self.clone();

        temp.par_entries_mut().for_each(|(cell, value)| {
            let mut sum = 0.0;

            for sx in -kernel_size..=kernel_size {
                sum += self.clamped_get(cell + IVec2::new(sx, 0));
            }

            *value = sum / (2 * kernel_size + 1) as f32;
        });

        self.par_entries_mut().for_each(|(cell, value)| {
            let mut sum = 0.0;

            for sy in -kernel_size..=kernel_size {
                sum += temp.clamped_get(cell + IVec2::new(0, sy));
            }

            *value = sum / (2 * kernel_size + 1) as f32;
        });
    }

    pub fn gaussian_blur(&mut self, sigma: f32) {
        let _scope = info_span!("gaussian_blur").entered();

        let mut kernel = [0.0; MAX_KERNEL_SIZE];
        let kernel_size = gaussian_kernel_size(sigma);
        let kernel = &mut kernel[..kernel_size];
        compute_gaussian_kernel(sigma, kernel);

        let kernel_radius = (kernel_size / 2) as i32;

        let mut temp = self.clone();

        temp.par_entries_mut().for_each(|(cell, value)| {
            let mut sum = 0.0;

            for (i, k) in kernel.iter().enumerate() {
                let sx = (i as i32) - (kernel_radius / 2) as i32;
                sum += k * self.clamped_get(cell + IVec2::new(sx, 0));
            }

            *value = sum;
        });

        self.par_entries_mut().for_each(|(cell, value)| {
            let mut sum = 0.0;

            for (i, k) in kernel.iter().enumerate() {
                let sy = (i as i32) - (kernel_radius / 2) as i32;
                sum += k * temp.clamped_get(cell + IVec2::new(0, sy));
            }

            *value = sum;
        });
    }
}

fn gaussian_kernel_size(sigma: f32) -> usize {
    let v = (2.0 * (sigma * 2.0).ceil() + 1.0) as usize;
    v.max(3).min(MAX_KERNEL_SIZE)
}

fn compute_gaussian_kernel(sigma: f32, out: &mut [f32]) {
    if out.len() < 3 {
        return;
    }

    let mid = out.len() / 2;
    out[mid] = 1.0;

    let (left, rest) = out.split_at_mut(mid);
    let right = &mut rest[1..];

    let denom = 2.0 * sigma * sigma;

    for (i, l) in left.iter_mut().enumerate() {
        let x = (mid - i) as f32;
        let v = fast_exp(-x * x / denom);
        *l = v;
    }

    let mut sum = 1.0;

    for (&l, r) in left.iter().zip(right.iter_mut().rev()) {
        *r = l;
        sum += l;
        sum += l;
    }

    for v in out {
        *v /= sum;
    }
}

// works only for -104 <= x <= 104
fn fast_exp(x: f32) -> f32 {
    const A: f32 = (1 << 23) as f32;
    const MASK: i32 = 0xff800000u32 as i32;
    const EXP2_23: f32 = 1.1920929e-7;
    const C0: f32 = 0.3371894346 * EXP2_23 * EXP2_23;
    const C1: f32 = 0.657636276 * EXP2_23;
    const C2: f32 = 1.00172476;

    let a = A * LOG2_E;
    let mul = (a * x) as i32;
    let floor = mul & MASK;
    let frac = (mul - floor) as f32;

    let approx = (C0 * frac + C1) * frac + C2;
    f32::from_bits(approx.to_bits().wrapping_add(floor as u32))
}

#![allow(clippy::excessive_precision)]

use std::num::Wrapping;

use bevy::math::Vec2;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg32;

const PRIME_X: Wrapping<i64> = Wrapping(0x5205402B9270C86F);
const PRIME_Y: Wrapping<i64> = Wrapping(0x598CD327003817B5);
const HASH_PRIME: Wrapping<i64> = Wrapping(0x53A3F72DEEC546F5);

const RSQUARED_2D: f32 = 2.0 / 3.0;
const SKEW_2D: f32 = 0.366025403784439;
const UNSKEW_2D: f32 = -0.21132486540518713;
const NORMALIZER_2D: f32 = 0.05481866495625118;

pub struct SimplexNoise2 {
    grads: [[f32; 2]; 256],
}

impl SimplexNoise2 {
    pub fn new(seed: u64) -> SimplexNoise2 {
        let mut rng = Pcg32::seed_from_u64(seed);

        let grads = std::array::from_fn(|_| {
            let x = rng.gen_range(-1.0..=1.0);
            let y = rng.gen_range(-1.0..=1.0);
            let s = NORMALIZER_2D * f32::hypot(x, y);
            [x / s, y / s]
        });

        SimplexNoise2 { grads }
    }

    pub fn get(&self, pos: Vec2) -> f32 {
        let s = SKEW_2D * (pos.x + pos.y);
        self.base(pos.x + s, pos.y + s) * 0.5 + 0.5
    }

    fn base(&self, xs: f32, ys: f32) -> f32 {
        // Get base points and offsets.
        let xsb = Wrapping(xs.floor() as i64);
        let ysb = Wrapping(ys.floor() as i64);
        let xi = xs - xsb.0 as f32;
        let yi = ys - ysb.0 as f32;

        // Prime pre-multiplication for hash.
        let xsbp = xsb * PRIME_X;
        let ysbp = ysb * PRIME_Y;

        // Unskew
        let t = (xi + yi) * UNSKEW_2D;
        let dx0 = xi + t;
        let dy0 = yi + t;

        // First vertex.
        let a0 = RSQUARED_2D - dx0 * dx0 - dy0 * dy0;
        let mut value = (a0 * a0) * (a0 * a0) * self.grad(xsbp, ysbp, dx0, dy0);

        // Second vertex.
        let a1 = (2.0 * (1.0 + 2.0 * UNSKEW_2D) * (1.0 / UNSKEW_2D + 2.0)) * t
            + ((-2.0 * (1.0 + 2.0 * UNSKEW_2D) * (1.0 + 2.0 * UNSKEW_2D)) + a0);
        let dx1 = dx0 - (1.0 + 2.0 * UNSKEW_2D);
        let dy1 = dy0 - (1.0 + 2.0 * UNSKEW_2D);
        value += (a1 * a1) * (a1 * a1) * self.grad(xsbp + PRIME_X, ysbp + PRIME_Y, dx1, dy1);

        // Third and fourth vertices.
        let xmyi = xi - yi;
        if t < UNSKEW_2D {
            if xi + xmyi > 1.0 {
                let dx2 = dx0 - (3.0 * UNSKEW_2D + 2.0);
                let dy2 = dy0 - (3.0 * UNSKEW_2D + 1.0);
                let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
                if a2 > 0.0 {
                    value += (a2 * a2)
                        * (a2 * a2)
                        * self.grad(xsbp + (PRIME_X << 1), ysbp + PRIME_Y, dx2, dy2);
                }
            } else {
                let dx2 = dx0 - UNSKEW_2D;
                let dy2 = dy0 - (UNSKEW_2D + 1.0);
                let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
                if a2 > 0.0 {
                    value += (a2 * a2) * (a2 * a2) * self.grad(xsbp, ysbp + PRIME_Y, dx2, dy2);
                }
            }

            if yi - xmyi > 1.0 {
                let dx3 = dx0 - (3.0 * UNSKEW_2D + 1.0);
                let dy3 = dy0 - (3.0 * UNSKEW_2D + 2.0);
                let a3 = RSQUARED_2D - dx3 * dx3 - dy3 * dy3;
                if a3 > 0.0 {
                    value += (a3 * a3)
                        * (a3 * a3)
                        * self.grad(xsbp + PRIME_X, ysbp + (PRIME_Y << 1), dx3, dy3);
                }
            } else {
                let dx3 = dx0 - (UNSKEW_2D + 1.0);
                let dy3 = dy0 - UNSKEW_2D;
                let a3 = RSQUARED_2D - dx3 * dx3 - dy3 * dy3;
                if a3 > 0.0 {
                    value += (a3 * a3) * (a3 * a3) * self.grad(xsbp + PRIME_X, ysbp, dx3, dy3);
                }
            }
        } else {
            if xi + xmyi < 0.0 {
                let dx2 = dx0 + (1.0 + UNSKEW_2D);
                let dy2 = dy0 + UNSKEW_2D;
                let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
                if a2 > 0.0 {
                    value += (a2 * a2) * (a2 * a2) * self.grad(xsbp - PRIME_X, ysbp, dx2, dy2);
                }
            } else {
                let dx2 = dx0 - (UNSKEW_2D + 1.0);
                let dy2 = dy0 - UNSKEW_2D;
                let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
                if a2 > 0.0 {
                    value += (a2 * a2) * (a2 * a2) * self.grad(xsbp + PRIME_X, ysbp, dx2, dy2);
                }
            }

            if yi < xmyi {
                let dx2 = dx0 + UNSKEW_2D;
                let dy2 = dy0 + (UNSKEW_2D + 1.0);
                let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
                if a2 > 0.0 {
                    value += (a2 * a2) * (a2 * a2) * self.grad(xsbp, ysbp - PRIME_Y, dx2, dy2);
                }
            } else {
                let dx2 = dx0 - UNSKEW_2D;
                let dy2 = dy0 - (UNSKEW_2D + 1.0);
                let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
                if a2 > 0.0 {
                    value += (a2 * a2) * (a2 * a2) * self.grad(xsbp, ysbp + PRIME_Y, dx2, dy2);
                }
            }
        }

        value
    }

    fn grad(&self, xsvp: Wrapping<i64>, ysvp: Wrapping<i64>, dx: f32, dy: f32) -> f32 {
        let idx = ((xsvp ^ ysvp) * HASH_PRIME).0 & 0xff;
        let [gx, gy] = self.grads[idx as usize];
        gx * dx + gy * dy
    }
}

use std::f32::consts::TAU;
use std::ops::{AddAssign, DivAssign, Index, IndexMut, MulAssign, SubAssign};

use bevy::core::cast_slice;
use bevy::prelude::{info_span, IVec2, UVec2, Vec2};
use rand::Rng;
use rayon::prelude::*;

use crate::SimplexNoise2;

pub const NEIGHBORHOOD_4: [IVec2; 4] = [
    IVec2::new(0, -1),
    IVec2::new(1, 0),
    IVec2::new(0, 1),
    IVec2::new(-1, 0),
];

pub const NEIGHBORHOOD_8: [IVec2; 8] = [
    IVec2::new(0, -1),
    IVec2::new(1, -1),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, 0),
    IVec2::new(-1, -1),
];

#[derive(Debug, Clone)]
pub struct Grid<T> {
    origin: IVec2,
    size: UVec2,
    data: Box<[T]>,
}

impl<T> Grid<T> {
    pub fn new(size: UVec2, fill: T) -> Grid<T>
    where
        T: Clone,
    {
        assert!(size.x < i32::MAX as u32);
        assert!(size.y < i32::MAX as u32);

        Grid {
            origin: IVec2::ZERO,
            size,
            data: vec![fill; (size.x as usize) * (size.y as usize)].into(),
        }
    }

    pub fn new_default(size: UVec2) -> Grid<T>
    where
        T: Default + Clone,
    {
        Grid::new(size, T::default())
    }

    pub fn from_data(size: UVec2, data: impl Into<Box<[T]>>) -> Grid<T> {
        let data = data.into();
        assert_eq!(data.len(), (size.x as usize) * (size.y as usize));
        Grid {
            origin: IVec2::ZERO,
            size,
            data,
        }
    }

    pub fn from_fn(size: UVec2, f: impl FnMut(IVec2) -> T) -> Grid<T> {
        Grid::from_data(
            size,
            (0..size.y as i32)
                .flat_map(move |y| (0..size.x as i32).map(move |x| IVec2::new(x, y)))
                .map(f)
                .collect::<Vec<T>>(),
        )
    }

    pub fn par_from_fn(size: UVec2, f: impl (Fn(IVec2) -> T) + Send + Sync) -> Grid<T>
    where
        T: Send,
    {
        Grid::from_data(
            size,
            (0..(size.x as usize) * (size.y as usize))
                .into_par_iter()
                .map(move |idx| {
                    let x = (idx % (size.x as usize)) as i32;
                    let y = (idx / (size.x as usize)) as i32;
                    IVec2::new(x, y)
                })
                .map(f)
                .collect::<Vec<T>>(),
        )
    }

    pub fn with_origin(mut self, origin: IVec2) -> Grid<T> {
        self.origin = origin;
        self
    }

    pub fn origin(&self) -> IVec2 {
        self.origin
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [T] {
        &mut self.data
    }

    pub fn index(&self, mut cell: IVec2) -> usize {
        cell -= self.origin;
        (cell.y as usize) * (self.size.x as usize) + (cell.x as usize)
    }

    pub fn contains_cell(&self, mut cell: IVec2) -> bool {
        cell -= self.origin;
        (cell.x >= 0 && (cell.x as u32) < self.size.x)
            && (cell.y >= 0 && (cell.y as u32) < self.size.y)
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> {
        let size = self.size;
        let origin = self.origin;
        (0..size.y as i32)
            .flat_map(move |y| (0..size.x as i32).map(move |x| origin + IVec2::new(x, y)))
    }

    pub fn par_cells(&self) -> impl IndexedParallelIterator<Item = IVec2> {
        let size = self.size;
        let origin = self.origin;
        (0..(size.x as usize) * (size.y as usize))
            .into_par_iter()
            .map(move |idx| {
                let x = (idx % (size.x as usize)) as i32;
                let y = (idx / (size.x as usize)) as i32;
                origin + IVec2::new(x, y)
            })
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    pub fn par_values(&self) -> impl IndexedParallelIterator<Item = &T>
    where
        T: Sync,
    {
        self.data.par_iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut()
    }

    pub fn par_values_mut(&mut self) -> impl IndexedParallelIterator<Item = &mut T>
    where
        T: Send,
    {
        self.data.par_iter_mut()
    }

    pub fn entries(&self) -> impl Iterator<Item = (IVec2, &T)> {
        self.cells().zip(self.values())
    }

    pub fn par_entries(&self) -> impl IndexedParallelIterator<Item = (IVec2, &T)>
    where
        T: Sync + 'static,
    {
        self.par_cells().zip(self.par_values())
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = (IVec2, &mut T)> {
        self.cells().zip(self.values_mut())
    }

    pub fn par_entries_mut(&mut self) -> impl IndexedParallelIterator<Item = (IVec2, &mut T)>
    where
        T: Send + 'static,
    {
        self.par_cells().zip(self.par_values_mut())
    }

    fn neighborhood<const N: usize>(
        &self,
        neighbors: [IVec2; N],
        center: IVec2,
    ) -> impl Iterator<Item = (usize, IVec2)> {
        neighbors
            .map(|dir| self.contains_cell(center + dir).then_some(center + dir))
            .into_iter()
            .enumerate()
            .flat_map(|(i, pt)| pt.map(|pt| (i, pt)))
    }

    pub fn neighborhood_4(&self, center: IVec2) -> impl Iterator<Item = (usize, IVec2)> {
        self.neighborhood(NEIGHBORHOOD_4, center)
    }

    pub fn neighborhood_8(&self, center: IVec2) -> impl Iterator<Item = (usize, IVec2)> {
        self.neighborhood(NEIGHBORHOOD_8, center)
    }

    pub fn get(&self, cell: IVec2) -> Option<&T> {
        if self.contains_cell(cell) {
            let index = self.index(cell);
            self.data.get(index)
        } else {
            None
        }
    }

    pub fn clamped_get(&self, mut cell: IVec2) -> &T {
        cell = cell
            .max(self.origin)
            .min(self.origin + self.size.as_ivec2() - 1);
        let index = self.index(cell);
        &self.data[index]
    }

    pub fn get_mut(&mut self, cell: IVec2) -> Option<&mut T> {
        if self.contains_cell(cell) {
            let index = self.index(cell);
            self.data.get_mut(index)
        } else {
            None
        }
    }

    pub fn set(&mut self, cell: IVec2, value: T) -> Option<T> {
        Some(std::mem::replace(self.get_mut(cell)?, value))
    }
}

impl<T> Index<IVec2> for Grid<T> {
    type Output = T;

    fn index(&self, cell: IVec2) -> &Self::Output {
        self.get(cell).unwrap_or_else(|| panic_oob(cell, self.size))
    }
}

impl<T> IndexMut<IVec2> for Grid<T> {
    fn index_mut(&mut self, cell: IVec2) -> &mut Self::Output {
        let size = self.size;
        self.get_mut(cell).unwrap_or_else(|| panic_oob(cell, size))
    }
}

impl<'a, T: AddAssign<&'a T> + 'a> AddAssign<&'a Grid<T>> for Grid<T> {
    fn add_assign(&mut self, rhs: &'a Grid<T>) {
        assert_eq!(self.size, rhs.size);
        assert_eq!(self.origin, rhs.origin);
        for (lhs, rhs) in self.data.iter_mut().zip(rhs.data.iter()) {
            *lhs += rhs;
        }
    }
}

impl<T: for<'a> AddAssign<&'a T>> AddAssign<Grid<T>> for Grid<T> {
    fn add_assign(&mut self, rhs: Grid<T>) {
        *self += &rhs;
    }
}

impl<'a, T: SubAssign<&'a T> + 'a> SubAssign<&'a Grid<T>> for Grid<T> {
    fn sub_assign(&mut self, rhs: &'a Grid<T>) {
        assert_eq!(self.size, rhs.size);
        assert_eq!(self.origin, rhs.origin);
        for (lhs, rhs) in self.data.iter_mut().zip(rhs.data.iter()) {
            *lhs -= rhs;
        }
    }
}

impl<T: for<'a> SubAssign<&'a T>> SubAssign<Grid<T>> for Grid<T> {
    fn sub_assign(&mut self, rhs: Grid<T>) {
        *self -= &rhs;
    }
}

impl<'a, T: MulAssign<&'a T> + 'a> MulAssign<&'a Grid<T>> for Grid<T> {
    fn mul_assign(&mut self, rhs: &'a Grid<T>) {
        assert_eq!(self.size, rhs.size);
        assert_eq!(self.origin, rhs.origin);
        for (lhs, rhs) in self.data.iter_mut().zip(rhs.data.iter()) {
            *lhs *= rhs;
        }
    }
}

impl<T: for<'a> MulAssign<&'a T>> MulAssign<Grid<T>> for Grid<T> {
    fn mul_assign(&mut self, rhs: Grid<T>) {
        *self *= &rhs;
    }
}

impl<'a, T: DivAssign<&'a T> + 'a> DivAssign<&'a Grid<T>> for Grid<T> {
    fn div_assign(&mut self, rhs: &'a Grid<T>) {
        assert_eq!(self.size, rhs.size);
        assert_eq!(self.origin, rhs.origin);
        for (lhs, rhs) in self.data.iter_mut().zip(rhs.data.iter()) {
            *lhs /= rhs;
        }
    }
}

impl<T: for<'a> DivAssign<&'a T>> DivAssign<Grid<T>> for Grid<T> {
    fn div_assign(&mut self, rhs: Grid<T>) {
        *self /= &rhs;
    }
}

impl Grid<f32> {
    pub fn add_noise(&mut self, noise: &SimplexNoise2, rotation: f32, scale: f32, amplitude: f32) {
        for (cell, value) in self.entries_mut() {
            let pos = Vec2::from_angle(rotation).rotate(cell.as_vec2());
            *value += noise.get(pos * scale) * amplitude;
        }
    }

    pub fn add_fbm_noise<R: Rng>(
        &mut self,
        rng: &mut R,
        mut scale: f32,
        mut amplitude: f32,
        octaves: usize,
    ) {
        let _scope = info_span!("add_fbm_noise").entered();

        let mut total_amplitude = 0.0;

        for _ in 0..octaves {
            let noise_seed = rng.gen::<u64>();
            let noise = SimplexNoise2::new(noise_seed);
            let angle = rng.gen_range(0.0..TAU);
            self.add_noise(&noise, angle, scale, amplitude);
            total_amplitude += amplitude;
            scale *= 2.0;
            amplitude /= 2.0;
        }

        for (_, value) in self.entries_mut() {
            *value /= total_amplitude;
        }
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

    pub fn blur(&mut self, kernel_size: i32) {
        let _scope = info_span!("blur").entered();

        let mut temp = self.clone();
        temp.par_entries_mut().for_each(|(cell, value)| {
            let mut sum = 0.0;
            for sx in -kernel_size..=kernel_size {
                sum += self.get(cell + IVec2::new(sx, 0)).copied().unwrap_or(0.0);
            }

            *value = sum / (2 * kernel_size + 1) as f32;
        });

        self.par_entries_mut().for_each(|(cell, value)| {
            let mut sum = 0.0;
            for sy in -kernel_size..=kernel_size {
                sum += temp.get(cell + IVec2::new(0, sy)).copied().unwrap_or(0.0);
            }

            *value = sum / (2 * kernel_size + 1) as f32;
        });
    }

    pub fn min_value(&self) -> f32 {
        self.data.iter().copied().fold(f32::INFINITY, f32::min)
    }

    pub fn max_value(&self) -> f32 {
        self.data.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    }

    pub fn to_bool(&self, cutoff: f32) -> Grid<bool> {
        let mut res = Grid::new(self.size, false).with_origin(self.origin);

        for (cell, &value) in self.entries() {
            if value > cutoff {
                res[cell] = true;
            }
        }

        res
    }

    pub fn debug_save(&self, path: &str) {
        let min = self.min_value();
        let max = self.max_value();
        let data = self.data.iter();
        let scaled_data = data
            .map(|v| ((v - min) / (max - min) * 65535.0) as u16)
            .collect::<Vec<_>>();

        image::save_buffer(
            path,
            cast_slice(&scaled_data),
            self.size.x,
            self.size.y,
            image::ColorType::L16,
        )
        .unwrap();
    }
}

impl Grid<bool> {
    pub fn to_f32(&self) -> Grid<f32> {
        let mut res = Grid::new(self.size, 0.0).with_origin(self.origin);

        for (cell, &value) in self.entries() {
            res[cell] = if value { 1.0 } else { 0.0 };
        }

        res
    }

    pub fn compute_edt(&self) -> Grid<f32> {
        let _scope = info_span!("compute_edt").entered();

        let data = edt::edt(
            self.data(),
            (self.size().x as usize, self.size().y as usize),
            false,
        );

        let max = data.iter().copied().fold(0.0, f64::max);

        let data = data
            .into_iter()
            .map(|v| (v / max) as f32)
            .collect::<Vec<_>>();

        Grid::from_data(self.size(), data).with_origin(self.origin)
    }

    pub fn debug_save(&self, path: &str) {
        let data = self.data.iter();
        let scaled_data = data.map(|&v| if v { 255 } else { 0 }).collect::<Vec<u8>>();

        image::save_buffer(
            path,
            &scaled_data,
            self.size.x,
            self.size.y,
            image::ColorType::L8,
        )
        .unwrap();
    }
}

#[inline(never)]
fn panic_oob(cell: IVec2, size: UVec2) -> ! {
    panic!("{} is outside grid of size {}", cell, size)
}

// TODO: move this somewhere else
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

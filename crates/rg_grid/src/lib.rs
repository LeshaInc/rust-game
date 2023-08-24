mod blur;
mod edt;
mod float_grid;
mod ops;
mod serde_blob;

use std::path::Path;

use bevy::math::Vec2Swizzles;
use bevy::prelude::*;
use bytemuck::{cast_slice, CheckedBitPattern, NoUninit};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub use crate::edt::EdtSettings;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "T: NoUninit", deserialize = "T: CheckedBitPattern"))]
pub struct Grid<T> {
    origin: IVec2,
    size: UVec2,
    #[serde(with = "serde_blob")]
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

    pub fn from_data(size: UVec2, data: impl Into<Box<[T]>>) -> Grid<T> {
        let data = data.into();
        assert_eq!(data.len(), (size.x as usize) * (size.y as usize));
        Grid {
            origin: IVec2::ZERO,
            size,
            data,
        }
    }

    pub fn from_fn_with_origin(size: UVec2, origin: IVec2, f: impl FnMut(IVec2) -> T) -> Grid<T> {
        let data = (0..size.y as i32)
            .flat_map(move |y| (0..size.x as i32).map(move |x| origin + IVec2::new(x, y)))
            .map(f)
            .collect::<Vec<T>>();
        Grid::from_data(size, data).with_origin(origin)
    }

    pub fn from_fn(size: UVec2, f: impl FnMut(IVec2) -> T) -> Grid<T> {
        Grid::from_fn_with_origin(size, IVec2::ZERO, f)
    }

    pub fn par_from_fn_with_origin(
        size: UVec2,
        origin: IVec2,
        f: impl (Fn(IVec2) -> T) + Send + Sync,
    ) -> Grid<T>
    where
        T: Send,
    {
        let data = (0..(size.x as usize) * (size.y as usize))
            .into_par_iter()
            .map(move |idx| {
                let x = (idx % (size.x as usize)) as i32;
                let y = (idx / (size.x as usize)) as i32;
                IVec2::new(x, y)
            })
            .map(f)
            .collect::<Vec<T>>();
        Grid::from_data(size, data).with_origin(origin)
    }

    pub fn par_from_fn(size: UVec2, f: impl (Fn(IVec2) -> T) + Send + Sync) -> Grid<T>
    where
        T: Send,
    {
        Grid::par_from_fn_with_origin(size, IVec2::ZERO, f)
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

    pub fn get(&self, cell: IVec2) -> Option<&T> {
        if self.contains_cell(cell) {
            let index = self.index(cell);
            self.data.get(index)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, cell: IVec2) -> Option<&mut T> {
        if self.contains_cell(cell) {
            let index = self.index(cell);
            self.data.get_mut(index)
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

    pub fn set(&mut self, cell: IVec2, value: T) -> Option<T> {
        Some(std::mem::replace(self.get_mut(cell)?, value))
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

    pub fn map<U>(&self, mut f: impl FnMut(IVec2, &T) -> U) -> Grid<U> {
        let data = self
            .entries()
            .map(|(cell, value)| f(cell, value))
            .collect::<Vec<_>>();
        Grid::from_data(self.size, data).with_origin(self.origin)
    }

    pub fn par_map<U>(&self, f: impl Fn(IVec2, &T) -> U + Send + Sync) -> Grid<U>
    where
        T: Sync + 'static,
        U: Send,
    {
        let data = self
            .par_entries()
            .map(|(cell, value)| f(cell, value))
            .collect::<Vec<_>>();
        Grid::from_data(self.size, data).with_origin(self.origin)
    }

    pub fn map_inplace(&mut self, mut f: impl FnMut(IVec2, &mut T)) {
        self.entries_mut().for_each(|(cell, value)| f(cell, value))
    }

    pub fn par_map_inplace(&mut self, f: impl Fn(IVec2, &mut T) + Send + Sync)
    where
        T: Send + 'static,
    {
        self.par_entries_mut()
            .for_each(|(cell, value)| f(cell, value))
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

    pub fn rows(&self) -> impl ExactSizeIterator<Item = &[T]> {
        self.data.chunks_exact(self.size.x as usize)
    }

    pub fn par_rows(&self) -> impl IndexedParallelIterator<Item = &[T]>
    where
        T: Sync + 'static,
    {
        self.data.par_chunks_exact(self.size.x as usize)
    }

    pub fn rows_mut(&mut self) -> impl ExactSizeIterator<Item = &mut [T]> {
        self.data.chunks_exact_mut(self.size.x as usize)
    }

    pub fn par_rows_mut(&mut self) -> impl IndexedParallelIterator<Item = &mut [T]>
    where
        T: Send + 'static,
    {
        self.data.par_chunks_exact_mut(self.size.x as usize)
    }

    pub fn transpose(&self) -> Grid<T>
    where
        T: Send + Sync + Copy + 'static,
    {
        Grid::par_from_fn_with_origin(self.size.yx(), self.origin.yx(), |cell| self[cell.yx()])
    }

    pub fn transpose_in_place(&mut self)
    where
        T: Send + Sync + Copy + 'static,
    {
        *self = self.transpose();
    }
}

impl Grid<[u8; 3]> {
    pub fn debug_save(&self, path: impl AsRef<Path>) {
        if !cfg!(debug_assertions) {
            return;
        }

        let _scope = info_span!("debug_save").entered();

        image::save_buffer(
            path,
            cast_slice(&self.data),
            self.size.x,
            self.size.y,
            image::ColorType::Rgb8,
        )
        .unwrap();
    }
}

impl Grid<bool> {
    pub fn to_f32(&self) -> Grid<f32> {
        self.map(|_, &v| if v { 1.0 } else { 0.0 })
    }

    pub fn debug_save(&self, path: impl AsRef<Path>) {
        self.to_f32().debug_save(path);
    }
}

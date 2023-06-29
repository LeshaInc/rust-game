use std::sync::Arc;

use bevy::prelude::IVec2;

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
    size: IVec2,
    data: Box<[T]>,
}

impl<T> Grid<T> {
    pub fn new(size: IVec2, fill: T) -> Grid<T>
    where
        T: Clone,
    {
        Grid {
            size,
            data: vec![fill; (size.x as usize) * (size.y as usize)].into(),
        }
    }

    pub fn new_default(size: IVec2) -> Grid<T>
    where
        T: Default + Clone,
    {
        Grid::new(size, T::default())
    }

    pub fn size(&self) -> IVec2 {
        self.size
    }

    fn index(&self, cell: IVec2) -> usize {
        (cell.y as usize) * (self.size.x as usize) + (cell.x as usize)
    }

    pub fn contains_cell(&self, cell: IVec2) -> bool {
        (cell.x >= 0 && cell.x < self.size.x) && (cell.y >= 0 && cell.y < self.size.y)
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> {
        let size = self.size;
        (0..size.y).flat_map(move |y| (0..size.x).map(move |x| IVec2::new(x, y)))
    }

    pub fn entries(&self) -> impl Iterator<Item = (IVec2, &T)> {
        self.cells().zip(self.data.iter())
    }

    pub fn entries_mut(&mut self) -> impl Iterator<Item = (IVec2, &mut T)> {
        self.cells().zip(self.data.iter_mut())
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

impl<T> std::ops::Index<IVec2> for Grid<T> {
    type Output = T;

    fn index(&self, cell: IVec2) -> &Self::Output {
        self.get(cell).unwrap_or_else(|| panic_oob(cell, self.size))
    }
}

impl<T> std::ops::IndexMut<IVec2> for Grid<T> {
    fn index_mut(&mut self, cell: IVec2) -> &mut Self::Output {
        let size = self.size;
        self.get_mut(cell).unwrap_or_else(|| panic_oob(cell, size))
    }
}

#[inline(never)]
fn panic_oob(cell: IVec2, size: IVec2) -> ! {
    panic!("{} is outside grid of size {}", cell, size)
}

#[derive(Debug, Clone)]
pub struct SharedGrid<T>(pub Arc<Grid<T>>);

impl<T> SharedGrid<T> {
    pub fn new(size: IVec2, fill: T) -> SharedGrid<T>
    where
        T: Clone,
    {
        SharedGrid(Arc::new(Grid::new(size, fill)))
    }

    pub fn new_default(size: IVec2) -> SharedGrid<T>
    where
        T: Default + Clone,
    {
        SharedGrid(Arc::new(Grid::new_default(size)))
    }

    pub fn size(&self) -> IVec2 {
        self.0.size()
    }

    pub fn contains_cell(&self, cell: IVec2) -> bool {
        self.0.contains_cell(cell)
    }

    pub fn cells(&self) -> impl Iterator<Item = IVec2> {
        self.0.cells()
    }

    pub fn entries(&self) -> impl Iterator<Item = (IVec2, &T)> {
        self.0.entries()
    }

    pub fn neighborhood_4(&self, center: IVec2) -> impl Iterator<Item = (usize, IVec2)> {
        self.0.neighborhood_4(center)
    }

    pub fn neighborhood_8(&self, center: IVec2) -> impl Iterator<Item = (usize, IVec2)> {
        self.0.neighborhood_8(center)
    }

    pub fn get(&self, cell: IVec2) -> Option<&T> {
        self.0.get(cell)
    }

    pub fn make_mut(&mut self) -> &mut Grid<T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0)
    }
}

impl<T> From<Grid<T>> for SharedGrid<T> {
    fn from(value: Grid<T>) -> Self {
        SharedGrid(Arc::new(value))
    }
}

impl<T> std::ops::Index<IVec2> for SharedGrid<T> {
    type Output = T;

    fn index(&self, cell: IVec2) -> &Self::Output {
        &self.0[cell]
    }
}

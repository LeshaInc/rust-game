use std::ops::Deref;

use bevy::prelude::IVec2;
use bevy::utils::HashMap;
use parking_lot::{RwLock, RwLockReadGuard};

use crate::Grid;

pub struct GridCache<T> {
    chunk_size: IVec2,
    chunks: RwLock<HashMap<IVec2, Grid<T>>>,
    builder: Box<dyn Fn(&mut Grid<T>)>,
}

impl<T: Default + Clone> GridCache<T> {
    pub fn new(chunk_size: IVec2, builder: impl Fn(&mut Grid<T>) + 'static) -> GridCache<T> {
        GridCache {
            chunk_size,
            chunks: RwLock::new(HashMap::default()),
            builder: Box::new(builder),
        }
    }

    pub fn get_chunk(&self, chunk_pos: IVec2) -> impl Deref<Target = Grid<T>> + '_ {
        let map = self.chunks.read();
        if let Ok(v) = RwLockReadGuard::try_map(map, |map| map.get(&chunk_pos)) {
            return v;
        }

        let mut map = self.chunks.write();
        let mut grid = Grid::new_default(self.chunk_size).with_origin(chunk_pos * self.chunk_size);
        (self.builder)(&mut grid);
        map.insert(chunk_pos, grid);

        self.get_chunk(chunk_pos)
    }
}

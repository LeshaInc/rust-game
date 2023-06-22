use std::sync::Arc;

use bevy::prelude::UVec2;

use crate::CHUNK_RESOLUTION;

const DATA_LEN: usize = (CHUNK_RESOLUTION as usize) * (CHUNK_RESOLUTION as usize);

#[derive(Debug, Clone)]
pub struct ChunkMap<T> {
    data: Arc<[T; DATA_LEN]>,
}

impl<T: Copy> ChunkMap<T> {
    pub fn new(fill: T) -> ChunkMap<T> {
        ChunkMap {
            data: Arc::new([fill; DATA_LEN]),
        }
    }

    pub fn get(&self, pos: UVec2) -> T {
        self.data[index(pos)]
    }

    pub fn make_mut(&mut self) -> ChunkMapRefMut<'_, T> {
        ChunkMapRefMut {
            data: Arc::make_mut(&mut self.data),
        }
    }
}

impl<T: Copy + Default> Default for ChunkMap<T> {
    fn default() -> ChunkMap<T> {
        ChunkMap::new(T::default())
    }
}

#[derive(Debug)]
pub struct ChunkMapRefMut<'a, T> {
    data: &'a mut [T; DATA_LEN],
}

impl<T: Copy> ChunkMapRefMut<'_, T> {
    pub fn get(&self, pos: UVec2) -> T {
        self.data[index(pos)]
    }

    pub fn get_mut(&mut self, pos: UVec2) -> &mut T {
        &mut self.data[index(pos)]
    }

    pub fn set(&mut self, pos: UVec2, value: T) {
        self.data[index(pos)] = value;
    }
}

fn index(pos: UVec2) -> usize {
    (pos.x as usize) * (CHUNK_RESOLUTION as usize) + (pos.y as usize)
}

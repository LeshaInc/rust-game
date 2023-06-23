use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Debug, Default, Resource)]
pub struct Chunks {
    map: HashMap<IVec2, Entity>,
}

impl Chunks {
    pub fn insert(&mut self, pos: IVec2, id: Entity) {
        self.map.insert(pos, id);
    }

    pub fn get(&self, pos: IVec2) -> Option<Entity> {
        self.map.get(&pos).copied()
    }

    pub fn get_neighbors(&self, pos: IVec2) -> [Option<(IVec2, Entity)>; 8] {
        NEIGHBOR_DIRS.map(|dir| {
            let n_pos = pos + dir;
            self.get(n_pos).map(|id| (n_pos, id))
        })
    }

    pub fn remove(&mut self, pos: IVec2) {
        self.map.remove(&pos);
    }
}

pub const NEIGHBOR_DIRS: [IVec2; 8] = [
    IVec2::new(-1, -1),
    IVec2::new(-1, 0),
    IVec2::new(-1, 1),
    IVec2::new(0, 1),
    IVec2::new(1, 1),
    IVec2::new(1, 0),
    IVec2::new(1, -1),
    IVec2::new(0, -1),
];

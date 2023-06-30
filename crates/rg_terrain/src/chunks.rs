use bevy::prelude::*;
use bevy::utils::HashMap;
use rg_core::NEIGHBORHOOD_8;

#[derive(Debug, Default, Resource)]
pub struct Chunks {
    map: HashMap<IVec2, Entity>,
}

impl Chunks {
    pub fn insert(&mut self, pos: IVec2, id: Entity) {
        self.map.insert(pos, id);
    }

    pub fn contains(&self, pos: IVec2) -> bool {
        self.map.contains_key(&pos)
    }

    pub fn get(&self, pos: IVec2) -> Option<Entity> {
        self.map.get(&pos).copied()
    }

    pub fn get_neighbors(&self, pos: IVec2) -> [Option<Entity>; 8] {
        NEIGHBORHOOD_8.map(|dir| self.get(pos + dir))
    }

    pub fn remove(&mut self, pos: IVec2) {
        self.map.remove(&pos);
    }

    pub fn retain(&mut self, mut f: impl FnMut(IVec2, Entity) -> bool) {
        self.map.retain(|k, v| f(*k, *v))
    }
}

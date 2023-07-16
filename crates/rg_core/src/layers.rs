use bevy_xpbd_3d::prelude::PhysicsLayer;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PhysicsLayer)]
pub enum CollisionLayer {
    Static,
    Dynamic,
    Character,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CollisionLayers: u32 {
        const STATIC_GEOMETRY = 1;
        const DYNAMIC_GEOMETRY = 1 << 1;
        const NAVMESH = 1 << 2;
        const CHARACTER = 1 << 3;
        const AGENTS = 1 << 4;
    }
}

impl From<CollisionLayers> for bevy_rapier3d::geometry::Group {
    fn from(value: CollisionLayers) -> Self {
        Self::from_bits_truncate(value.bits())
    }
}

impl From<CollisionLayers> for bevy_rapier3d::rapier::geometry::Group {
    fn from(value: CollisionLayers) -> Self {
        Self::from_bits_truncate(value.bits())
    }
}

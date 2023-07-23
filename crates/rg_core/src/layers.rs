use bevy_rapier3d::prelude::CollisionGroups;

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct CollisionLayers: u32 {
        const STATIC = 1;
        const DYNAMIC = 1 << 1;
        const WALKABLE = 1 << 2;
        const AGENTS = Self::DYNAMIC.bits() | 1 << 3;
        const CHARACTER = Self::AGENTS.bits() | 1 << 4;

        const STATIC_AND_DYNAMIC = Self::STATIC.bits() | Self::DYNAMIC.bits();
        const STATIC_WALKABLE = Self::STATIC.bits() | Self::WALKABLE.bits();
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

impl CollisionLayers {
    pub const fn with_mask(self, mask: CollisionLayers) -> CollisionGroups {
        CollisionGroups::new(
            bevy_rapier3d::geometry::Group::from_bits_truncate(self.bits()),
            bevy_rapier3d::geometry::Group::from_bits_truncate(mask.bits()),
        )
    }

    pub const STATIC_GROUP: CollisionGroups =
        CollisionLayers::STATIC.with_mask(CollisionLayers::DYNAMIC);

    pub const STATIC_WALKABLE_GROUP: CollisionGroups =
        CollisionLayers::STATIC_WALKABLE.with_mask(CollisionLayers::DYNAMIC);

    pub const DYNAMIC_GROUP: CollisionGroups =
        CollisionLayers::DYNAMIC.with_mask(CollisionLayers::STATIC_AND_DYNAMIC);

    pub const CHARACTER_GROUP: CollisionGroups =
        CollisionLayers::CHARACTER.with_mask(CollisionLayers::STATIC);
}

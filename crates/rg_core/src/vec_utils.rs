use bevy::prelude::*;

pub trait VecToBits {
    type Output;

    fn to_bits(self) -> Self::Output;
}

impl VecToBits for Vec2 {
    type Output = UVec2;

    #[inline]
    fn to_bits(self) -> UVec2 {
        UVec2::new(self.x.to_bits(), self.y.to_bits())
    }
}

impl VecToBits for Vec3 {
    type Output = UVec3;

    #[inline]
    fn to_bits(self) -> UVec3 {
        UVec3::new(self.x.to_bits(), self.y.to_bits(), self.z.to_bits())
    }
}

impl VecToBits for Vec4 {
    type Output = UVec4;

    #[inline]
    fn to_bits(self) -> UVec4 {
        UVec4::new(
            self.x.to_bits(),
            self.y.to_bits(),
            self.z.to_bits(),
            self.w.to_bits(),
        )
    }
}

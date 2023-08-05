use std::ops::*;

use bevy::prelude::*;

use super::Grid;

type Type = IVec2;

impl<T> Index<Type> for Grid<T> {
    type Output = T;

    fn index(&self, cell: IVec2) -> &Self::Output {
        self.get(cell).unwrap_or_else(|| panic_oob(cell, self.size))
    }
}

impl<T> IndexMut<IVec2> for Grid<T> {
    fn index_mut(&mut self, cell: IVec2) -> &mut Self::Output {
        let size = self.size;
        self.get_mut(cell).unwrap_or_else(|| panic_oob(cell, size))
    }
}

#[inline(never)]
fn panic_oob(cell: IVec2, size: UVec2) -> ! {
    panic!("{} is outside grid of size {}", cell, size)
}

macro_rules! impl_op {
    ($trait:ident, $method:ident) => {
        impl<T: Copy + $trait<T>> $trait<&Grid<T>> for &Grid<T> {
            type Output = Grid<T::Output>;

            fn $method(self, rhs: &Grid<T>) -> Self::Output {
                assert_eq!(self.size, rhs.size);
                assert_eq!(self.origin, rhs.origin);
                let lhs_it = self.values().copied();
                let rhs_it = self.values().copied();
                let data = lhs_it
                    .zip(rhs_it)
                    .map(|(a, b)| a.$method(b))
                    .collect::<Vec<_>>();
                Grid::from_data(self.size, data).with_origin(self.origin)
            }
        }

        impl<T: Copy + $trait<T>> $trait<Grid<T>> for Grid<T> {
            type Output = Grid<T::Output>;

            fn $method(self, rhs: Grid<T>) -> Self::Output {
                $trait::$method(&self, &rhs)
            }
        }

        impl<T: Copy + $trait<T>> $trait<T> for &Grid<T> {
            type Output = Grid<T::Output>;

            fn $method(self, rhs: T) -> Self::Output {
                let lhs_it = self.values().copied();
                let data = lhs_it.map(|lhs| lhs.$method(rhs)).collect::<Vec<_>>();
                Grid::from_data(self.size, data).with_origin(self.origin)
            }
        }

        impl<T: Copy + $trait<T>> $trait<T> for Grid<T> {
            type Output = Grid<T::Output>;

            fn $method(self, rhs: T) -> Self::Output {
                $trait::$method(&self, rhs)
            }
        }
    };
}

impl_op!(Add, add);
impl_op!(Sub, sub);
impl_op!(Mul, mul);
impl_op!(Div, div);
impl_op!(Rem, rem);
impl_op!(BitAnd, bitand);
impl_op!(BitOr, bitor);
impl_op!(BitXor, bitxor);
impl_op!(Shl, shl);
impl_op!(Shr, shr);

macro_rules! impl_assign_op {
    ($trait:ident, $method:ident) => {
        impl<T: Copy + $trait<T>> $trait<&Grid<T>> for Grid<T> {
            fn $method(&mut self, rhs: &Grid<T>) {
                assert_eq!(self.size, rhs.size);
                assert_eq!(self.origin, rhs.origin);
                for (lhs, rhs) in self.values_mut().zip(rhs.values()) {
                    lhs.$method(*rhs);
                }
            }
        }

        impl<T: Copy + $trait<T>> $trait<Grid<T>> for Grid<T> {
            fn $method(&mut self, rhs: Grid<T>) {
                $trait::$method(self, &rhs);
            }
        }

        impl<T: Copy + $trait<T>> $trait<T> for Grid<T> {
            fn $method(&mut self, rhs: T) {
                for lhs in self.values_mut() {
                    lhs.$method(rhs);
                }
            }
        }
    };
}

impl_assign_op!(AddAssign, add_assign);
impl_assign_op!(SubAssign, sub_assign);
impl_assign_op!(MulAssign, mul_assign);
impl_assign_op!(DivAssign, div_assign);
impl_assign_op!(RemAssign, rem_assign);
impl_assign_op!(BitAndAssign, bitand_assign);
impl_assign_op!(BitOrAssign, bitor_assign);
impl_assign_op!(BitXorAssign, bitxor_assign);
impl_assign_op!(ShlAssign, shl_assign);
impl_assign_op!(ShrAssign, shr_assign);

macro_rules! impl_unary_op {
    ($trait:ident, $method:ident) => {
        impl<T: Copy + $trait> $trait for &Grid<T> {
            type Output = Grid<T::Output>;

            fn $method(self) -> Self::Output {
                self.map(|_, v| v.$method())
            }
        }

        impl<T: Copy + $trait> $trait for Grid<T> {
            type Output = Grid<T::Output>;

            fn $method(self) -> Self::Output {
                self.map(|_, v| v.$method())
            }
        }
    };
}

impl_unary_op!(Neg, neg);
impl_unary_op!(Not, not);

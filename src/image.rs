//! Simple image structs and operations.
//!
//! This doesn't use the `image` crate, however converting an `Image` from
//! this module to an `ImageBuffer` should be easy.

use std::ops::{Index, IndexMut};
use Vec2;
use Extent2;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Image<T> {
    pub pixels: Vec<T>,
    pub size: Extent2<u32>,
}

macro_rules! image_impl_index {
    ($($idx:ident)+) => { $(
        impl<T> Index<Vec2<$idx>> for Image<T> {
            type Output = T;
            fn index(&self, v: Vec2<$idx>) -> &Self::Output {
                let Vec2 { x, y } = v;
                let x = x as u32;
                let y = y as u32;
                assert!(x < self.size.w);
                assert!(y < self.size.h);
                let i = self.size.w * y + x;
                &self.pixels[i as usize]
            }
        }
        impl<T> IndexMut<Vec2<$idx>> for Image<T> {
            fn index_mut(&mut self, v: Vec2<$idx>) -> &mut Self::Output {
                let Vec2 { x, y } = v;
                let x = x as u32;
                let y = y as u32;
                assert!(x < self.size.w);
                assert!(y < self.size.h);
                let i = self.size.w * y + x;
                &mut self.pixels[i as usize]
            }
        }
        impl<T> Index<($idx, $idx)> for Image<T> {
            type Output = T;
            fn index(&self, v: ($idx, $idx)) -> &Self::Output {
                self.index(Vec2::new(v.0, v.1))
            }
        }
        impl<T> IndexMut<($idx, $idx)> for Image<T> {
            fn index_mut(&mut self, v: ($idx, $idx)) -> &mut Self::Output {
                self.index_mut(Vec2::new(v.0, v.1))
            }
        }
    )+ };
}

image_impl_index!{u8 u16 u32 usize}

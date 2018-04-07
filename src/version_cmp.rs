//! Internal module for performing version comparisons.
#![allow(dead_code)]

pub fn lt<T: PartialOrd>(a: (T, T), b: (T, T)) -> bool { a.0 < b.0 || (a.0 == b.0 && a.1 < b.1) }
pub fn le<T: PartialOrd>(a: (T, T), b: (T, T)) -> bool { a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1) }
pub fn gt<T: PartialOrd>(a: (T, T), b: (T, T)) -> bool { a.0 > b.0 || (a.0 == b.0 && a.1 > b.1) }
pub fn ge<T: PartialOrd>(a: (T, T), b: (T, T)) -> bool { a.0 > b.0 || (a.0 == b.0 && a.1 >= b.1) }


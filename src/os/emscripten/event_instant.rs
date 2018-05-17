use std::cmp::Ordering;
use std::time::Duration;
use std::ops::{Add, Sub, AddAssign, SubAssign};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct OsEventInstant;

impl PartialOrd for OsEventInstant {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        unimplemented!()
    }
}

impl OsEventInstant {
    pub fn duration_since(&self, earlier: Self) -> Option<Duration> {
        assert!(self >= &earlier); // Normally already checked by EventInstant::duration_since
        unimplemented!()
    }
}
impl Add<Duration> for OsEventInstant {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self {
        unimplemented!()
    }
}
impl Sub<Duration> for OsEventInstant {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self {
        unimplemented!()
    }
}
impl AddAssign<Duration> for OsEventInstant {
    fn add_assign(&mut self, rhs: Duration) {
        unimplemented!()
    }
}
impl SubAssign<Duration> for OsEventInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        unimplemented!()
    }
}
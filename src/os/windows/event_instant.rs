use std::cmp::Ordering;
use std::time::{Duration, Instant};
use std::ops::{Add, Sub, AddAssign, SubAssign};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OsEventInstant {
    Wndproc(Instant),
}

impl PartialOrd for OsEventInstant {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (OsEventInstant::Wndproc(ref a), OsEventInstant::Wndproc(ref b)) => a.partial_cmp(b),
        }
    }
}

impl OsEventInstant {
    pub fn duration_since(&self, earlier: Self) -> Option<Duration> {
        assert!(self >= &earlier); // Normally already checked by EventInstant::duration_since
        match (*self, earlier) {
            (OsEventInstant::Wndproc(a), OsEventInstant::Wndproc(b)) => Some(a.duration_since(b)),
        }
    }
}
impl Add<Duration> for OsEventInstant {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self {
        match self {
            OsEventInstant::Wndproc(a) => OsEventInstant::Wndproc(a + rhs),
        }
    }
}
impl Sub<Duration> for OsEventInstant {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self {
        match self {
            OsEventInstant::Wndproc(a) => OsEventInstant::Wndproc(a - rhs),
        }
    }
}
impl AddAssign<Duration> for OsEventInstant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}
impl SubAssign<Duration> for OsEventInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}
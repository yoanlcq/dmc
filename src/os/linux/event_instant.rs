use super::libc as c;
use super::x11::xlib as x;

use std::ops::{Add, Sub, AddAssign, SubAssign};
use std::cmp::Ordering;
use std::time::Duration;
use time_utils;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OsEventInstant {
    X11EventTimeMillis(x::Time),
    // We don't take a c::timeval directly because it doesn't derive the traits we want.
    LinuxInputEventTimeval { tv_sec: c::time_t, tv_usec: c::suseconds_t },
    UdevUsecs(u64),
    // By the way, these last two variants really are not the same. The number of seconds
    // reported by a input_event may be crazy (unless udevadm and evtest tricked me
    // by not having the same timestamp format).
}

impl PartialOrd for OsEventInstant {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (OsEventInstant::X11EventTimeMillis(a), OsEventInstant::X11EventTimeMillis(b)) => a.partial_cmp(&b),
            (OsEventInstant::UdevUsecs(a), OsEventInstant::UdevUsecs(b)) => a.partial_cmp(&b),
            (OsEventInstant::LinuxInputEventTimeval { tv_sec: a_sec, tv_usec: a_usec },
             OsEventInstant::LinuxInputEventTimeval { tv_sec: b_sec, tv_usec: b_usec, }) => {
                if a_sec != b_sec {
                    a_sec.partial_cmp(&b_sec)
                } else {
                    a_usec.partial_cmp(&b_usec)
                }
            },
            _ => None,
        }
    }
}

impl OsEventInstant {
    pub fn duration_since(&self, earlier: Self) -> Option<Duration> {
        assert!(self >= &earlier); // Normally already checked by EventInstant::duration_since
        match (*self, earlier) {
            (OsEventInstant::X11EventTimeMillis(late), OsEventInstant::X11EventTimeMillis(early)) => {
                Some(Duration::from_millis(late - early))
            },
            (OsEventInstant::UdevUsecs(late), OsEventInstant::UdevUsecs(early)) => {
                Some(time_utils::duration_from_usecs(late - early))
            },
            (OsEventInstant::LinuxInputEventTimeval { tv_sec: late_sec, tv_usec: late_usec },
             OsEventInstant::LinuxInputEventTimeval { tv_sec: early_sec, tv_usec: early_usec, }) => {
                let early = c::timeval { tv_sec: early_sec, tv_usec: early_usec };
                let late = c::timeval { tv_sec: late_sec, tv_usec: late_usec };
                let early = time_utils::duration_from_timeval(early);
                let late = time_utils::duration_from_timeval(late);
                Some(late - early)
            },
            _ => None,
        }
    }
}
impl Add<Duration> for OsEventInstant {
    type Output = Self;
    fn add(self, rhs: Duration) -> Self {
        match self {
            OsEventInstant::X11EventTimeMillis(ms) => OsEventInstant::X11EventTimeMillis(ms.saturating_add(time_utils::duration_to_millis(&rhs))),
            OsEventInstant::UdevUsecs(us) => OsEventInstant::UdevUsecs(us.saturating_add(time_utils::duration_to_usecs(&rhs))),
            OsEventInstant::LinuxInputEventTimeval { tv_sec, tv_usec } => {
                OsEventInstant::LinuxInputEventTimeval {
                    tv_sec: tv_sec.saturating_add(rhs.as_secs() as _),
                    tv_usec: tv_usec.saturating_add((rhs.subsec_nanos() / 1000) as _),
                }
            },
        }
    }
}
impl Sub<Duration> for OsEventInstant {
    type Output = Self;
    fn sub(self, rhs: Duration) -> Self {
        match self {
            OsEventInstant::X11EventTimeMillis(ms) => OsEventInstant::X11EventTimeMillis(ms.saturating_sub(time_utils::duration_to_millis(&rhs))),
            OsEventInstant::UdevUsecs(us) => OsEventInstant::UdevUsecs(us.saturating_sub(time_utils::duration_to_usecs(&rhs))),
            OsEventInstant::LinuxInputEventTimeval { tv_sec, tv_usec } => {
                OsEventInstant::LinuxInputEventTimeval {
                    tv_sec: tv_sec.saturating_sub(rhs.as_secs() as _),
                    tv_usec: tv_usec.saturating_sub((rhs.subsec_nanos() / 1000) as _),
                }
            },
        }
    }
}
impl AddAssign<Duration> for OsEventInstant {
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.add(rhs);
    }
}
impl SubAssign<Duration> for OsEventInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.sub(rhs);
    }
}



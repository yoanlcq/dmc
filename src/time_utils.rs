use std::time::Duration;

#[cfg(x11)]
pub use self::c::*;
#[cfg(x11)]
pub mod c {
    use super::*;
    extern crate libc as c;

    pub fn duration_from_timeval(timeval: c::timeval) -> Duration {
        let c::timeval { tv_sec, tv_usec } = timeval;
        let secs = ::std::cmp::max(tv_sec, 0) as u64;
        let tv_usec = ::std::cmp::max(tv_usec, 0);
        let nanos = tv_usec.saturating_mul(1000) as u32;
        Duration::new(secs, nanos)
    }
}

pub fn duration_from_usecs(mut usecs: u64) -> Duration {
    let secs = usecs / 1_000_000;
    usecs -= secs * 1_000_000;
    let nanos = usecs * 1_000;
    Duration::new(secs, nanos as u32)
}
pub fn duration_to_usecs(d: &Duration) -> u64 {
    d.as_secs().saturating_mul(1_000_000).saturating_add((d.subsec_nanos() / 1_000) as u64)
}
pub fn duration_to_millis(d: &Duration) -> u64 {
    d.as_secs().saturating_mul(1_000).saturating_add((d.subsec_nanos() / 1_000_000) as u64)
}

